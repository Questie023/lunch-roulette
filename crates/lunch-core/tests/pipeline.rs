use std::collections::{BTreeMap, BTreeSet};

use jiff::civil::date;
use lunch_core::model::*;
use lunch_core::pipeline::{PickContext, run_pick};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn restaurant(id: i64) -> Restaurant {
    Restaurant {
        id,
        name: format!("R{id}"),
        address: None,
        lat: None,
        lon: None,
        walk_min: Some(5),
        cuisine: "thai".into(),
        price_level: 2,
        opening_hours: Some("Mo-Fr 11:00-14:00".into()),
        website: None,
        menu_url: None,
        source: "manual".into(),
        active: true,
        diet_support: BTreeMap::new(),
    }
}

fn person(id: &str, prefs: UserPrefs) -> (User, UserPrefs) {
    (
        User {
            id: id.into(),
            display_name: format!("Name {id}"),
        },
        prefs,
    )
}

/// Tuesday 2026-10-06, one participant with default prefs unless overridden.
fn ctx(restaurants: Vec<Restaurant>, participants: Vec<(User, UserPrefs)>) -> PickContext {
    PickContext {
        settings: Settings::default(),
        today: date(2026, 10, 6),
        participants,
        restaurants,
        offers_today: vec![],
        last_accepted: BTreeMap::new(),
        already_shown: BTreeSet::new(),
    }
}

fn solo() -> Vec<(User, UserPrefs)> {
    vec![person("U1", UserPrefs::default())]
}

fn roll(c: &PickContext, opts: PickOptions) -> PickOutcome {
    run_pick(c, opts, &mut StdRng::seed_from_u64(42))
}

fn stage(o: PickOutcome) -> FilterStage {
    match o {
        PickOutcome::Empty { stage } => stage,
        other => panic!("expected an empty outcome, got {other:?}"),
    }
}

fn picked(o: PickOutcome) -> (RestaurantId, Explanation) {
    match o {
        PickOutcome::Picked {
            restaurant_id,
            explanation,
        } => (restaurant_id, explanation),
        other => panic!("expected a pick, got {other:?}"),
    }
}

#[test]
fn picks_the_only_candidate_and_explains() {
    let (id, e) = picked(roll(&ctx(vec![restaurant(1)], solo()), PickOptions::default()));
    assert_eq!(id, 1);
    assert_eq!(e.walk_min, Some(5));
    assert!(e.reasons.contains(&Reason::NotRecentlyVisited));
    assert!(e.warnings.is_empty());
}

#[test]
fn no_active_restaurants() {
    assert_eq!(
        stage(roll(&ctx(vec![], solo()), PickOptions::default())),
        FilterStage::NoRestaurants
    );
    let inactive = Restaurant {
        active: false,
        ..restaurant(1)
    };
    assert_eq!(
        stage(roll(&ctx(vec![inactive], solo()), PickOptions::default())),
        FilterStage::NoRestaurants
    );
}

#[test]
fn already_shown_are_excluded() {
    let mut c = ctx(vec![restaurant(1)], solo());
    c.already_shown.insert(1);
    assert_eq!(stage(roll(&c, PickOptions::default())), FilterStage::AllShown);
}

#[test]
fn a_dislike_does_not_filter() {
    let hater = UserPrefs {
        ratings: [(1, Rating::Bad)].into(),
        ..UserPrefs::default()
    };
    let c = ctx(
        vec![restaurant(1)],
        vec![person("U1", UserPrefs::default()), person("U2", hater)],
    );
    let (id, _) = picked(roll(&c, PickOptions::default()));
    assert_eq!(id, 1);
}

#[test]
fn closed_today_is_removed() {
    let weekend_only = Restaurant {
        opening_hours: Some("Sa 11:00-14:00".into()),
        ..restaurant(1)
    };
    assert_eq!(
        stage(roll(&ctx(vec![weekend_only], solo()), PickOptions::default())),
        FilterStage::Closed
    );
}

#[test]
fn unknown_hours_are_kept_with_a_warning() {
    let unknown = Restaurant {
        opening_hours: None,
        ..restaurant(1)
    };
    let (_, e) = picked(roll(&ctx(vec![unknown], solo()), PickOptions::default()));
    assert!(e.warnings.contains(&Warning::HoursUnknown));
}

#[test]
fn too_far_unless_distance_is_ignored() {
    let far = Restaurant {
        walk_min: Some(25),
        ..restaurant(1)
    };
    let c = ctx(vec![far], solo());
    assert_eq!(stage(roll(&c, PickOptions::default())), FilterStage::TooFar);
    let (_, e) = picked(roll(&c, PickOptions { ignore_distance: true }));
    assert!(e.warnings.contains(&Warning::DistanceIgnored));
}

#[test]
fn strictest_walking_limit_wins() {
    let walker = UserPrefs {
        max_walk_min: Some(15),
        ..UserPrefs::default()
    };
    let lazy = UserPrefs {
        max_walk_min: Some(5),
        ..UserPrefs::default()
    };
    let eight = Restaurant {
        walk_min: Some(8),
        ..restaurant(1)
    };
    let c = ctx(vec![eight], vec![person("U1", walker), person("U2", lazy)]);
    assert_eq!(stage(roll(&c, PickOptions::default())), FilterStage::TooFar);
}

#[test]
fn known_diet_conflict_is_removed() {
    let vegan = UserPrefs {
        constraints: [DietTag::Vegan].into(),
        ..UserPrefs::default()
    };
    let steakhouse = Restaurant {
        diet_support: [(DietTag::Vegan, Support::No)].into(),
        ..restaurant(1)
    };
    let c = ctx(vec![steakhouse], vec![person("U1", vegan)]);
    assert_eq!(stage(roll(&c, PickOptions::default())), FilterStage::DietConflict);
}

#[test]
fn unknown_diet_is_kept_with_an_anonymous_warning() {
    let allergic = UserPrefs {
        constraints: [DietTag::PeanutFree].into(),
        ..UserPrefs::default()
    };
    let (_, e) = picked(roll(
        &ctx(vec![restaurant(1)], vec![person("U1", allergic)]),
        PickOptions::default(),
    ));
    assert!(e.warnings.contains(&Warning::DietInfoUnknown));
    let json = serde_json::to_string(&e).unwrap();
    assert!(
        !json.contains("U1") && !json.contains("Name U1") && !json.contains("peanut"),
        "{json}"
    );
}

#[test]
fn recently_accepted_is_not_called_fresh() {
    let mut c = ctx(vec![restaurant(1)], solo());
    c.last_accepted.insert(1, date(2026, 10, 5));
    let (_, e) = picked(roll(&c, PickOptions::default()));
    assert!(!e.reasons.contains(&Reason::NotRecentlyVisited));
}

#[test]
fn same_seed_same_result() {
    let c = ctx((1..=5).map(restaurant).collect(), solo());
    assert_eq!(roll(&c, PickOptions::default()), roll(&c, PickOptions::default()));
}
