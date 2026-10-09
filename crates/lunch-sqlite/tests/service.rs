use std::future::Future;

use jiff::Timestamp;
use jiff::civil::date;
use lunch_core::Error;
use lunch_core::clock::FixedClock;
use lunch_core::db::{Db, Stmt};
use lunch_core::model::*;
use lunch_core::service::LunchService;
use lunch_core::store::Store;
use lunch_sqlite::SqliteDb;
use rand::SeedableRng;
use rand::rngs::StdRng;

type Service = LunchService<SqliteDb, FixedClock, StdRng>;

fn run<F: Future>(f: F) -> F::Output {
    pollster::block_on(f)
}

fn at(iso: &str) -> FixedClock {
    FixedClock(iso.parse::<Timestamp>().unwrap())
}

/// Tuesday 2026-10-06 12:00 in Stockholm.
const LUNCHTIME: &str = "2026-10-06T10:00:00Z";

/// A fresh database with no restaurants, so tests don't depend on seed data.
fn service(clock: FixedClock) -> Service {
    let db = SqliteDb::open_in_memory().unwrap();
    run(db.batch(vec![
        Stmt::new("DELETE FROM restaurant_diet_support", vec![]),
        Stmt::new("DELETE FROM restaurants", vec![]),
    ]))
    .unwrap();
    LunchService::new(Store::new(db), clock, StdRng::seed_from_u64(9))
}

fn anna() -> User {
    User {
        id: "U_ANNA".into(),
        display_name: "Anna".into(),
    }
}

fn max() -> User {
    User {
        id: "U_MAX".into(),
        display_name: "Max".into(),
    }
}

fn input(name: &str) -> RestaurantInput {
    RestaurantInput {
        name: name.into(),
        address: None,
        walk_min: Some(5),
        cuisine: "thai".into(),
        price_level: 2,
        opening_hours: Some("Mo-Fr 11:00-14:00".into()),
        menu_url: None,
    }
}

fn picked_name(r: PickResponse) -> String {
    match r {
        PickResponse::Picked(v) => v.latest_pick.unwrap().restaurant.name,
        other => panic!("expected a pick, got {other:?}"),
    }
}

fn empty_stage(r: PickResponse) -> FilterStage {
    match r {
        PickResponse::Empty { stage, .. } => stage,
        other => panic!("expected empty, got {other:?}"),
    }
}

#[test]
fn open_session_auto_joins_and_is_shared_per_channel_and_day() {
    let mut s = service(at(LUNCHTIME));
    let a = run(s.open_session("C1", &anna())).unwrap();
    let b = run(s.open_session("C1", &max())).unwrap();
    assert_eq!(a.session.id, b.session.id);
    let names: Vec<_> = b.participants.iter().map(|u| u.display_name.as_str()).collect();
    assert_eq!(names, ["Anna", "Max"]);
    let other = run(s.open_session("C2", &anna())).unwrap();
    assert_ne!(other.session.id, a.session.id);
}

#[test]
fn today_uses_office_timezone() {
    // 22:30 UTC on Oct 5 is already 00:30 on Oct 6 in Stockholm (CEST, UTC+2).
    let mut s = service(at("2026-10-05T22:30:00Z"));
    let v = run(s.open_session("C1", &anna())).unwrap();
    assert_eq!(v.session.date, date(2026, 10, 6));
}

#[test]
fn join_and_leave_are_idempotent() {
    let mut s = service(at(LUNCHTIME));
    let id = run(s.open_session("C1", &anna())).unwrap().session.id;
    run(s.join(id, &max())).unwrap();
    assert_eq!(run(s.join(id, &max())).unwrap().participants.len(), 2);
    run(s.leave(id, &max().id)).unwrap();
    assert_eq!(run(s.leave(id, &max().id)).unwrap().participants.len(), 1);
}

#[test]
fn pick_needs_participants() {
    let mut s = service(at(LUNCHTIME));
    run(s.add_restaurant(input("A"))).unwrap();
    let id = run(s.open_session("C1", &anna())).unwrap().session.id;
    run(s.leave(id, &anna().id)).unwrap();
    assert_eq!(run(s.pick(id, PickOptions::default())), Err(Error::NoParticipants));
}

#[test]
fn rerolls_never_repeat_within_a_session() {
    let mut s = service(at(LUNCHTIME));
    run(s.add_restaurant(input("A"))).unwrap();
    run(s.add_restaurant(input("B"))).unwrap();
    let id = run(s.open_session("C1", &anna())).unwrap().session.id;
    let first = picked_name(run(s.pick(id, PickOptions::default())).unwrap());
    let second = picked_name(run(s.pick(id, PickOptions::default())).unwrap());
    assert_ne!(first, second);
    assert_eq!(
        empty_stage(run(s.pick(id, PickOptions::default())).unwrap()),
        FilterStage::AllShown
    );
}

#[test]
fn manual_pick_then_accept_decides_the_session() {
    let mut s = service(at(LUNCHTIME));
    let a = run(s.add_restaurant(input("A"))).unwrap();
    let id = run(s.open_session("C1", &anna())).unwrap().session.id;
    let pick = run(s.manual_pick(id, a)).unwrap().latest_pick.unwrap();
    assert_eq!(pick.kind, PickKind::Manual);
    assert_eq!(pick.explanation.reasons, vec![Reason::ManualChoice]);
    let v = run(s.accept(id, pick.pick_id)).unwrap();
    assert_eq!(v.session.status, SessionStatus::Decided);
    assert!(v.latest_pick.unwrap().accepted);
}

#[test]
fn feedback_rejects_an_unknown_pick() {
    let mut s = service(at(LUNCHTIME));
    let id = run(s.open_session("C1", &anna())).unwrap().session.id;
    assert!(matches!(
        run(s.feedback(id, 999_999, &anna(), Rating::Good)),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn feedback_keeps_the_display_name() {
    let mut s = service(at(LUNCHTIME));
    let a = run(s.add_restaurant(input("A"))).unwrap();
    let full = User {
        id: "U_ANNA".into(),
        display_name: "Anna Svensson".into(),
    };
    let id = run(s.open_session("C1", &full)).unwrap().session.id;
    let pick = run(s.manual_pick(id, a)).unwrap().latest_pick.unwrap().pick_id;
    // Slack button payloads only carry the handle.
    let handle = User {
        id: "U_ANNA".into(),
        display_name: "anna.svensson".into(),
    };
    run(s.feedback(id, pick, &handle, Rating::Good)).unwrap();
    let names: Vec<_> = run(s.view(id))
        .unwrap()
        .participants
        .into_iter()
        .map(|u| u.display_name)
        .collect();
    assert_eq!(names, ["Anna Svensson"]);
}

#[test]
fn feedback_rates_the_pick_that_was_shown() {
    let mut s = service(at(LUNCHTIME));
    let a = run(s.add_restaurant(input("A"))).unwrap();
    let b = run(s.add_restaurant(input("B"))).unwrap();
    let id = run(s.open_session("C1", &anna())).unwrap().session.id;
    let shown = run(s.manual_pick(id, a)).unwrap().latest_pick.unwrap().pick_id;
    run(s.manual_pick(id, b)).unwrap(); // someone hit "Try another" just before the 👎 landed
    assert_eq!(run(s.feedback(id, shown, &anna(), Rating::Bad)).unwrap(), a);
    assert_eq!(
        run(s.store().prefs(&anna().id)).unwrap().ratings,
        [(a, Rating::Bad)].into()
    );
}

#[test]
fn accept_rejects_a_replaced_pick() {
    let mut s = service(at(LUNCHTIME));
    let a = run(s.add_restaurant(input("A"))).unwrap();
    let b = run(s.add_restaurant(input("B"))).unwrap();
    let id = run(s.open_session("C1", &anna())).unwrap().session.id;
    let shown = run(s.manual_pick(id, a)).unwrap().latest_pick.unwrap().pick_id;
    run(s.manual_pick(id, b)).unwrap();
    assert!(matches!(run(s.accept(id, shown)), Err(Error::Invalid(_))));
    assert_eq!(run(s.view(id)).unwrap().session.status, SessionStatus::Open);
}

#[test]
fn a_new_pick_undecides_the_session() {
    let mut s = service(at(LUNCHTIME));
    let a = run(s.add_restaurant(input("A"))).unwrap();
    let b = run(s.add_restaurant(input("B"))).unwrap();
    let id = run(s.open_session("C1", &anna())).unwrap().session.id;
    let first = run(s.manual_pick(id, a)).unwrap().latest_pick.unwrap().pick_id;
    run(s.accept(id, first)).unwrap();
    let v = run(s.manual_pick(id, b)).unwrap();
    assert_eq!(v.session.status, SessionStatus::Open);
    assert!(!v.latest_pick.unwrap().accepted);
    let ctx = run(s.store().pick_context(id, date(2026, 10, 6))).unwrap();
    assert!(ctx.last_accepted.is_empty(), "{:?}", ctx.last_accepted);
}

#[test]
fn add_restaurant_validates() {
    let mut s = service(at(LUNCHTIME));
    run(s.add_restaurant(input("Curry House"))).unwrap();
    assert!(matches!(
        run(s.add_restaurant(input("curry house"))),
        Err(Error::Invalid(_))
    ));
    let bad_hours = RestaurantInput {
        opening_hours: Some("whenever".into()),
        ..input("New")
    };
    assert!(matches!(run(s.add_restaurant(bad_hours)), Err(Error::Invalid(_))));
    assert_eq!(run(s.restaurants()).unwrap().len(), 1);
}

#[test]
fn a_thumbs_down_no_longer_removes_a_place() {
    let mut s = service(at(LUNCHTIME));
    let a = run(s.add_restaurant(input("A"))).unwrap();
    let id = run(s.open_session("C1", &anna())).unwrap().session.id;
    let pick = run(s.manual_pick(id, a)).unwrap().latest_pick.unwrap().pick_id;
    run(s.feedback(id, pick, &anna(), Rating::Bad)).unwrap();

    let mut tomorrow = LunchService::new(s.into_store(), at("2026-10-07T10:00:00Z"), StdRng::seed_from_u64(1));
    let id2 = run(tomorrow.open_session("C1", &anna())).unwrap().session.id;
    assert_eq!(
        picked_name(run(tomorrow.pick(id2, PickOptions::default())).unwrap()),
        "A"
    );
}

#[test]
fn the_latest_rating_wins() {
    let mut s = service(at(LUNCHTIME));
    let a = run(s.add_restaurant(input("A"))).unwrap();
    let id = run(s.open_session("C1", &anna())).unwrap().session.id;
    let pick = run(s.manual_pick(id, a)).unwrap().latest_pick.unwrap().pick_id;
    run(s.feedback(id, pick, &anna(), Rating::Bad)).unwrap();
    run(s.feedback(id, pick, &anna(), Rating::Ok)).unwrap();
    assert_eq!(
        run(s.store().prefs(&anna().id)).unwrap().ratings,
        [(a, Rating::Ok)].into()
    );

    // A week later the group goes again and Anna liked it.
    let mut later = LunchService::new(s.into_store(), at("2026-10-13T10:00:00Z"), StdRng::seed_from_u64(3));
    let id2 = run(later.open_session("C1", &anna())).unwrap().session.id;
    let pick2 = run(later.manual_pick(id2, a)).unwrap().latest_pick.unwrap().pick_id;
    run(later.feedback(id2, pick2, &anna(), Rating::Good)).unwrap();
    assert_eq!(
        run(later.store().prefs(&anna().id)).unwrap().ratings,
        [(a, Rating::Good)].into()
    );
}

#[test]
fn latest_rating_wins_within_the_same_second() {
    let mut s = service(at(LUNCHTIME));
    let a = run(s.add_restaurant(input("A"))).unwrap();
    let c1 = run(s.open_session("C1", &anna())).unwrap().session.id;
    let c2 = run(s.open_session("C2", &anna())).unwrap().session.id;
    let p1 = run(s.manual_pick(c1, a)).unwrap().latest_pick.unwrap().pick_id;
    let p2 = run(s.manual_pick(c2, a)).unwrap().latest_pick.unwrap().pick_id;
    run(s.feedback(c1, p1, &anna(), Rating::Bad)).unwrap();
    run(s.feedback(c2, p2, &anna(), Rating::Good)).unwrap(); // same clock second
    assert_eq!(
        run(s.store().prefs(&anna().id)).unwrap().ratings,
        [(a, Rating::Good)].into()
    );
}

#[test]
fn only_participants_ratings_reach_the_pick() {
    let mut s = service(at(LUNCHTIME));
    let a = run(s.add_restaurant(input("A"))).unwrap();
    let id = run(s.open_session("C1", &anna())).unwrap().session.id;
    let pick = run(s.manual_pick(id, a)).unwrap().latest_pick.unwrap().pick_id;
    run(s.feedback(id, pick, &max(), Rating::Bad)).unwrap(); // Max rated but never joined
    let ctx = run(s.store().pick_context(id, date(2026, 10, 6))).unwrap();
    assert_eq!(ctx.participants.len(), 1);
    assert!(ctx.participants[0].1.ratings.is_empty());
}
