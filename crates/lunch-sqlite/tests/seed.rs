use lunch_core::geo::walk_minutes;
use lunch_core::model::is_valid_url;
use lunch_core::rules::opening_hours::parse_opening_hours;
use lunch_core::store::Store;
use lunch_sqlite::SqliteDb;

fn with_example_seed() -> Store<SqliteDb> {
    let db = SqliteDb::open_in_memory().unwrap();
    db.conn()
        .execute_batch(include_str!("../../../seeds/example.sql"))
        .unwrap();
    Store::new(db)
}

#[test]
fn a_fresh_database_has_no_restaurants() {
    let store = Store::new(SqliteDb::open_in_memory().unwrap());
    assert!(pollster::block_on(store.restaurants(true)).unwrap().is_empty());
}

#[test]
fn example_seed_restaurants_are_valid() {
    let all = pollster::block_on(with_example_seed().restaurants(true)).unwrap();
    assert_eq!(all.len(), 3);
    for r in &all {
        let url = r
            .menu_url
            .as_deref()
            .unwrap_or_else(|| panic!("{} has no menu link", r.name));
        assert!(is_valid_url(url), "{}: {url}", r.name);
        let hours = r
            .opening_hours
            .as_deref()
            .unwrap_or_else(|| panic!("{} has no hours", r.name));
        parse_opening_hours(hours).unwrap_or_else(|e| panic!("{}: {e}", r.name));
    }
}

#[test]
fn example_seed_places_are_within_walking_range() {
    let store = with_example_seed();
    let settings = pollster::block_on(store.settings()).unwrap();
    assert_eq!((settings.office_lat, settings.office_lon), (60.0, 10.0));
    for r in pollster::block_on(store.restaurants(false)).unwrap() {
        let walk = walk_minutes(&r, &settings).unwrap();
        assert!(walk <= settings.default_max_walk_min, "{} is {walk} min away", r.name);
    }
}
