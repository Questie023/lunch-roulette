-- Example data for local development and demos: a made-up office and three made-up restaurants.
-- Apply it with:
--   npx wrangler d1 execute lunch-roulette --local --file ../../seeds/example.sql   (from crates/lunch-worker)
-- Use your own copy with real places for a real team; keep it out of git (seeds/local/ is ignored).

INSERT OR REPLACE INTO settings (key, value) VALUES
  ('office_lat', '60.00000'),
  ('office_lon', '10.00000'),
  ('default_max_walk_min', '10');

INSERT INTO restaurants (name, address, lat, lon, cuisine, price_level, opening_hours, website, menu_url, source, created_at) VALUES
  ('Noodle Bar', '1 Example Street', 60.00130, 10.00000, 'thai', 2, 'Mo-Fr 11:00-14:00',
   'https://noodle-bar.example', 'https://noodle-bar.example/lunch', 'manual', 1759622400),
  ('Curry House', '2 Example Street', 60.00000, 10.00360, 'indian', 2, 'Mo-Fr 11:00-15:00; Sa 12:00-15:00',
   'https://curry-house.example', 'https://curry-house.example/menu', 'manual', 1759622400),
  ('Green Kitchen', '3 Example Street', 59.99900, 9.99800, 'vegetarian', 1, 'Mo-Sa 10:30-14:30',
   'https://green-kitchen.example', 'https://green-kitchen.example/today', 'manual', 1759622400);

-- Only facts a restaurant states explicitly. A missing row means "unknown", never "safe".
INSERT INTO restaurant_diet_support (restaurant_id, tag, supported)
  SELECT id, 'vegetarian', 1 FROM restaurants WHERE name IN ('Noodle Bar', 'Curry House', 'Green Kitchen');
INSERT INTO restaurant_diet_support (restaurant_id, tag, supported)
  SELECT id, 'vegan', 1 FROM restaurants WHERE name = 'Green Kitchen';
