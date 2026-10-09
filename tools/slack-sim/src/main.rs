//! Signs and POSTs fake Slack requests to a local `wrangler dev`.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

const USAGE: &str = "usage:
  slack-sim lunch [text]
  slack-sim action <action_id> <value> [message_ts]
  slack-sim add <name> <cuisine> <walk_min> <price 1-3> [hours] [menu_url]
  slack-sim bad-signature
env: SIM_URL (http://localhost:8787) SIM_SECRET (dev-signing-secret) SIM_USER (U_SIM) SIM_NAME (sim) SIM_CHANNEL (C_SIM)";

const RESPONSE_URL: &str = "https://example.invalid/response";

fn env(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn interaction(payload: Value) -> String {
    serde_urlencoded::to_string([("payload", payload.to_string())]).expect("form encoding")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let user = env("SIM_USER", "U_SIM");
    let name = env("SIM_NAME", "sim");
    let channel = env("SIM_CHANNEL", "C_SIM");

    let (path, body, signed) = match args.first().map(String::as_str) {
        Some("lunch") => {
            let text = args.get(1).map_or("", String::as_str);
            let form = serde_urlencoded::to_string([
                ("command", "/lunch"),
                ("text", text),
                ("user_id", user.as_str()),
                ("user_name", name.as_str()),
                ("channel_id", channel.as_str()),
                ("response_url", RESPONSE_URL),
                ("trigger_id", "sim.trigger"),
            ])
            .expect("form encoding");
            ("/slack/commands", form, true)
        }
        Some("action") if args.len() >= 3 => {
            let (action_id, value) = (&args[1], &args[2]);
            let ts = args.get(3).map_or("1700000000.000100", String::as_str);
            let action = if action_id == "manual" {
                json!({"action_id": action_id, "selected_option": {"value": value}})
            } else {
                json!({"action_id": action_id, "value": value})
            };
            let payload = json!({
                "type": "block_actions",
                "user": {"id": user, "username": name, "name": name},
                "channel": {"id": channel},
                "message": {"ts": ts},
                "response_url": RESPONSE_URL,
                "trigger_id": "sim.trigger",
                "actions": [action],
            });
            ("/slack/interactions", interaction(payload), true)
        }
        Some("add") if args.len() >= 5 => {
            let text = |v: &str| json!({"v": {"type": "plain_text_input", "value": v}});
            let mut values = Map::new();
            values.insert("name".into(), text(&args[1]));
            values.insert("cuisine".into(), text(&args[2]));
            values.insert("walk".into(), text(&args[3]));
            values.insert(
                "price".into(),
                json!({"v": {"type": "static_select", "selected_option": {"value": args[4]}}}),
            );
            if let Some(hours) = args.get(5) {
                values.insert("hours".into(), text(hours));
            }
            if let Some(menu) = args.get(6) {
                values.insert("menu".into(), text(menu));
            }
            let payload = json!({
                "type": "view_submission",
                "user": {"id": user, "name": name},
                "view": {"callback_id": "add_restaurant", "private_metadata": channel, "state": {"values": values}},
            });
            ("/slack/interactions", interaction(payload), true)
        }
        Some("bad-signature") => ("/slack/commands", "command=%2Flunch".to_string(), false),
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    };

    let url = format!("{}{path}", env("SIM_URL", "http://localhost:8787"));
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970")
        .as_secs()
        .to_string();
    let signature = if signed {
        lunch_slack::verify::sign(&env("SIM_SECRET", "dev-signing-secret"), &ts, &body)
    } else {
        "v0=deadbeef".to_string()
    };
    let started = Instant::now();
    let result = ureq::post(url.as_str())
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("X-Slack-Request-Timestamp", ts.as_str())
        .header("X-Slack-Signature", signature.as_str())
        .send(body.as_str());
    let elapsed = started.elapsed().as_millis();
    match result {
        Ok(mut response) => {
            let status = response.status();
            let text = response.body_mut().read_to_string().unwrap_or_default();
            println!("{status} in {elapsed} ms\n{text}");
        }
        Err(ureq::Error::StatusCode(code)) => println!("{code} in {elapsed} ms"),
        Err(e) => {
            eprintln!("request failed: {e}");
            std::process::exit(1);
        }
    }
}
