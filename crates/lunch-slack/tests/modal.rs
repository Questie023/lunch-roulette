use lunch_slack::modal::{ADD_RESTAURANT, add_restaurant_modal, errors_response, parse_add_restaurant};
use lunch_slack::payload::View;
use serde_json::{Value, json};

fn view(values: Value) -> View {
    serde_json::from_value(json!({
        "callback_id": ADD_RESTAURANT,
        "private_metadata": "C1",
        "state": {"values": values},
    }))
    .unwrap()
}

fn text(v: &str) -> Value {
    json!({"v": {"type": "plain_text_input", "value": v}})
}

fn price(v: &str) -> Value {
    json!({"v": {"type": "static_select", "selected_option": {"value": v}}})
}

#[test]
fn modal_carries_the_channel_and_has_six_inputs() {
    let m = add_restaurant_modal("C42");
    assert_eq!(m["callback_id"], ADD_RESTAURANT);
    assert_eq!(m["private_metadata"], "C42");
    assert_eq!(m["blocks"].as_array().unwrap().len(), 6);
}

#[test]
fn valid_submission_becomes_input() {
    let input = parse_add_restaurant(&view(json!({
        "name": text("  Bar & Grill "),
        "cuisine": text("Swedish"),
        "walk": text("6"),
        "price": price("3"),
        "hours": text("Mo-Fr 11:00-14:00"),
        "menu": text("https://example.com/lunch"),
    })))
    .unwrap();
    assert_eq!(input.name, "Bar & Grill");
    assert_eq!(input.cuisine, "swedish");
    assert_eq!(input.walk_min, Some(6));
    assert_eq!(input.price_level, 3);
    assert_eq!(input.opening_hours.as_deref(), Some("Mo-Fr 11:00-14:00"));
}

#[test]
fn optional_fields_may_be_empty() {
    let input = parse_add_restaurant(&view(json!({
        "name": text("Curry House"), "cuisine": text("persian"), "walk": text("13"), "price": price("2"),
        "hours": {"v": {"type": "plain_text_input", "value": null}}, "menu": text(""),
    })))
    .unwrap();
    assert_eq!((input.opening_hours, input.menu_url), (None, None));
}

#[test]
fn missing_and_invalid_fields_are_reported_per_block() {
    let errors = parse_add_restaurant(&view(json!({
        "name": text(""), "cuisine": text("thai"), "walk": text("ten"), "price": price("2"),
        "hours": text("whenever"), "menu": text("ftp://x"),
    })))
    .unwrap_err();
    let keys: Vec<&str> = errors.keys().map(String::as_str).collect();
    assert_eq!(keys, ["hours", "menu", "name", "walk"]);
}

#[test]
fn errors_response_shape() {
    let r = errors_response(&[("name".to_string(), "Please enter a name.".to_string())].into());
    assert_eq!(r["response_action"], "errors");
    assert_eq!(r["errors"]["name"], "Please enter a name.");
}

#[test]
fn overlong_name_and_bad_link_are_reported() {
    let errors = parse_add_restaurant(&view(json!({
        "name": text(&"N".repeat(61)), "cuisine": text("thai"), "walk": text("5"), "price": price("2"),
        "menu": text("https://a b"),
    })))
    .unwrap_err();
    let keys: Vec<&str> = errors.keys().map(String::as_str).collect();
    assert_eq!(keys, ["menu", "name"]);
}

#[test]
fn inputs_carry_max_length() {
    let modal = add_restaurant_modal("C1");
    let blocks = modal["blocks"].as_array().unwrap();
    let max = |id: &str| blocks.iter().find(|b| b["block_id"] == id).unwrap()["element"]["max_length"].as_u64();
    assert_eq!(
        (max("name"), max("cuisine"), max("walk"), max("hours"), max("menu")),
        (Some(60), Some(30), Some(2), Some(100), Some(300))
    );
}
