//! The pick pipeline: filter → score → variety → weighted roulette, plus the explanation.

use std::collections::{BTreeMap, BTreeSet};

use jiff::civil::{Date, Time, Weekday};
use rand::Rng;

use crate::geo::walk_minutes;
use crate::model::{
    DietTag, Explanation, FilterStage, LunchOffer, PickOptions, PickOutcome, Reason, Restaurant, RestaurantId,
    Settings, User, UserPrefs, Warning,
};
use crate::rules::constraints::{Verdict, check_constraints};
use crate::rules::opening_hours::{Openness, parse_opening_hours};
use crate::rules::roulette;
use crate::rules::score::{Candidate, Scored, score};
use crate::rules::variety::variety_multiplier;

/// A restaurant not chosen for this many days counts as "not recently visited".
const FRESH_AFTER_DAYS: u32 = 14;

#[derive(Clone, Debug, PartialEq)]
pub struct PickContext {
    pub settings: Settings,
    pub today: Date,
    pub participants: Vec<(User, UserPrefs)>,
    pub restaurants: Vec<Restaurant>,
    pub offers_today: Vec<LunchOffer>,
    /// Most recent accepted pick date per restaurant, across all channels.
    pub last_accepted: BTreeMap<RestaurantId, Date>,
    /// Restaurants already rolled in this session; re-rolls skip them.
    pub already_shown: BTreeSet<RestaurantId>,
}

struct Staged<'a> {
    restaurant: &'a Restaurant,
    open: Openness,
    walk: Option<u32>,
    diet: Verdict,
}

pub fn run_pick<R: Rng + ?Sized>(ctx: &PickContext, opts: PickOptions, rng: &mut R) -> PickOutcome {
    let empty = |stage| PickOutcome::Empty { stage };
    let members: Vec<&UserPrefs> = ctx.participants.iter().map(|(_, prefs)| prefs).collect();
    let required: BTreeSet<DietTag> = members.iter().flat_map(|p| p.constraints.iter().copied()).collect();
    let default_walk = ctx.settings.default_max_walk_min;
    let max_walk = members
        .iter()
        .map(|p| p.max_walk_min.unwrap_or(default_walk))
        .min()
        .unwrap_or(default_walk);
    let window = (ctx.settings.lunch_start, ctx.settings.lunch_end);
    let weekday = ctx.today.weekday();

    let mut pool: Vec<&Restaurant> = ctx.restaurants.iter().filter(|r| r.active).collect();
    if pool.is_empty() {
        return empty(FilterStage::NoRestaurants);
    }
    pool.retain(|r| !ctx.already_shown.contains(&r.id));
    if pool.is_empty() {
        return empty(FilterStage::AllShown);
    }

    let mut staged: Vec<Staged> = pool
        .into_iter()
        .map(|r| Staged {
            restaurant: r,
            open: openness(r, weekday, window),
            walk: walk_minutes(r, &ctx.settings),
            diet: Verdict::Safe,
        })
        .collect();
    staged.retain(|s| s.open != Openness::Closed);
    if staged.is_empty() {
        return empty(FilterStage::Closed);
    }
    if !opts.ignore_distance {
        staged.retain(|s| s.walk.is_none_or(|w| w <= max_walk));
        if staged.is_empty() {
            return empty(FilterStage::TooFar);
        }
    }
    for s in &mut staged {
        let offers: Vec<&LunchOffer> = ctx
            .offers_today
            .iter()
            .filter(|o| o.restaurant_id == s.restaurant.id)
            .collect();
        s.diet = check_constraints(&required, &s.restaurant.diet_support, &offers);
    }
    staged.retain(|s| !matches!(s.diet, Verdict::Conflict(_)));
    if staged.is_empty() {
        return empty(FilterStage::DietConflict);
    }

    let candidates: Vec<Candidate> = staged
        .iter()
        .map(|s| Candidate {
            restaurant: s.restaurant,
        })
        .collect();
    let mut opinions = score(&candidates, &members);
    // Guard against a `score` that returns the wrong number of results.
    if opinions.len() != staged.len() {
        opinions = staged
            .iter()
            .map(|_| Scored {
                weight: 1.0,
                reasons: Vec::new(),
            })
            .collect();
    }
    let mut scored = Vec::with_capacity(staged.len());
    for (s, opinion) in staged.iter().zip(opinions) {
        let days = ctx
            .last_accepted
            .get(&s.restaurant.id)
            .map(|d| days_between(*d, ctx.today));
        scored.push((
            sanitize_weight(opinion.weight) * variety_multiplier(days),
            opinion.reasons,
            days,
        ));
    }
    let roll: Vec<f64> = scored.iter().map(|(weight, _, _)| *weight).collect();
    // Weights are always > 0 (see `sanitize_weight`); the fallback only guards against a buggy roulette.
    let index = roulette::pick(&roll, rng).filter(|&i| i < staged.len()).unwrap_or(0);
    let chosen = &staged[index];
    let (_, mut reasons, days) = scored.swap_remove(index);

    if days.is_none_or(|d| d >= FRESH_AFTER_DAYS) {
        reasons.push(Reason::NotRecentlyVisited);
    }
    let mut warnings = Vec::new();
    if chosen.open == Openness::Unknown {
        warnings.push(Warning::HoursUnknown);
    }
    if chosen.walk.is_none() {
        warnings.push(Warning::DistanceUnknown);
    }
    if matches!(chosen.diet, Verdict::Unknown(_)) {
        warnings.push(Warning::DietInfoUnknown);
    }
    if opts.ignore_distance {
        warnings.push(Warning::DistanceIgnored);
    }
    PickOutcome::Picked {
        restaurant_id: chosen.restaurant.id,
        explanation: Explanation {
            walk_min: chosen.walk,
            reasons,
            warnings,
        },
    }
}

/// Keeps a score weight inside the promised [0.5, 2] so one bad value cannot starve the roulette.
fn sanitize_weight(weight: f64) -> f64 {
    if weight.is_finite() {
        weight.clamp(0.5, 2.0)
    } else {
        1.0
    }
}

fn openness(r: &Restaurant, weekday: Weekday, window: (Time, Time)) -> Openness {
    match r.opening_hours.as_deref().map(parse_opening_hours) {
        Some(Ok(hours)) => hours.status(weekday, window),
        Some(Err(_)) | None => Openness::Unknown,
    }
}

fn days_between(earlier: Date, today: Date) -> u32 {
    u32::try_from((today - earlier).get_days()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::sanitize_weight;

    #[test]
    fn score_weights_are_forced_into_range() {
        assert_eq!(sanitize_weight(1.3), 1.3);
        assert_eq!(sanitize_weight(0.0), 0.5);
        assert_eq!(sanitize_weight(-4.0), 0.5);
        assert_eq!(sanitize_weight(9.0), 2.0);
        assert_eq!(sanitize_weight(f64::NAN), 1.0);
        assert_eq!(sanitize_weight(f64::INFINITY), 1.0);
    }
}
