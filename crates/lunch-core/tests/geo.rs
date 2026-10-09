use std::collections::BTreeMap;

use lunch_core::geo::{haversine_m, walk_minutes};
use lunch_core::model::{Restaurant, Settings};

fn office() -> Settings {
    Settings {
        office_lat: 60.0,
        office_lon: 10.0,
        ..Settings::default()
    }
}

fn at(lat: Option<f64>, lon: Option<f64>, walk_min: Option<u32>) -> Restaurant {
    Restaurant {
        id: 1,
        name: "X".into(),
        address: None,
        lat,
        lon,
        walk_min,
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

#[test]
fn haversine_matches_a_known_short_walk() {
    // 0.0013° due north of the office: ~145 m.
    let d = haversine_m(60.0, 10.0, 60.0013, 10.0);
    assert!((140.0..155.0).contains(&d), "{d}");
}

#[test]
fn walk_minutes_from_coordinates_include_detour() {
    assert_eq!(walk_minutes(&at(Some(60.0013), Some(10.0), None), &office()), Some(3));
    assert_eq!(walk_minutes(&at(Some(60.0), Some(10.0036), None), &office()), Some(4));
}

#[test]
fn explicit_walk_time_wins() {
    assert_eq!(
        walk_minutes(&at(Some(60.0013), Some(10.0), Some(7)), &office()),
        Some(7)
    );
}

#[test]
fn unknown_without_coordinates() {
    assert_eq!(walk_minutes(&at(None, None, None), &office()), None);
}
