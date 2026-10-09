use crate::model::{Restaurant, Settings};

const EARTH_RADIUS_M: f64 = 6_371_000.0;
/// Streets are not straight lines.
const DETOUR_FACTOR: f64 = 1.3;
const WALK_M_PER_MIN: f64 = 80.0;

pub fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * a.sqrt().asin()
}

/// Explicit `walk_min` wins; otherwise estimated from coordinates; `None` if neither is known.
pub fn walk_minutes(r: &Restaurant, s: &Settings) -> Option<u32> {
    if let Some(minutes) = r.walk_min {
        return Some(minutes);
    }
    let (lat, lon) = (r.lat?, r.lon?);
    let meters = haversine_m(s.office_lat, s.office_lon, lat, lon) * DETOUR_FACTOR;
    Some((meters / WALK_M_PER_MIN).ceil().max(1.0) as u32)
}
