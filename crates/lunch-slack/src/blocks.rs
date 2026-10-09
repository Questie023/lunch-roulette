//! Block Kit rendering of the lunch board. Pure functions → `serde_json::Value`.

use lunch_core::model::{
    FilterStage, MAX_CUISINE_CHARS, MAX_NAME_CHARS, PickId, Rating, Restaurant, SessionId, SessionView, ShownPick,
    is_valid_url,
};
use serde_json::{Value, json};

use crate::actions::{ActionValue, LunchAction};

pub const HELP: &str = "*Lunch Roulette*\n\
    • `/lunch` — open today's lunch board in this channel\n\
    • `/lunch add` — add a restaurant\n\
    • `/lunch help` — show this message";

/// Escapes `&`, `<` and `>` for mrkdwn text.
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Keeps at most `max` characters, ending in `…` when shortened. Clip before `escape`.
fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max - 1).chain(['…']).collect()
    }
}

/// Slack caps plain-text labels (buttons, options) at 75 characters.
fn label(s: &str) -> String {
    clip(s, 75)
}

/// A reason may quote a cuisine typed by a user.
const MAX_REASON_CHARS: usize = 100;
/// Slack caps a text element at 3000 characters; names past this budget collapse into "+N more".
const PARTICIPANTS_BUDGET: usize = 2800;
const MAX_PARTICIPANT_CHARS: usize = 40;

fn button(text: &str, action: LunchAction, value: ActionValue) -> Value {
    json!({
        "type": "button",
        "text": {"type": "plain_text", "text": text, "emoji": true},
        "action_id": action.id(),
        "value": value.encode(),
    })
}

fn primary(mut b: Value) -> Value {
    b["style"] = json!("primary");
    b
}

fn price(level: u8) -> String {
    "💰".repeat(usize::from(level))
}

fn participants_line(view: &SessionView) -> String {
    if view.participants.is_empty() {
        return "_Nobody has joined yet._".into();
    }
    let total = view.participants.len();
    let mut line = format!("*Going ({total}):* ");
    let mut shown = 0;
    for user in &view.participants {
        let name = escape(&clip(&user.display_name, MAX_PARTICIPANT_CHARS));
        if line.chars().count() + name.chars().count() + 2 > PARTICIPANTS_BUDGET {
            break;
        }
        if shown > 0 {
            line.push_str(", ");
        }
        line.push_str(&name);
        shown += 1;
    }
    if shown < total {
        line.push_str(&format!(" +{} more", total - shown));
    }
    line
}

fn pick_blocks(pick: &ShownPick) -> Vec<Value> {
    let r = &pick.restaurant;
    let name = escape(&clip(&r.name, MAX_NAME_CHARS));
    let title = if pick.accepted {
        format!("✅ *We're going to {name}*")
    } else {
        format!("🎲 *Today's pick: {name}*")
    };
    let walk = pick
        .explanation
        .walk_min
        .map_or_else(|| "🚶 ? min".to_string(), |m| format!("🚶 {m} min"));
    let cuisine = escape(&clip(&r.cuisine, MAX_CUISINE_CHARS));
    let mut text = format!("{title}\n🍽️ {cuisine} · {walk} · {}", price(r.price_level));
    if !pick.explanation.reasons.is_empty() {
        text.push_str("\n*Why:*");
        for reason in &pick.explanation.reasons {
            text.push_str(&format!("\n• {}", escape(&clip(&reason.to_string(), MAX_REASON_CHARS))));
        }
    }
    let mut out = vec![json!({"type": "section", "text": {"type": "mrkdwn", "text": text}})];
    if !pick.explanation.warnings.is_empty() {
        let lines: Vec<String> = pick.explanation.warnings.iter().map(|w| w.to_string()).collect();
        out.push(json!({"type": "context", "elements": [{"type": "mrkdwn", "text": lines.join("\n")}]}));
    }
    out
}

fn actions_block(view: &SessionView, restaurants: &[Restaurant], notice: Option<FilterStage>) -> Value {
    let sid = view.session.id;
    let v = ActionValue::session(sid);
    let mut elements = vec![
        button("🙋 Join", LunchAction::Join, v),
        button("Leave", LunchAction::Leave, v),
    ];
    let roll = if view.latest_pick.is_some() {
        "🔄 Try another"
    } else {
        "🎲 Pick a place"
    };
    elements.push(primary(button(roll, LunchAction::Pick, v)));
    if notice == Some(FilterStage::TooFar) {
        elements.push(button("🚶 Ignore distance", LunchAction::IgnoreDistance, v));
    }
    if let Some(pick) = view.latest_pick.as_ref().filter(|p| !p.accepted) {
        elements.push(primary(button(
            "✅ Let's go",
            LunchAction::Accept,
            ActionValue::with_target(sid, pick.pick_id),
        )));
    }
    if !restaurants.is_empty() {
        let options: Vec<Value> = restaurants
            .iter()
            .take(100)
            .map(|r| {
                json!({
                    "text": {"type": "plain_text", "text": label(&r.name)},
                    "value": ActionValue::with_target(sid, r.id).encode(),
                })
            })
            .collect();
        elements.push(json!({
            "type": "static_select",
            "action_id": LunchAction::Manual.id(),
            "placeholder": {"type": "plain_text", "text": "Pick by hand"},
            "options": options,
        }));
    }
    let menu = view.latest_pick.as_ref().and_then(|p| p.restaurant.menu_url.as_deref());
    if let Some(url) = menu.filter(|u| is_valid_url(u)) {
        elements.push(json!({
            "type": "button",
            "text": {"type": "plain_text", "text": "📋 Today's menu", "emoji": true},
            "action_id": LunchAction::Menu.id(),
            "url": url,
        }));
    }
    json!({"type": "actions", "block_id": "lunch_actions", "elements": elements})
}

/// Feedback values name the pick, so a rating lands on the place that was actually shown.
fn feedback_block(sid: SessionId, pick: PickId) -> Value {
    let v = ActionValue::with_target(sid, pick);
    json!({
        "type": "actions",
        "block_id": "lunch_feedback",
        "elements": [
            button("👍 Good choice", LunchAction::Feedback(Rating::Good), v),
            button("🙂 Okay", LunchAction::Feedback(Rating::Ok), v),
            button("👎 Not for me", LunchAction::Feedback(Rating::Bad), v),
        ],
    })
}

/// The whole board: header, participants, latest pick, empty-result notice, buttons, feedback.
pub fn session_message(view: &SessionView, restaurants: &[Restaurant], notice: Option<FilterStage>) -> Value {
    let mut blocks = vec![
        json!({"type": "section", "text": {"type": "mrkdwn",
            "text": format!("🍴 *Lunch today* · {}", view.session.date.strftime("%a %-d %b"))}}),
        json!({"type": "context", "elements": [{"type": "mrkdwn", "text": participants_line(view)}]}),
    ];
    if let Some(pick) = &view.latest_pick {
        blocks.push(json!({"type": "divider"}));
        blocks.extend(pick_blocks(pick));
    }
    if let Some(stage) = notice {
        blocks.push(json!({"type": "section", "text": {"type": "mrkdwn", "text": format!("🤷 {stage}")}}));
    }
    blocks.push(actions_block(view, restaurants, notice));
    if let Some(pick) = view.latest_pick.as_ref().filter(|p| p.accepted) {
        blocks.push(json!({"type": "context", "elements": [{"type": "mrkdwn", "text": "How was it?"}]}));
        blocks.push(feedback_block(view.session.id, pick.pick_id));
    }
    let fallback = match &view.latest_pick {
        Some(p) if p.accepted => format!("Lunch today: {}", p.restaurant.name),
        Some(p) => format!("Lunch pick: {}", p.restaurant.name),
        None => format!("Lunch today: {} going", view.participants.len()),
    };
    json!({"text": fallback, "blocks": blocks})
}

/// A reply only the clicking user sees; never replaces the shared board.
pub fn ephemeral(text: &str) -> Value {
    json!({"response_type": "ephemeral", "replace_original": false, "text": text})
}
