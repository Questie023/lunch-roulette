use lunch_core::Error;
use lunch_core::model::{
    DietTag, MAX_CUISINE_CHARS, MAX_HOURS_CHARS, MAX_NAME_CHARS, MAX_URL_CHARS, Rating, RestaurantInput, SessionStatus,
    Settings, is_valid_url,
};

#[test]
fn diet_tags_round_trip_through_strings() {
    for tag in DietTag::ALL {
        assert_eq!(tag.as_str().parse::<DietTag>().unwrap(), *tag);
    }
    assert_eq!(DietTag::ALL.len(), 18);
}

#[test]
fn unknown_diet_tag_is_invalid() {
    assert!(matches!("paleo".parse::<DietTag>(), Err(Error::Invalid(_))));
}

#[test]
fn string_enums_round_trip() {
    assert_eq!("decided".parse::<SessionStatus>().unwrap(), SessionStatus::Decided);
    assert_eq!(Rating::Bad.as_str(), "bad");
}

#[test]
fn settings_from_pairs_overrides_defaults() {
    let s = Settings::from_pairs([
        ("lunch_start", "11:00"),
        ("default_max_walk_min", "15"),
        ("unknown_key", "x"),
    ])
    .unwrap();
    assert_eq!(s.lunch_start, jiff::civil::time(11, 0, 0, 0));
    assert_eq!(s.default_max_walk_min, 15);
    assert_eq!(s.timezone, "Europe/Stockholm");
    assert!(s.time_zone().is_ok());
}

#[test]
fn settings_reject_bad_values() {
    assert!(Settings::from_pairs([("office_lat", "north")]).is_err());
}

fn input() -> RestaurantInput {
    RestaurantInput {
        name: "Curry House".into(),
        address: None,
        walk_min: Some(6),
        cuisine: "indian".into(),
        price_level: 2,
        opening_hours: Some("Mo-Fr 11:00-14:00".into()),
        menu_url: None,
    }
}

#[test]
fn restaurant_input_validation() {
    assert!(input().validate().is_ok());
    assert!(
        RestaurantInput {
            name: "  ".into(),
            ..input()
        }
        .validate()
        .is_err()
    );
    assert!(
        RestaurantInput {
            cuisine: "".into(),
            ..input()
        }
        .validate()
        .is_err()
    );
    assert!(
        RestaurantInput {
            price_level: 4,
            ..input()
        }
        .validate()
        .is_err()
    );
    assert!(
        RestaurantInput {
            walk_min: Some(0),
            ..input()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn restaurant_input_length_limits_count_characters() {
    let at_limit = "й".repeat(MAX_NAME_CHARS); // 2 bytes per char: limits are in characters
    assert!(
        RestaurantInput {
            name: at_limit.clone(),
            ..input()
        }
        .validate()
        .is_ok()
    );
    assert!(
        RestaurantInput {
            name: format!("{at_limit}x"),
            ..input()
        }
        .validate()
        .is_err()
    );
    assert!(
        RestaurantInput {
            cuisine: "x".repeat(MAX_CUISINE_CHARS + 1),
            ..input()
        }
        .validate()
        .is_err()
    );
    let long_hours = Some("x".repeat(MAX_HOURS_CHARS + 1));
    assert!(
        RestaurantInput {
            opening_hours: long_hours,
            ..input()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn restaurant_input_rejects_bad_menu_links() {
    let with = |url: &str| RestaurantInput {
        menu_url: Some(url.into()),
        ..input()
    };
    assert!(with("https://curry-house.example/menu").validate().is_ok());
    assert!(with("https://a b").validate().is_err());
}

#[test]
fn url_validation() {
    let good = [
        "https://curry-house.example/menu",
        "http://example.com",
        "https://www.noodle-bar.example/#lunch",
        "https://green-kitchen.example/menus/today.php?week=41",
    ];
    for url in good {
        assert!(is_valid_url(url), "{url}");
    }
    let too_long = format!("https://x.se/{}", "a".repeat(MAX_URL_CHARS));
    let bad = [
        "",
        "https://",
        "https://a b",
        "javascript:alert(1)",
        "ftp://x.se",
        "https://localhost",
        "https://x.se/\n",
        &too_long,
    ];
    for url in bad {
        assert!(!is_valid_url(url), "{url:?}");
    }
}
