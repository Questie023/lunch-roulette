//! Row shapes returned by `sql::*` (deserialized by column name) and their assembly into domain
//! types. Integers arrive as i64 and booleans as 0/1 on both SQLite and D1.

use std::collections::BTreeMap;

use jiff::civil::Date;
use serde::Deserialize;

use crate::model::{
    DietTag, LunchOffer, Restaurant, RestaurantId, Session, Settings, ShownPick, Support, User, UserPrefs,
};
use crate::{Error, Result};

#[derive(Debug, Deserialize)]
pub struct SettingRow {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct UserRow {
    pub id: String,
    pub display_name: String,
    pub max_walk_min: Option<i64>,
    pub price_pref: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UserTagRow {
    pub user_id: String,
    pub tag: String,
}

#[derive(Debug, Deserialize)]
pub struct CuisineRow {
    pub user_id: String,
    pub cuisine: String,
    pub weight: i64,
}

#[derive(Debug, Deserialize)]
pub struct StanceRow {
    pub user_id: String,
    pub restaurant_id: i64,
    pub stance: String,
}

#[derive(Debug, Deserialize)]
pub struct RatingRow {
    pub user_id: String,
    pub restaurant_id: i64,
    pub rating: String,
}

#[derive(Debug, Deserialize)]
pub struct RestaurantRow {
    pub id: i64,
    pub name: String,
    pub address: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub walk_min: Option<i64>,
    pub cuisine: String,
    pub price_level: i64,
    pub opening_hours: Option<String>,
    pub website: Option<String>,
    pub menu_url: Option<String>,
    pub source: String,
    pub active: i64,
}

#[derive(Debug, Deserialize)]
pub struct DietSupportRow {
    pub restaurant_id: i64,
    pub tag: String,
    pub supported: i64,
}

#[derive(Debug, Deserialize)]
pub struct OfferRow {
    pub id: i64,
    pub restaurant_id: i64,
    pub date: String,
    pub name: String,
    pub description: Option<String>,
    pub price_sek: Option<i64>,
    pub diet_tags: String,
    pub source: String,
}

#[derive(Debug, Deserialize)]
pub struct SessionRow {
    pub id: i64,
    pub channel_id: String,
    pub date: String,
    pub message_ts: Option<String>,
    pub status: String,
    pub created_by: String,
}

#[derive(Debug, Deserialize)]
pub struct PickRow {
    pub id: i64,
    pub restaurant_id: i64,
    pub kind: String,
    pub accepted: i64,
    pub explanation: String,
}

#[derive(Debug, Deserialize)]
pub struct RestaurantDateRow {
    pub restaurant_id: i64,
    pub date: String,
}

#[derive(Debug, Deserialize)]
pub struct IdRow {
    pub id: i64,
}

fn corrupt(what: &str, e: impl std::fmt::Display) -> Error {
    Error::Storage(format!("bad {what} in database: {e}"))
}

fn date(s: &str) -> Result<Date> {
    s.parse().map_err(|e| corrupt("date", e))
}

fn small<T: TryFrom<i64>>(what: &str, v: i64) -> Result<T> {
    T::try_from(v).map_err(|_| corrupt(what, v))
}

pub fn settings(rows: &[SettingRow]) -> Result<Settings> {
    Settings::from_pairs(rows.iter().map(|r| (r.key.as_str(), r.value.as_str())))
}

pub fn users_with_prefs(
    users: Vec<UserRow>,
    tags: Vec<UserTagRow>,
    cuisines: Vec<CuisineRow>,
    stances: Vec<StanceRow>,
    ratings: Vec<RatingRow>,
) -> Result<Vec<(User, UserPrefs)>> {
    let mut out: Vec<(User, UserPrefs)> = Vec::with_capacity(users.len());
    let mut index = BTreeMap::new();
    for row in users {
        let prefs = UserPrefs {
            max_walk_min: row.max_walk_min.map(|v| small("max_walk_min", v)).transpose()?,
            price_pref: row.price_pref.map(|v| small("price_pref", v)).transpose()?,
            ..UserPrefs::default()
        };
        index.insert(row.id.clone(), out.len());
        out.push((
            User {
                id: row.id,
                display_name: row.display_name,
            },
            prefs,
        ));
    }
    for t in tags {
        if let Some(&i) = index.get(&t.user_id) {
            out[i].1.constraints.insert(t.tag.parse()?);
        }
    }
    for c in cuisines {
        if let Some(&i) = index.get(&c.user_id) {
            out[i].1.cuisine_weights.insert(c.cuisine, small("weight", c.weight)?);
        }
    }
    for s in stances {
        if let Some(&i) = index.get(&s.user_id) {
            match s.stance.as_str() {
                "favorite" => {
                    out[i].1.favorites.insert(s.restaurant_id);
                }
                // Legacy vetoes are deleted by migration 0003 and never written again.
                "never" => {}
                other => return Err(corrupt("stance", other)),
            }
        }
    }
    for r in ratings {
        if let Some(&i) = index.get(&r.user_id) {
            out[i].1.ratings.insert(r.restaurant_id, r.rating.parse()?);
        }
    }
    Ok(out)
}

pub fn restaurants(rows: Vec<RestaurantRow>, support: Vec<DietSupportRow>) -> Result<Vec<Restaurant>> {
    let mut by_restaurant: BTreeMap<i64, BTreeMap<DietTag, Support>> = BTreeMap::new();
    for s in support {
        let value = if s.supported == 1 { Support::Yes } else { Support::No };
        by_restaurant
            .entry(s.restaurant_id)
            .or_default()
            .insert(s.tag.parse()?, value);
    }
    rows.into_iter()
        .map(|r| {
            Ok(Restaurant {
                id: r.id,
                name: r.name,
                address: r.address,
                lat: r.lat,
                lon: r.lon,
                walk_min: r.walk_min.map(|v| small("walk_min", v)).transpose()?,
                cuisine: r.cuisine,
                price_level: small("price_level", r.price_level)?,
                opening_hours: r.opening_hours,
                website: r.website,
                menu_url: r.menu_url,
                source: r.source,
                active: r.active == 1,
                diet_support: by_restaurant.remove(&r.id).unwrap_or_default(),
            })
        })
        .collect()
}

pub fn offers(rows: Vec<OfferRow>) -> Result<Vec<LunchOffer>> {
    rows.into_iter()
        .map(|r| {
            Ok(LunchOffer {
                id: r.id,
                restaurant_id: r.restaurant_id,
                date: date(&r.date)?,
                name: r.name,
                description: r.description,
                price_sek: r.price_sek.map(|v| small("price_sek", v)).transpose()?,
                diet_tags: serde_json::from_str(&r.diet_tags).map_err(|e| corrupt("diet_tags", e))?,
                source: r.source,
            })
        })
        .collect()
}

pub fn session(row: SessionRow) -> Result<Session> {
    Ok(Session {
        id: row.id,
        channel_id: row.channel_id,
        date: date(&row.date)?,
        message_ts: row.message_ts,
        status: row.status.parse()?,
        created_by: row.created_by,
    })
}

pub fn shown_pick(row: PickRow, restaurant: Restaurant) -> Result<ShownPick> {
    Ok(ShownPick {
        pick_id: row.id,
        restaurant,
        kind: row.kind.parse()?,
        accepted: row.accepted == 1,
        explanation: serde_json::from_str(&row.explanation).map_err(|e| corrupt("explanation", e))?,
    })
}

pub fn last_accepted(rows: Vec<RestaurantDateRow>) -> Result<BTreeMap<RestaurantId, Date>> {
    rows.into_iter()
        .map(|r| Ok((r.restaurant_id, date(&r.date)?)))
        .collect()
}
