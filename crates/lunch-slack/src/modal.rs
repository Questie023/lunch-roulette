//! `/lunch add` modal and its submission parsing.

use std::collections::BTreeMap;

use lunch_core::model::{
    MAX_CUISINE_CHARS, MAX_HOURS_CHARS, MAX_NAME_CHARS, MAX_URL_CHARS, RestaurantInput, is_valid_url,
};
use lunch_core::rules::opening_hours::parse_opening_hours;
use serde_json::{Value, json};

use crate::payload::View;

pub const ADD_RESTAURANT: &str = "add_restaurant";

/// Field errors keyed by `block_id`, ready for `{"response_action": "errors"}`.
pub type FieldErrors = BTreeMap<String, String>;

fn text_input(block_id: &str, label: &str, placeholder: &str, optional: bool, max_length: usize) -> Value {
    json!({
        "type": "input",
        "block_id": block_id,
        "optional": optional,
        "label": {"type": "plain_text", "text": label},
        "element": {
            "type": "plain_text_input",
            "action_id": "v",
            "max_length": max_length,
            "placeholder": {"type": "plain_text", "text": placeholder},
        },
    })
}

fn price_option(level: u8) -> Value {
    let text = match level {
        1 => "💰 cheap",
        2 => "💰💰 normal",
        _ => "💰💰💰 pricey",
    };
    json!({"text": {"type": "plain_text", "text": text, "emoji": true}, "value": level.to_string()})
}

/// `channel_id` travels in `private_metadata` so the confirmation can be posted back there.
pub fn add_restaurant_modal(channel_id: &str) -> Value {
    json!({
        "type": "modal",
        "callback_id": ADD_RESTAURANT,
        "private_metadata": channel_id,
        "title": {"type": "plain_text", "text": "Add a restaurant"},
        "submit": {"type": "plain_text", "text": "Add"},
        "close": {"type": "plain_text", "text": "Cancel"},
        "blocks": [
            text_input("name", "Name", "Noodle Bar", false, MAX_NAME_CHARS),
            text_input("cuisine", "Cuisine", "thai", false, MAX_CUISINE_CHARS),
            text_input("walk", "Walking time from the office (minutes)", "5", false, 2),
            {
                "type": "input",
                "block_id": "price",
                "label": {"type": "plain_text", "text": "Price level"},
                "element": {
                    "type": "static_select",
                    "action_id": "v",
                    "initial_option": price_option(2),
                    "options": [price_option(1), price_option(2), price_option(3)],
                },
            },
            text_input("hours", "Lunch hours", "Mo-Fr 11:00-14:00", true, MAX_HOURS_CHARS),
            text_input("menu", "Menu link", "https://…", true, MAX_URL_CHARS),
        ],
    })
}

pub fn parse_add_restaurant(view: &View) -> Result<RestaurantInput, FieldErrors> {
    let field = |block: &str| view.state.values.get(block).and_then(|actions| actions.get("v"));
    let text = |block: &str| {
        field(block)
            .and_then(|v| v.value.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let mut errors = FieldErrors::new();

    let name = text("name");
    if name.is_none() {
        errors.insert("name".into(), "Please enter a name.".into());
    } else if name.as_deref().is_some_and(|n| n.chars().count() > MAX_NAME_CHARS) {
        errors.insert("name".into(), format!("At most {MAX_NAME_CHARS} characters."));
    }
    let cuisine = text("cuisine");
    if cuisine.is_none() {
        errors.insert("cuisine".into(), "Please enter a cuisine, e.g. thai.".into());
    } else if cuisine
        .as_deref()
        .is_some_and(|c| c.chars().count() > MAX_CUISINE_CHARS)
    {
        errors.insert("cuisine".into(), format!("At most {MAX_CUISINE_CHARS} characters."));
    }
    let walk_min = match text("walk").map(|w| w.parse::<u32>()) {
        Some(Ok(w)) if (1..=60).contains(&w) => Some(w),
        _ => {
            errors.insert("walk".into(), "Enter whole minutes between 1 and 60.".into());
            None
        }
    };
    let price_level = field("price")
        .and_then(|v| v.selected_option.as_ref())
        .and_then(|o| o.value.parse::<u8>().ok())
        .unwrap_or(2);
    let opening_hours = text("hours");
    if opening_hours
        .as_deref()
        .is_some_and(|h| h.chars().count() > MAX_HOURS_CHARS)
    {
        errors.insert("hours".into(), format!("At most {MAX_HOURS_CHARS} characters."));
    } else if let Some(Err(e)) = opening_hours.as_deref().map(parse_opening_hours) {
        errors.insert(
            "hours".into(),
            format!("Use the format Mo-Fr 11:00-14:00 ({}).", e.reason),
        );
    }
    let menu_url = text("menu");
    if menu_url.as_deref().is_some_and(|m| !is_valid_url(m)) {
        errors.insert(
            "menu".into(),
            "Use a full link such as https://example.com/lunch".into(),
        );
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(RestaurantInput {
        name: name.unwrap_or_default(),
        address: None,
        walk_min,
        cuisine: cuisine.unwrap_or_default().to_lowercase(),
        price_level,
        opening_hours,
        menu_url,
    })
}

pub fn errors_response(errors: &FieldErrors) -> Value {
    json!({"response_action": "errors", "errors": errors})
}
