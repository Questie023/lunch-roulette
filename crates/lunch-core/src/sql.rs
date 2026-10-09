//! Every SQL statement, shared verbatim by SQLite and D1. Numbered params only.

pub const SETTINGS: &str = "SELECT key, value FROM settings";

pub const UPSERT_USER: &str = "INSERT INTO users (id, display_name, created_at) VALUES (?1, ?2, ?3) \
    ON CONFLICT (id) DO UPDATE SET display_name = excluded.display_name";
/// Creates the user if missing but never renames: button payloads only carry the Slack handle.
pub const ENSURE_USER: &str = "INSERT INTO users (id, display_name, created_at) VALUES (?1, ?2, ?3) \
    ON CONFLICT (id) DO NOTHING";
pub const USER: &str = "SELECT id, display_name, max_walk_min, price_pref FROM users WHERE id = ?1";
pub const USER_TAGS: &str = "SELECT user_id, tag FROM user_constraints WHERE user_id = ?1";
pub const USER_CUISINES: &str = "SELECT user_id, cuisine, weight FROM user_cuisine_prefs WHERE user_id = ?1";
pub const USER_STANCES: &str = "SELECT user_id, restaurant_id, stance FROM user_restaurant_prefs WHERE user_id = ?1";
/// Latest rating per place; ties in `created_at` resolve to the newer row id.
pub const USER_RATINGS: &str = "SELECT f.user_id, f.restaurant_id, f.rating FROM feedback f \
    WHERE f.user_id = ?1 AND f.id = (SELECT g.id FROM feedback g WHERE g.user_id = f.user_id \
    AND g.restaurant_id = f.restaurant_id ORDER BY g.created_at DESC, g.id DESC LIMIT 1)";
pub const UPDATE_USER_LIMITS: &str = "UPDATE users SET max_walk_min = ?2, price_pref = ?3 WHERE id = ?1";
pub const CLEAR_USER_TAGS: &str = "DELETE FROM user_constraints WHERE user_id = ?1";
pub const CLEAR_USER_CUISINES: &str = "DELETE FROM user_cuisine_prefs WHERE user_id = ?1";
pub const CLEAR_USER_STANCES: &str = "DELETE FROM user_restaurant_prefs WHERE user_id = ?1";
pub const INSERT_USER_TAG: &str = "INSERT INTO user_constraints (user_id, tag) VALUES (?1, ?2)";
pub const INSERT_USER_CUISINE: &str = "INSERT INTO user_cuisine_prefs (user_id, cuisine, weight) VALUES (?1, ?2, ?3)";
pub const UPSERT_USER_STANCE: &str = "INSERT INTO user_restaurant_prefs (user_id, restaurant_id, stance) VALUES (?1, ?2, ?3) \
    ON CONFLICT (user_id, restaurant_id) DO UPDATE SET stance = excluded.stance";

pub const INSERT_SESSION_IF_MISSING: &str = "INSERT INTO lunch_sessions (channel_id, date, created_by, created_at) \
    VALUES (?1, ?2, ?3, ?4) ON CONFLICT (channel_id, date) DO NOTHING";
pub const SESSION_BY_CHANNEL_DATE: &str = "SELECT id, channel_id, date, message_ts, status, created_by \
    FROM lunch_sessions WHERE channel_id = ?1 AND date = ?2";
pub const SESSION_BY_ID: &str = "SELECT id, channel_id, date, message_ts, status, created_by \
    FROM lunch_sessions WHERE id = ?1";
pub const SET_MESSAGE_TS: &str = "UPDATE lunch_sessions SET message_ts = ?2 WHERE id = ?1";
pub const SET_SESSION_STATUS: &str = "UPDATE lunch_sessions SET status = ?2 WHERE id = ?1";

pub const JOIN: &str = "INSERT INTO session_participants (session_id, user_id, joined_at) VALUES (?1, ?2, ?3) \
    ON CONFLICT DO NOTHING";
pub const LEAVE: &str = "DELETE FROM session_participants WHERE session_id = ?1 AND user_id = ?2";
pub const PARTICIPANTS: &str = "SELECT u.id, u.display_name, u.max_walk_min, u.price_pref \
    FROM session_participants sp JOIN users u ON u.id = sp.user_id \
    WHERE sp.session_id = ?1 ORDER BY sp.joined_at, u.id";
pub const PARTICIPANT_TAGS: &str = "SELECT c.user_id, c.tag FROM user_constraints c \
    JOIN session_participants sp ON sp.user_id = c.user_id WHERE sp.session_id = ?1";
pub const PARTICIPANT_CUISINES: &str = "SELECT c.user_id, c.cuisine, c.weight FROM user_cuisine_prefs c \
    JOIN session_participants sp ON sp.user_id = c.user_id WHERE sp.session_id = ?1";
pub const PARTICIPANT_STANCES: &str = "SELECT s.user_id, s.restaurant_id, s.stance FROM user_restaurant_prefs s \
    JOIN session_participants sp ON sp.user_id = s.user_id WHERE sp.session_id = ?1";
pub const PARTICIPANT_RATINGS: &str = "SELECT f.user_id, f.restaurant_id, f.rating FROM feedback f \
    JOIN session_participants sp ON sp.user_id = f.user_id WHERE sp.session_id = ?1 \
    AND f.id = (SELECT g.id FROM feedback g WHERE g.user_id = f.user_id \
    AND g.restaurant_id = f.restaurant_id ORDER BY g.created_at DESC, g.id DESC LIMIT 1)";

pub const RESTAURANTS: &str = "SELECT id, name, address, lat, lon, walk_min, cuisine, price_level, opening_hours, \
    website, menu_url, source, active FROM restaurants WHERE active = 1 OR ?1 = 1 ORDER BY name";
pub const RESTAURANT: &str = "SELECT id, name, address, lat, lon, walk_min, cuisine, price_level, opening_hours, \
    website, menu_url, source, active FROM restaurants WHERE id = ?1";
pub const DIET_SUPPORT: &str = "SELECT restaurant_id, tag, supported FROM restaurant_diet_support";
pub const DIET_SUPPORT_FOR: &str =
    "SELECT restaurant_id, tag, supported FROM restaurant_diet_support WHERE restaurant_id = ?1";
pub const INSERT_RESTAURANT: &str = "INSERT INTO restaurants \
    (name, address, walk_min, cuisine, price_level, opening_hours, menu_url, source, created_at) \
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'manual', ?8) RETURNING id";

pub const OFFERS_FOR_DATE: &str = "SELECT id, restaurant_id, date, name, description, price_sek, diet_tags, source \
    FROM lunch_offers WHERE date = ?1";
pub const LAST_ACCEPTED: &str = "SELECT p.restaurant_id, MAX(s.date) AS date FROM session_picks p \
    JOIN lunch_sessions s ON s.id = p.session_id WHERE p.accepted = 1 AND s.date <= ?1 GROUP BY p.restaurant_id";
pub const SHOWN_IN_SESSION: &str = "SELECT DISTINCT restaurant_id AS id FROM session_picks WHERE session_id = ?1";

pub const INSERT_PICK: &str = "INSERT INTO session_picks (session_id, restaurant_id, kind, explanation, created_at) \
    VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id";
pub const LATEST_PICK: &str = "SELECT id, restaurant_id, kind, accepted, explanation FROM session_picks \
    WHERE session_id = ?1 ORDER BY id DESC LIMIT 1";
pub const PICK_RESTAURANT: &str = "SELECT restaurant_id AS id FROM session_picks WHERE session_id = ?1 AND id = ?2";
pub const CLEAR_ACCEPTED: &str = "UPDATE session_picks SET accepted = 0 WHERE session_id = ?1";
pub const PICK_IN_SESSION: &str = "SELECT id FROM session_picks WHERE session_id = ?1 AND id = ?2";
pub const ACCEPT_PICK: &str = "UPDATE session_picks SET accepted = CASE WHEN id = ?2 THEN 1 ELSE 0 END \
    WHERE session_id = ?1";

pub const UPSERT_FEEDBACK: &str = "INSERT INTO feedback (user_id, restaurant_id, session_id, rating, created_at) \
    VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (user_id, session_id, restaurant_id) DO UPDATE SET \
    rating = excluded.rating, created_at = excluded.created_at";
