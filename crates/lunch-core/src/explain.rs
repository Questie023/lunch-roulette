//! English UI strings for explanation parts. Shared by Slack now and the TUI later.

use std::fmt;

use crate::model::{FilterStage, Reason, Warning};

fn people(count: u32) -> String {
    if count == 1 {
        "1 person".into()
    } else {
        format!("{count} people")
    }
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reason::CuisineLiked { cuisine, count } => {
                let verb = if *count == 1 { "likes" } else { "like" };
                write!(f, "{} {verb} {cuisine} food", people(*count))
            }
            Reason::Favorite { count } => write!(f, "a favourite of {}", people(*count)),
            Reason::LikedBefore { liked, of } => write!(f, "{liked} of {of} liked it last time"),
            Reason::NotRecentlyVisited => f.write_str("you haven't been there recently"),
            Reason::ManualChoice => f.write_str("picked by hand"),
        }
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Warning::HoursUnknown => "⏰ Opening hours unknown — check before you go.",
            Warning::DistanceUnknown => "📍 Walking distance unknown.",
            Warning::DietInfoUnknown => {
                "⚠️ We don't know whether there's an option for everyone's dietary needs — please check with the staff."
            }
            Warning::DistanceIgnored => "🚶 The walking-distance limit was ignored for this pick.",
        })
    }
}

impl fmt::Display for FilterStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            FilterStage::NoRestaurants => "There are no restaurants yet. Add one with `/lunch add`.",
            FilterStage::AllShown => "You've already seen every option for today.",
            FilterStage::Closed => "Everything left is closed during the lunch window.",
            FilterStage::TooFar => "Everything left is further than the group's walking limit.",
            FilterStage::DietConflict => "Nothing left fits everyone's dietary needs.",
        })
    }
}
