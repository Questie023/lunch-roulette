//! Interactive element ids and the values they carry. Every value starts with the session id,
//! so clicks on an older board still act on the right session.

use lunch_core::model::{Rating, SessionId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LunchAction {
    Join,
    Leave,
    /// "Pick a place" and "Try another" — re-rolls skip what was already shown.
    Pick,
    IgnoreDistance,
    Manual,
    Accept,
    Feedback(Rating),
    /// URL button; Slack still sends a click event, which is ignored.
    Menu,
}

const ALL: [LunchAction; 10] = [
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

impl LunchAction {
    pub fn id(self) -> &'static str {
        match self {
            LunchAction::Join => "join",
            LunchAction::Leave => "leave",
            LunchAction::Pick => "pick",
            LunchAction::IgnoreDistance => "ignore_distance",
            LunchAction::Manual => "manual",
            LunchAction::Accept => "accept",
            LunchAction::Feedback(Rating::Good) => "fb_good",
            LunchAction::Feedback(Rating::Ok) => "fb_ok",
            LunchAction::Feedback(Rating::Bad) => "fb_bad",
            LunchAction::Menu => "menu",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        ALL.into_iter().find(|a| a.id() == id)
    }
}

/// `"<session>"` or `"<session>:<target>"`, where target is a pick id or a restaurant id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionValue {
    pub session: SessionId,
    pub target: Option<i64>,
}

impl ActionValue {
    pub fn session(session: SessionId) -> Self {
        ActionValue { session, target: None }
    }

    pub fn with_target(session: SessionId, target: i64) -> Self {
        ActionValue {
            session,
            target: Some(target),
        }
    }

    pub fn encode(self) -> String {
        match self.target {
            Some(t) => format!("{}:{t}", self.session),
            None => self.session.to_string(),
        }
    }

    pub fn decode(s: &str) -> Option<Self> {
        let (session, target) = match s.split_once(':') {
            Some((a, b)) => (a, Some(b)),
            None => (s, None),
        };
        let target = match target {
            Some(t) => Some(t.parse().ok()?),
            None => None,
        };
        Some(ActionValue {
            session: session.parse().ok()?,
            target,
        })
    }
}
