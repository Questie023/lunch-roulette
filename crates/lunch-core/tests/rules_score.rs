use std::collections::BTreeMap;

use lunch_core::model::{Rating, Reason, Restaurant, RestaurantId, UserPrefs};
use lunch_core::rules::score::{Candidate, Scored, score};

fn place(id: RestaurantId) -> Restaurant {
    Restaurant {
        id,
        name: format!("R{id}"),
        address: None,
        lat: None,
        lon: None,
        walk_min: Some(5),
        cuisine: "thai".into(),
        price_level: 2,
        opening_hours: None,
        website: None,
        menu_url: None,
        source: "manual".into(),
        active: true,
        diet_support: BTreeMap::new(),
    }
}

/// Eight thai places at price 2, ids 1..=8.
fn eight() -> Vec<Restaurant> {
    (1..=8).map(place).collect()
}

fn run(places: &[Restaurant], group: &[&UserPrefs]) -> Vec<Scored> {
    let candidates: Vec<Candidate> = places.iter().map(|r| Candidate { restaurant: r }).collect();
    score(&candidates, group)
}

fn weights(places: &[Restaurant], group: &[&UserPrefs]) -> Vec<f64> {
    run(places, group).into_iter().map(|s| s.weight).collect()
}

fn share(w: &[f64], i: usize) -> f64 {
    w[i] / w.iter().sum::<f64>()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn rates(entries: &[(RestaurantId, Rating)]) -> UserPrefs {
    UserPrefs {
        ratings: entries.iter().copied().collect(),
        ..UserPrefs::default()
    }
}

/// 👍 on place 1, 👎 on the other seven.
fn manipulator() -> UserPrefs {
    let mut entries = vec![(1, Rating::Good)];
    entries.extend((2..=8).map(|id| (id, Rating::Bad)));
    rates(&entries)
}

#[test]
fn one_result_per_candidate_in_order() {
    assert_eq!(run(&eight(), &[&UserPrefs::default()]).len(), 8);
}

#[test]
fn nobody_joined_is_neutral() {
    assert!(weights(&eight(), &[]).iter().all(|w| close(*w, 1.0)));
}

#[test]
fn weights_stay_within_bounds() {
    let fan = UserPrefs {
        favorites: [1].into(),
        cuisine_weights: [("thai".to_string(), 2)].into(),
        price_pref: Some(3),
        ..rates(&[(1, Rating::Good)])
    };
    let m = manipulator();
    for w in weights(&eight(), &[&fan, &m]) {
        assert!((0.5..=2.0).contains(&w), "{w}");
    }
}

#[test]
fn hating_everything_is_neutral() {
    let hater = rates(&(1..=8).map(|id| (id, Rating::Bad)).collect::<Vec<_>>());
    assert!(weights(&eight(), &[&hater]).iter().all(|w| close(*w, 1.0)));
}

#[test]
fn explains_how_many_like_the_cuisine() {
    let fan = UserPrefs {
        cuisine_weights: [("thai".to_string(), 1)].into(),
        ..UserPrefs::default()
    };
    let scored = run(&eight(), &[&fan, &fan, &UserPrefs::default()]);
    assert!(scored[0].reasons.contains(&Reason::CuisineLiked {
        cuisine: "thai".into(),
        count: 2
    }));
}

#[test]
fn ratings_move_the_weight() {
    let anna = rates(&[(1, Rating::Good), (2, Rating::Bad)]);
    let w = weights(&eight(), &[&anna]);
    assert!(w[0] > w[2] && w[2] > w[1], "{w:?}");
}

#[test]
fn favourite_cuisine_and_price_directions() {
    let mut places = eight();
    places[1].cuisine = "swedish".into();
    places[2].price_level = 3;
    let anna = UserPrefs {
        favorites: [1].into(),
        cuisine_weights: [("swedish".to_string(), -2)].into(),
        price_pref: Some(2),
        ..UserPrefs::default()
    };
    let w = weights(&places, &[&anna]);
    assert!(w[0] > w[3], "favourite {w:?}");
    assert!(w[1] < w[3], "disliked cuisine {w:?}");
    assert!(w[2] < w[3], "price mismatch {w:?}");
}

#[test]
fn one_person_of_four_gets_a_quarter_say() {
    let m = manipulator();
    let n = UserPrefs::default();
    let w = weights(&eight(), &[&m, &n, &n, &n]);
    assert!(close(w[0], 2f64.powf(0.25)), "{w:?}");
    assert!(close(w[1], 2f64.powf(-0.0625)), "{w:?}");
}

#[test]
fn unanimous_group_hits_the_cap() {
    let m = manipulator();
    let w = weights(&eight(), &[&m, &m, &m, &m]);
    assert!(close(w[0], 2.0), "{w:?}");
    assert!(close(w[1], 2f64.powf(-0.25)), "{w:?}");
}

#[test]
fn fairness_guarantees() {
    let n = UserPrefs::default();
    let m = manipulator();
    let fan = rates(&[(1, Rating::Good)]);
    let places = eight();
    assert!(share(&weights(&places, &[&m, &n, &n, &n]), 0) <= 0.155);
    assert!(share(&weights(&places, &[&fan, &n, &n, &n]), 0) <= 0.150);
    let unanimous = share(&weights(&places, &[&fan, &fan, &fan, &fan]), 0);
    assert!(unanimous > 0.20 && unanimous <= 0.225, "{unanimous}");
    assert!(share(&weights(&places, &[&m, &m, &m, &m]), 0) <= 0.255);
}

#[test]
fn liked_before_and_favourite_reasons() {
    let a = UserPrefs {
        favorites: [1].into(),
        ..rates(&[(1, Rating::Good)])
    };
    let b = rates(&[(1, Rating::Good)]);
    let c = rates(&[(1, Rating::Bad)]);
    let scored = run(&eight(), &[&a, &b, &c]);
    assert!(
        scored[0].reasons.contains(&Reason::LikedBefore { liked: 2, of: 3 }),
        "{:?}",
        scored[0].reasons
    );
    assert!(scored[0].reasons.contains(&Reason::Favorite { count: 1 }));
    assert!(
        !scored[1]
            .reasons
            .iter()
            .any(|r| matches!(r, Reason::LikedBefore { .. }))
    );
}
