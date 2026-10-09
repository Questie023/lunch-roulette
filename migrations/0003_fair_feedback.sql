-- Fair roulette (2026-10-09): one rating per person per place per session, and no more vetoes.
-- SQLite cannot change a UNIQUE constraint in place, so the table is rebuilt. Nothing references
-- `feedback`, so dropping it is safe with foreign keys on.

CREATE TABLE feedback_v2 (
  id INTEGER PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  restaurant_id INTEGER NOT NULL REFERENCES restaurants(id),
  session_id INTEGER REFERENCES lunch_sessions(id) ON DELETE SET NULL,
  rating TEXT NOT NULL CHECK (rating IN ('good', 'ok', 'bad')),
  created_at INTEGER NOT NULL,
  UNIQUE (user_id, session_id, restaurant_id)
);

INSERT INTO feedback_v2 (id, user_id, restaurant_id, session_id, rating, created_at)
  SELECT id, user_id, restaurant_id, session_id, rating, created_at FROM feedback;

DROP TABLE feedback;
ALTER TABLE feedback_v2 RENAME TO feedback;
CREATE INDEX feedback_user_restaurant ON feedback (user_id, restaurant_id, created_at);

-- 👎 is a soft dislike now; vetoes are gone (production had none left on 2026-10-09).
DELETE FROM user_restaurant_prefs WHERE stance = 'never';
