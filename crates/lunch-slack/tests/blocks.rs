use std::collections::BTreeMap;

use jiff::civil::date;
use lunch_core::model::*;
use lunch_slack::actions::{ActionValue, LunchAction};
use lunch_slack::blocks::{ephemeral, escape, session_message};
use serde_json::Value;

fn noodle_bar() -> Restaurant {
    Restaurant {
        id: 7,
        name: "Noodle Bar".into(),
        address: Some("1 Example Street".into()),
        lat: Some(60.0013),
        lon: Some(10.0),
        walk_min: None,
        cuisine: "thai".into(),
        price_level: 2,
        opening_hours: Some("Mo-Fr 11:00-14:00".into()),
        website: None,
        menu_url: Some("https://noodle-bar.example/lunch".into()),
        source: "canvas".into(),
        active: true,
        diet_support: BTreeMap::new(),
    }
}

fn view(pick: Option<ShownPick>) -> SessionView {
    SessionView {
        session: Session {
            id: 42,
            channel_id: "C1".into(),
            date: date(2026, 10, 6),
            message_ts: Some("1700000000.000100".into()),
            status: SessionStatus::Open,
            created_by: "U1".into(),
        },
        participants: vec![
            User {
                id: "U1".into(),
                display_name: "Anna".into(),
            },
            User {
                id: "U2".into(),
                display_name: "Bar & <Max>".into(),
            },
        ],
        latest_pick: pick,
    }
}

fn pick(accepted: bool) -> ShownPick {
    ShownPick {
        pick_id: 9,
        restaurant: noodle_bar(),
        kind: PickKind::Roll,
        accepted,
        explanation: Explanation {
            walk_min: Some(3),
            reasons: vec![
                Reason::CuisineLiked {
                    cuisine: "thai".into(),
                    count: 2,
                },
                Reason::NotRecentlyVisited,
            ],
            warnings: vec![Warning::DietInfoUnknown],
        },
    }
}

#[test]
fn action_ids_round_trip() {
    let all = [
        LunchAction::Join,
        LunchAction::Leave,
        LunchAction::Pick,
        LunchAction::IgnoreDistance,
        LunchAction::Manual,
        LunchAction::Accept,
        LunchAction::Feedback(Rating::Good),
        LunchAction::Feedback(Rating::Ok),
        LunchAction::Feedback(Rating::Bad),
        LunchAction::Menu,
    ];
    for a in all {
        assert_eq!(LunchAction::from_id(a.id()), Some(a));
    }
    assert_eq!(LunchAction::from_id("nope"), None);
}

#[test]
fn action_values_round_trip() {
    assert_eq!(ActionValue::decode("42"), Some(ActionValue::session(42)));
    assert_eq!(ActionValue::decode("42:7"), Some(ActionValue::with_target(42, 7)));
    assert_eq!(ActionValue::with_target(42, 7).encode(), "42:7");
    assert_eq!(ActionValue::decode("x"), None);
    assert_eq!(ActionValue::decode("42:x"), None);
}

#[test]
fn board_without_pick() {
    insta::assert_json_snapshot!(
        "board_without_pick",
        session_message(&view(None), &[noodle_bar()], None)
    );
}

#[test]
fn board_with_pick_and_warning() {
    insta::assert_json_snapshot!(
        "board_with_pick",
        session_message(&view(Some(pick(false))), &[noodle_bar()], None)
    );
}

#[test]
fn board_after_accept_offers_feedback() {
    insta::assert_json_snapshot!(
        "board_accepted",
        session_message(&view(Some(pick(true))), &[noodle_bar()], None)
    );
}

#[test]
fn board_too_far_offers_ignore_distance() {
    let msg = session_message(&view(None), &[noodle_bar()], Some(FilterStage::TooFar));
    insta::assert_json_snapshot!("board_too_far", msg);
}

#[test]
fn escapes_mrkdwn_in_names() {
    let json = session_message(&view(None), &[], None).to_string();
    assert!(json.contains("Bar &amp; &lt;Max&gt;"), "{json}");
    assert!(!json.contains("<Max>"));
    assert_eq!(escape("a&b<c>"), "a&amp;b&lt;c&gt;");
}

#[test]
fn participants_are_never_mentioned() {
    let json = session_message(&view(Some(pick(true))), &[noodle_bar()], None).to_string();
    assert!(!json.contains("<@U"));
}

#[test]
fn ephemeral_does_not_replace_the_board() {
    let e = ephemeral("hi");
    assert_eq!(e["response_type"], "ephemeral");
    assert_eq!(e["replace_original"], false);
}

#[test]
fn feedback_buttons_carry_the_pick() {
    let msg = session_message(&view(Some(pick(true))), &[noodle_bar()], None);
    let blocks = msg["blocks"].as_array().unwrap();
    let feedback = blocks.iter().find(|b| b["block_id"] == "lunch_feedback").unwrap();
    for button in feedback["elements"].as_array().unwrap() {
        assert_eq!(button["value"], "42:9");
    }
}

/// Every `text` string anywhere in the message, as character counts.
fn text_lengths(v: &Value, out: &mut Vec<usize>) {
    match v {
        Value::Object(map) => {
            if let Some(Value::String(t)) = map.get("text") {
                out.push(t.chars().count());
            }
            map.values().for_each(|x| text_lengths(x, out));
        }
        Value::Array(items) => items.iter().for_each(|x| text_lengths(x, out)),
        _ => {}
    }
}

#[test]
fn long_names_stay_within_slack_limits() {
    let huge = Restaurant {
        name: "&".repeat(5000),
        cuisine: "<".repeat(5000),
        ..noodle_bar()
    };
    let mut shown = pick(false);
    shown.restaurant = huge.clone();
    shown.explanation.reasons.push(Reason::CuisineLiked {
        cuisine: huge.cuisine.clone(),
        count: 2,
    });
    let mut v = view(Some(shown));
    v.participants = (0..300)
        .map(|i| User {
            id: format!("U{i}"),
            display_name: "&".repeat(80),
        })
        .collect();
    let mut lengths = Vec::new();
    text_lengths(&session_message(&v, &[huge], None)["blocks"], &mut lengths);
    assert!(lengths.iter().all(|&n| n <= 3000), "{lengths:?}");
}

#[test]
fn invalid_menu_url_hides_the_button() {
    let broken = Restaurant {
        menu_url: Some("https://a b".into()),
        ..noodle_bar()
    };
    let mut shown = pick(false);
    shown.restaurant = broken.clone();
    let json = session_message(&view(Some(shown)), &[broken], None).to_string();
    assert!(!json.contains("Today's menu"), "{json}");
}

#[test]
fn thumbs_down_is_a_soft_dislike() {
    let json = session_message(&view(Some(pick(true))), &[noodle_bar()], None).to_string();
    assert!(json.contains("👎 Not for me"), "{json}");
    assert!(!json.contains("Don't suggest again"));
}
