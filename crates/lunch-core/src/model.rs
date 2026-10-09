use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use jiff::Timestamp;
use jiff::civil::{Date, Time};
use serde::{Deserialize, Serialize};

use crate::Error;

pub type UserId = String;
pub type RestaurantId = i64;
pub type SessionId = i64;
pub type PickId = i64;

/// Implements `ALL`, `as_str`, `FromStr` and `Display` for a fieldless enum stored as TEXT.
macro_rules! string_enum {
    ($name:ident { $($variant:ident => $s:literal),+ $(,)? }) => {
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $s),+ }
            }
        }

        impl std::str::FromStr for $name {
            type Err = Error;

            fn from_str(s: &str) -> crate::Result<Self> {
                match s {
                    $($s => Ok($name::$variant),)+
                    other => Err(Error::Invalid(format!(concat!("unknown ", stringify!($name), ": {}"), other))),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

/// A hard dietary requirement: "I need an option that is …".
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DietTag {
    Vegetarian,
    Vegan,
    Halal,
    Kosher,
    GlutenFree,
    CrustaceanFree,
    EggFree,
    FishFree,
    PeanutFree,
    SoyFree,
    MilkFree,
    NutFree,
    CeleryFree,
    MustardFree,
    SesameFree,
    SulphiteFree,
    LupinFree,
    MolluscFree,
}

string_enum!(DietTag {
    Vegetarian => "vegetarian",
    Vegan => "vegan",
    Halal => "halal",
    Kosher => "kosher",
    GlutenFree => "gluten_free",
    CrustaceanFree => "crustacean_free",
    EggFree => "egg_free",
    FishFree => "fish_free",
    PeanutFree => "peanut_free",
    SoyFree => "soy_free",
    MilkFree => "milk_free",
    NutFree => "nut_free",
    CeleryFree => "celery_free",
    MustardFree => "mustard_free",
    SesameFree => "sesame_free",
    SulphiteFree => "sulphite_free",
    LupinFree => "lupin_free",
    MolluscFree => "mollusc_free",
});

impl DietTag {
    pub fn label(self) -> &'static str {
        match self {
            DietTag::Vegetarian => "Vegetarian",
            DietTag::Vegan => "Vegan",
            DietTag::Halal => "Halal",
            DietTag::Kosher => "Kosher",
            DietTag::GlutenFree => "Gluten-free",
            DietTag::CrustaceanFree => "No crustaceans",
            DietTag::EggFree => "No eggs",
            DietTag::FishFree => "No fish",
            DietTag::PeanutFree => "No peanuts",
            DietTag::SoyFree => "No soy",
            DietTag::MilkFree => "No milk",
            DietTag::NutFree => "No tree nuts",
            DietTag::CeleryFree => "No celery",
            DietTag::MustardFree => "No mustard",
            DietTag::SesameFree => "No sesame",
            DietTag::SulphiteFree => "No sulphites",
            DietTag::LupinFree => "No lupin",
            DietTag::MolluscFree => "No molluscs",
        }
    }
}

/// What a restaurant declares about a diet tag. A missing entry means "unknown".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Support {
    Yes,
    No,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub office_lat: f64,
    pub office_lon: f64,
    pub timezone: String,
    pub lunch_start: Time,
    pub lunch_end: Time,
    pub default_max_walk_min: u32,
    pub currency: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            office_lat: 59.33258,
            office_lon: 18.06490,
            timezone: "Europe/Stockholm".into(),
            lunch_start: jiff::civil::time(11, 30, 0, 0),
            lunch_end: jiff::civil::time(13, 30, 0, 0),
            default_max_walk_min: 10,
            currency: "SEK".into(),
        }
    }
}

impl Settings {
    /// Builds settings from `settings` table rows. Unknown keys are ignored; missing keys keep defaults.
    pub fn from_pairs<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> crate::Result<Self> {
        let mut s = Settings::default();
        for (key, value) in pairs {
            let bad = |e: &dyn fmt::Display| Error::Invalid(format!("setting {key}={value}: {e}"));
            match key {
                "office_lat" => s.office_lat = value.parse().map_err(|e| bad(&e))?,
                "office_lon" => s.office_lon = value.parse().map_err(|e| bad(&e))?,
                "timezone" => s.timezone = value.to_string(),
                "lunch_start" => s.lunch_start = value.parse().map_err(|e| bad(&e))?,
                "lunch_end" => s.lunch_end = value.parse().map_err(|e| bad(&e))?,
                "default_max_walk_min" => s.default_max_walk_min = value.parse().map_err(|e| bad(&e))?,
                "currency" => s.currency = value.to_string(),
                _ => {}
            }
        }
        Ok(s)
    }

    pub fn time_zone(&self) -> crate::Result<jiff::tz::TimeZone> {
        jiff::tz::TimeZone::get(&self.timezone).map_err(|e| Error::Invalid(format!("timezone {}: {e}", self.timezone)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub display_name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UserPrefs {
    pub max_walk_min: Option<u32>,
    pub price_pref: Option<u8>,
    pub constraints: BTreeSet<DietTag>,
    /// Cuisine (lowercase) → weight in −2..=2.
    pub cuisine_weights: BTreeMap<String, i8>,
    pub favorites: BTreeSet<RestaurantId>,
    /// The person's latest rating of each place (from feedback, not edited as a preference).
    pub ratings: BTreeMap<RestaurantId, Rating>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Restaurant {
    pub id: RestaurantId,
    pub name: String,
    pub address: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// Explicit walking time from the office; wins over coordinates.
    pub walk_min: Option<u32>,
    pub cuisine: String,
    /// 1 = cheap, 2 = normal, 3 = pricey.
    pub price_level: u8,
    pub opening_hours: Option<String>,
    pub website: Option<String>,
    pub menu_url: Option<String>,
    pub source: String,
    pub active: bool,
    pub diet_support: BTreeMap<DietTag, Support>,
}

/// Input limits for user-entered restaurant fields, in characters. Slack caps a text block at
/// 3000 characters and escaping can grow text fivefold (`&` → `&amp;`), so keep these small.
pub const MAX_NAME_CHARS: usize = 60;
pub const MAX_CUISINE_CHARS: usize = 30;
pub const MAX_HOURS_CHARS: usize = 100;
pub const MAX_URL_CHARS: usize = 300;

/// An `http(s)` link Slack accepts as a button URL: a dotted host, no whitespace or control characters.
pub fn is_valid_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")) else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    url.chars().count() <= MAX_URL_CHARS
        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
        && host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
}

fn at_most(value: &str, max: usize, what: &str) -> crate::Result<()> {
    if value.chars().count() > max {
        return Err(Error::Invalid(format!("{what} must be at most {max} characters")));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RestaurantInput {
    pub name: String,
    pub address: Option<String>,
    pub walk_min: Option<u32>,
    pub cuisine: String,
    pub price_level: u8,
    pub opening_hours: Option<String>,
    pub menu_url: Option<String>,
}

impl RestaurantInput {
    pub fn validate(&self) -> crate::Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::Invalid("name is required".into()));
        }
        if self.cuisine.trim().is_empty() {
            return Err(Error::Invalid("cuisine is required".into()));
        }
        at_most(self.name.trim(), MAX_NAME_CHARS, "name")?;
        at_most(self.cuisine.trim(), MAX_CUISINE_CHARS, "cuisine")?;
        if let Some(hours) = &self.opening_hours {
            at_most(hours, MAX_HOURS_CHARS, "opening hours")?;
        }
        if let Some(url) = &self.menu_url
            && !is_valid_url(url)
        {
            return Err(Error::Invalid("menu link must be a full https:// address".into()));
        }
        if !(1..=3).contains(&self.price_level) {
            return Err(Error::Invalid("price level must be 1–3".into()));
        }
        if let Some(w) = self.walk_min
            && !(1..=60).contains(&w)
        {
            return Err(Error::Invalid("walking time must be 1–60 minutes".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LunchOffer {
    pub id: i64,
    pub restaurant_id: RestaurantId,
    pub date: Date,
    pub name: String,
    pub description: Option<String>,
    pub price_sek: Option<u32>,
    /// Diet tags this dish is known to satisfy.
    pub diet_tags: BTreeSet<DietTag>,
    pub source: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Open,
    Decided,
}

string_enum!(SessionStatus { Open => "open", Decided => "decided" });

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub channel_id: String,
    pub date: Date,
    pub message_ts: Option<String>,
    pub status: SessionStatus,
    pub created_by: UserId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PickKind {
    Roll,
    Manual,
}

string_enum!(PickKind { Roll => "roll", Manual => "manual" });

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rating {
    Good,
    Ok,
    Bad,
}

string_enum!(Rating { Good => "good", Ok => "ok", Bad => "bad" });

/// Why a restaurant was suggested. Aggregated only — never names a person.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reason {
    CuisineLiked {
        cuisine: String,
        count: u32,
    },
    Favorite {
        count: u32,
    },
    /// `of` participants rated the place; `liked` of them gave 👍.
    LikedBefore {
        liked: u32,
        of: u32,
    },
    NotRecentlyVisited,
    ManualChoice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Warning {
    HoursUnknown,
    DistanceUnknown,
    DietInfoUnknown,
    DistanceIgnored,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Explanation {
    pub walk_min: Option<u32>,
    pub reasons: Vec<Reason>,
    pub warnings: Vec<Warning>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShownPick {
    pub pick_id: PickId,
    pub restaurant: Restaurant,
    pub kind: PickKind,
    pub accepted: bool,
    pub explanation: Explanation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionView {
    pub session: Session,
    pub participants: Vec<User>,
    pub latest_pick: Option<ShownPick>,
}

/// The pipeline stage that removed the last candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterStage {
    NoRestaurants,
    AllShown,
    Closed,
    TooFar,
    DietConflict,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PickOptions {
    pub ignore_distance: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PickOutcome {
    Picked {
        restaurant_id: RestaurantId,
        explanation: Explanation,
    },
    Empty {
        stage: FilterStage,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum PickResponse {
    Picked(SessionView),
    Empty { stage: FilterStage, view: SessionView },
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewPick {
    pub session_id: SessionId,
    pub restaurant_id: RestaurantId,
    pub kind: PickKind,
    pub explanation: Explanation,
    pub created_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewFeedback {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub restaurant_id: RestaurantId,
    pub rating: Rating,
    pub created_at: Timestamp,
}
