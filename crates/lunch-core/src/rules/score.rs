//! Group opinion → a bounded weight multiplier per candidate.
//!
//! How a weight is computed:
//! ```text
//! raw_i(r) = clamp(rating_i(r) + 0.5·favorite_i(r) + cuisine_i(r)/2 − 0.25·|price(r) − pref_i|, −1, 1)
//!            rating: Good +1, Ok 0, Bad −1, none 0; cuisine: weight −2..2, none 0;
//!            price term 0 when the person has no price preference
//! o_i(r)   = clamp(raw_i(r) − mean over all candidates of raw_i, −1, 1)    (centring)
//! g(r)     = mean over the group of o_i(r)                                  (0 for an empty group)
//! weight   = 2^g(r)                                                          ∈ [0.5, 2]
//! ```
//! Reasons: CuisineLiked { count of members with cuisine weight > 0 } when count ≥ 1;
//! Favorite { count } when ≥ 1; LikedBefore { liked: members who rated 👍, of: members who rated it }
//! when liked ≥ 1.

use crate::model::{Rating, Reason, Restaurant, UserPrefs};

pub struct Candidate<'a> {
    pub restaurant: &'a Restaurant,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scored {
    /// Multiplier in [0.5, 2]; the pipeline multiplies it by the variety factor.
    pub weight: f64,
    pub reasons: Vec<Reason>,
}

/// One `Scored` per candidate, in the same order. `group` holds only today's participants.
pub fn score(candidates: &[Candidate], group: &[&UserPrefs]) -> Vec<Scored> {
    let opinions: Vec<Vec<f64>> = group
        .iter()
        .map(|person| centred_opinions(person, candidates))
        .collect();
    candidates
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let g = if opinions.is_empty() {
                0.0
            } else {
                opinions.iter().map(|o| o[i]).sum::<f64>() / opinions.len() as f64
            };
            Scored {
                weight: 2f64.powf(g),
                reasons: reasons(c.restaurant, group),
            }
        })
        .collect()
}

/// One person's opinion of every candidate, centred on their own average and clamped to [−1, 1].
fn centred_opinions(person: &UserPrefs, candidates: &[Candidate]) -> Vec<f64> {
    let raw: Vec<f64> = candidates.iter().map(|c| raw_opinion(person, c.restaurant)).collect();
    let mean = raw.iter().sum::<f64>() / raw.len().max(1) as f64;
    raw.iter().map(|x| (x - mean).clamp(-1.0, 1.0)).collect()
}

fn raw_opinion(person: &UserPrefs, r: &Restaurant) -> f64 {
    let rating = match person.ratings.get(&r.id) {
        Some(Rating::Good) => 1.0,
        Some(Rating::Bad) => -1.0,
        Some(Rating::Ok) | None => 0.0,
    };
    let favorite = if person.favorites.contains(&r.id) { 0.5 } else { 0.0 };
    let cuisine = f64::from(person.cuisine_weights.get(&r.cuisine).copied().unwrap_or(0)) / 2.0;
    let price = person.price_pref.map_or(0.0, |pref| {
        0.25 * f64::from((i16::from(r.price_level) - i16::from(pref)).abs())
    });
    (rating + favorite + cuisine - price).clamp(-1.0, 1.0)
}

fn reasons(r: &Restaurant, group: &[&UserPrefs]) -> Vec<Reason> {
    let count = |f: &dyn Fn(&UserPrefs) -> bool| group.iter().filter(|p| f(p)).count() as u32;
    let mut out = Vec::new();
    let likers = count(&|p| p.cuisine_weights.get(&r.cuisine).is_some_and(|w| *w > 0));
    if likers > 0 {
        out.push(Reason::CuisineLiked {
            cuisine: r.cuisine.clone(),
            count: likers,
        });
    }
    let fans = count(&|p| p.favorites.contains(&r.id));
    if fans > 0 {
        out.push(Reason::Favorite { count: fans });
    }
    let liked = count(&|p| p.ratings.get(&r.id) == Some(&Rating::Good));
    if liked > 0 {
        out.push(Reason::LikedBefore {
            liked,
            of: count(&|p| p.ratings.contains_key(&r.id)),
        });
    }
    out
}
