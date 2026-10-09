//! Minimal Slack Web API client. `SLACK_DRY_RUN=1` logs calls instead of sending them.

use serde_json::{Value, json};
use worker::wasm_bindgen::JsValue;
use worker::{Env, Error, Fetch, Headers, Method, Request, RequestInit, Result, console_log};

pub struct SlackClient {
    token: String,
    dry_run: bool,
}

impl SlackClient {
    pub fn from_env(env: &Env) -> Result<Self> {
        Ok(SlackClient {
            token: env.secret("SLACK_BOT_TOKEN")?.to_string(),
            dry_run: crate::app::flag(env, "SLACK_DRY_RUN"),
        })
    }

    /// Calls a write method with a JSON body; errors when Slack answers `ok: false`.
    pub async fn call(&self, method: &str, body: &Value) -> Result<Value> {
        if self.dry_run {
            console_log!("[slack dry-run] {method} {body}");
            return Ok(json!({"ok": true, "ts": "1700000000.000100"}));
        }
        let response = post_json(&format!("https://slack.com/api/{method}"), Some(&self.token), body).await?;
        if response["ok"].as_bool() == Some(true) {
            Ok(response)
        } else {
            let error = response["error"].as_str().unwrap_or("unknown_error");
            Err(Error::RustError(format!("slack {method}: {error}")))
        }
    }

    /// Best-effort display name via `users.info`; `None` in dry-run or on any failure.
    pub async fn display_name(&self, user_id: &str) -> Option<String> {
        if self.dry_run {
            return None;
        }
        let headers = Headers::new();
        headers.set("Authorization", &format!("Bearer {}", self.token)).ok()?;
        let mut init = RequestInit::new();
        init.with_method(Method::Get).with_headers(headers);
        let url = format!("https://slack.com/api/users.info?user={user_id}");
        let request = Request::new_with_init(&url, &init).ok()?;
        let v: Value = Fetch::Request(request).send().await.ok()?.json().await.ok()?;
        let profile = &v["user"]["profile"];
        [&profile["display_name"], &profile["real_name"], &v["user"]["name"]]
            .into_iter()
            .filter_map(Value::as_str)
            .find(|s| !s.trim().is_empty())
            .map(str::to_string)
    }

    /// Replies through a `response_url` (ephemeral messages); needs no token.
    pub async fn respond(&self, response_url: &str, body: &Value) -> Result<()> {
        if self.dry_run {
            console_log!("[slack dry-run] respond {body}");
            return Ok(());
        }
        post_json(response_url, None, body).await.map(|_| ())
    }
}

async fn post_json(url: &str, token: Option<&str>, body: &Value) -> Result<Value> {
    let headers = Headers::new();
    headers.set("Content-Type", "application/json; charset=utf-8")?;
    if let Some(token) = token {
        headers.set("Authorization", &format!("Bearer {token}"))?;
    }
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(JsValue::from_str(&body.to_string())));
    let mut response = Fetch::Request(Request::new_with_init(url, &init)?).send().await?;
    let text = response.text().await?;
    // response_url answers plain "ok", not JSON.
    Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
}
