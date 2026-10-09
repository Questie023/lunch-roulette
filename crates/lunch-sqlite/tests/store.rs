use lunch_core::store::Store;
use lunch_core::testkit::store_scenario;
use lunch_sqlite::SqliteDb;

#[test]
fn sqlite_passes_the_portable_store_scenario() {
    let store = Store::new(SqliteDb::open_in_memory().unwrap());
    let passed = pollster::block_on(store_scenario(&store, "sqlite")).unwrap();
    assert_eq!(passed.len(), 17, "{passed:?}");
}

#[test]
fn scenario_cleans_up_after_itself() {
    let store = Store::new(SqliteDb::open_in_memory().unwrap());
    pollster::block_on(store_scenario(&store, "a")).unwrap();
    let users: i64 = store
        .db()
        .conn()
        .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
        .unwrap();
    let sessions: i64 = store
        .db()
        .conn()
        .query_row("SELECT COUNT(*) FROM lunch_sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!((users, sessions), (0, 0));
}
