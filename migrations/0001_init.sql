-- Shared by D1 (wrangler d1 migrations apply) and rusqlite (lunch-sqlite::migrate).
-- No BEGIN/PRAGMA here: D1 rejects them.

CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

INSERT INTO settings (key, value) VALUES
  ('office_lat', '59.33258'),
  ('office_lon', '18.06490'),
  ('timezone', 'Europe/Stockholm'),
  ('lunch_start', '11:30'),
  ('lunch_end', '13:30'),
  ('default_max_walk_min', '10'),
  ('currency', 'SEK');

CREATE TABLE users (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  max_walk_min INTEGER CHECK (max_walk_min IS NULL OR max_walk_min BETWEEN 1 AND 60),
  price_pref INTEGER CHECK (price_pref IS NULL OR price_pref BETWEEN 1 AND 3),
  created_at INTEGER NOT NULL
);

CREATE TABLE user_constraints (
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  tag TEXT NOT NULL,
  PRIMARY KEY (user_id, tag)
);

CREATE TABLE user_cuisine_prefs (
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  cuisine TEXT NOT NULL,
  weight INTEGER NOT NULL CHECK (weight BETWEEN -2 AND 2),
  PRIMARY KEY (user_id, cuisine)
);

CREATE TABLE restaurants (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  address TEXT,
  lat REAL,
  lon REAL,
  walk_min INTEGER CHECK (walk_min IS NULL OR walk_min BETWEEN 1 AND 60),
  cuisine TEXT NOT NULL,
  price_level INTEGER NOT NULL DEFAULT 2 CHECK (price_level BETWEEN 1 AND 3),
  opening_hours TEXT,
  website TEXT,
  menu_url TEXT,
  source TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'canvas', 'osm')),
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  created_at INTEGER NOT NULL
);

CREATE TABLE user_restaurant_prefs (
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  restaurant_id INTEGER NOT NULL REFERENCES restaurants(id) ON DELETE CASCADE,
  stance TEXT NOT NULL CHECK (stance IN ('favorite', 'never')),
  PRIMARY KEY (user_id, restaurant_id)
);

CREATE TABLE restaurant_diet_support (
  restaurant_id INTEGER NOT NULL REFERENCES restaurants(id) ON DELETE CASCADE,
  tag TEXT NOT NULL,
  supported INTEGER NOT NULL CHECK (supported IN (0, 1)),
  PRIMARY KEY (restaurant_id, tag)
);

CREATE TABLE lunch_offers (
  id INTEGER PRIMARY KEY,
  restaurant_id INTEGER NOT NULL REFERENCES restaurants(id) ON DELETE CASCADE,
  date TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  price_sek INTEGER,
  diet_tags TEXT NOT NULL DEFAULT '[]',
  source TEXT NOT NULL,
  fetched_at INTEGER NOT NULL
);
CREATE INDEX lunch_offers_date ON lunch_offers (date);

CREATE TABLE lunch_sessions (
  id INTEGER PRIMARY KEY,
  channel_id TEXT NOT NULL,
  date TEXT NOT NULL,
  message_ts TEXT,
  status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'decided')),
  created_by TEXT NOT NULL REFERENCES users(id),
  created_at INTEGER NOT NULL,
  UNIQUE (channel_id, date)
);

CREATE TABLE session_participants (
  session_id INTEGER NOT NULL REFERENCES lunch_sessions(id) ON DELETE CASCADE,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  joined_at INTEGER NOT NULL,
  PRIMARY KEY (session_id, user_id)
);

CREATE TABLE session_picks (
  id INTEGER PRIMARY KEY,
  session_id INTEGER NOT NULL REFERENCES lunch_sessions(id) ON DELETE CASCADE,
  restaurant_id INTEGER NOT NULL REFERENCES restaurants(id),
  kind TEXT NOT NULL CHECK (kind IN ('roll', 'manual')),
  accepted INTEGER NOT NULL DEFAULT 0 CHECK (accepted IN (0, 1)),
  explanation TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE INDEX session_picks_session ON session_picks (session_id);

CREATE TABLE feedback (
  id INTEGER PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  restaurant_id INTEGER NOT NULL REFERENCES restaurants(id),
  session_id INTEGER REFERENCES lunch_sessions(id) ON DELETE SET NULL,
  rating TEXT NOT NULL CHECK (rating IN ('good', 'ok', 'bad')),
  created_at INTEGER NOT NULL,
  UNIQUE (user_id, session_id)
);
