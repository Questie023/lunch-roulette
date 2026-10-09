use lunch_core::db::{Db, Stmt, Value};
use lunch_sqlite::{MIGRATION_COUNT, SqliteDb};
use serde::Deserialize;

#[derive(Deserialize)]
struct KeyValue {
    key: String,
    value: String,
}

#[test]
fn fresh_db_is_migrated_and_migration_is_idempotent() {
    let db = SqliteDb::open_in_memory().unwrap();
    let version: i64 = db
        .conn()
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(version as usize, MIGRATION_COUNT);

    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    lunch_sqlite::migrate(&mut conn).unwrap();
    lunch_sqlite::migrate(&mut conn).unwrap();
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
    assert_eq!(version as usize, MIGRATION_COUNT);
}

#[test]
fn all_deserializes_rows_by_column_name() {
    let db = SqliteDb::open_in_memory().unwrap();
    let rows: Vec<KeyValue> = pollster::block_on(db.all(
        "SELECT key, value FROM settings WHERE key = ?1",
        vec![Value::from("timezone")],
    ))
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].key.as_str(), rows[0].value.as_str()),
        ("timezone", "Europe/Stockholm")
    );
}

#[test]
fn batch_is_atomic() {
    let db = SqliteDb::open_in_memory().unwrap();
    let result = pollster::block_on(db.batch(vec![
        Stmt::new(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            vec!["k1".into(), "v1".into()],
        ),
        // Duplicate primary key → the whole batch must roll back.
        Stmt::new(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            vec!["k1".into(), "v2".into()],
        ),
    ]));
    assert!(result.is_err());
    let rows: Vec<KeyValue> =
        pollster::block_on(db.all("SELECT key, value FROM settings WHERE key = 'k1'", vec![])).unwrap();
    assert!(rows.is_empty());
}

#[test]
fn foreign_keys_are_enforced() {
    let db = SqliteDb::open_in_memory().unwrap();
    let result = pollster::block_on(db.batch(vec![Stmt::new(
        "INSERT INTO session_participants (session_id, user_id, joined_at) VALUES (?1, ?2, ?3)",
        vec![Value::Int(999), "U_NOBODY".into(), Value::Int(0)],
    )]));
    assert!(result.is_err());
}

#[test]
fn migration_3_keeps_feedback_and_drops_vetoes() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    conn.execute_batch(include_str!("../../../migrations/0001_init.sql"))
        .unwrap();
    conn.pragma_update(None, "user_version", 2).unwrap();
    conn.execute_batch(
        "INSERT INTO users (id, display_name, created_at) VALUES ('U1', 'Anna', 0);
         INSERT INTO lunch_sessions (id, channel_id, date, created_by, created_at) VALUES (1, 'C1', '2026-10-06', 'U1', 0);
         INSERT INTO restaurants (name, cuisine, created_at) VALUES ('Alpha', 'thai', 0), ('Bravo', 'thai', 0), ('Charlie', 'thai', 0);
         INSERT INTO feedback (user_id, restaurant_id, session_id, rating, created_at)
           SELECT 'U1', id, 1, 'good', 0 FROM restaurants WHERE name = 'Alpha';
         INSERT INTO user_restaurant_prefs (user_id, restaurant_id, stance)
           SELECT 'U1', id, 'never' FROM restaurants WHERE name = 'Bravo';
         INSERT INTO user_restaurant_prefs (user_id, restaurant_id, stance)
           SELECT 'U1', id, 'favorite' FROM restaurants WHERE name = 'Charlie';",
    )
    .unwrap();

    lunch_sqlite::migrate(&mut conn).unwrap();

    let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap() };
    assert_eq!(count("SELECT COUNT(*) FROM feedback WHERE rating = 'good'"), 1);
    assert_eq!(
        count("SELECT COUNT(*) FROM user_restaurant_prefs WHERE stance = 'never'"),
        0
    );
    assert_eq!(
        count("SELECT COUNT(*) FROM user_restaurant_prefs WHERE stance = 'favorite'"),
        1
    );
    // A second place rated in the same session is a second row now.
    conn.execute(
        "INSERT INTO feedback (user_id, restaurant_id, session_id, rating, created_at)
         SELECT 'U1', id, 1, 'bad', 0 FROM restaurants WHERE name = 'Bravo'",
        [],
    )
    .unwrap();
    assert_eq!(count("SELECT COUNT(*) FROM feedback WHERE session_id = 1"), 2);
}
