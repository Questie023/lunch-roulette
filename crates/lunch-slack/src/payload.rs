//! Incoming Slack payloads: slash commands (form-encoded) and interactions (`payload=<json>`).

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum PayloadError {
    #[error("bad form body: {0}")]
    Form(#[from] serde_urlencoded::de::Error),
    #[error("bad interaction JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SlashCommand {
    pub command: String,
    #[serde(default)]
    pub text: String,
    pub user_id: String,
    #[serde(default)]
    pub user_name: String,
    pub channel_id: String,
    pub response_url: String,
    pub trigger_id: String,
}

pub fn parse_slash(body: &str) -> Result<SlashCommand, PayloadError> {
    Ok(serde_urlencoded::from_str(body)?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LunchCommand {
    Open,
    Add,
    Help,
}

pub fn parse_lunch_text(text: &str) -> LunchCommand {
    match text.trim().to_ascii_lowercase().as_str() {
        "" => LunchCommand::Open,
        "add" => LunchCommand::Add,
        _ => LunchCommand::Help,
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Interaction {
    BlockActions(BlockActions),
    ViewSubmission(ViewSubmission),
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockActions {
    pub user: SlackUser,
    #[serde(default)]
    pub channel: Option<IdOnly>,
    #[serde(default)]
    pub message: Option<MessageRef>,
    #[serde(default)]
    pub response_url: Option<String>,
    pub trigger_id: String,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlackUser {
    pub id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

impl SlackUser {
    /// Best name available in the payload; the worker upgrades it via `users.info` on join.
    pub fn display(&self) -> String {
        self.name
            .clone()
            .or_else(|| self.username.clone())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| self.id.clone())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdOnly {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageRef {
    pub ts: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Action {
    pub action_id: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub selected_option: Option<SelectedOption>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SelectedOption {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ViewSubmission {
    pub user: SlackUser,
    pub view: View,
}

#[derive(Debug, Clone, Deserialize)]
pub struct View {
    pub callback_id: String,
    #[serde(default)]
    pub private_metadata: String,
    pub state: ViewState,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ViewState {
    /// block_id → action_id → value
    pub values: HashMap<String, HashMap<String, InputValue>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InputValue {
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub selected_option: Option<SelectedOption>,
}

pub fn parse_interaction(body: &str) -> Result<Interaction, PayloadError> {
    #[derive(Deserialize)]
    struct Form {
        payload: String,
    }
    let form: Form = serde_urlencoded::from_str(body)?;
    Ok(serde_json::from_str(&form.payload)?)
}
