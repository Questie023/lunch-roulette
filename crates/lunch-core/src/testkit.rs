//! Portable store scenario. Runs natively against SQLite (tests) and inside the worker against D1
//! (`POST /__selftest`), so both `Db` implementations are held to the same behaviour.

use jiff::Timestamp;
use jiff::civil::date;

use crate::db::{Db, Stmt, Value};
use crate::model::{
    DietTag, Explanation, NewFeedback, NewPick, PickKind, Rating, Reason, RestaurantInput, SessionStatus, User,
    UserPrefs, Warning,
};
use crate::rows::IdRow;
use crate::store::Store;
use crate::{Error, Result};

macro_rules! check {
    ($passed:ident, $cond:expr, $name:literal) => {
        if !$cond {
            return Err(Error::Invalid(concat!("check failed: ", $name).into()));
        }
        $passed.push($name);
    };
}

/// Exercises every `Store` method, then deletes what it created. `suffix` keeps runs independent.
pub async fn store_scenario<D: Db>(store: &Store<D>, suffix: &str) -> Result<Vec<&'static str>> {
    let result = run(store, suffix).await;
    cleanup(store, suffix).await?;
    result
}

async fn run<D: Db>(store: &Store<D>, suffix: &str) -> Result<Vec<&'static str>> {
    let mut passed = Vec::new();
    let now = Timestamp::from_second(1_791_100_000).map_err(|e| Error::Invalid(e.to_string()))?;
    let today = date(2026, 10, 6);
    let user = User {
        id: format!("U_TEST_{suffix}"),
        display_name: "Test & <User>".into(),
    };
    let channel = format!("C_TEST_{suffix}");

    let settings = store.settings().await?;
    check!(passed, settings.timezone == "Europe/Stockholm", "settings load");

    store.upsert_user(&user, now).await?;
    store.upsert_user(&user, now).await?;
    let input = RestaurantInput {
        name: format!("Test Bistro {suffix}"),
        address: None,
        walk_min: Some(4),
        cuisine: "Thai".into(),
        price_level: 2,
        opening_hours: Some("Mo-Fr 11:00-14:00".into()),
        menu_url: Some("https://example.com/menu".into()),
    };
    let restaurant_id = store.save_restaurant(&input, now).await?;
    let saved = store.restaurant(restaurant_id).await?;
    check!(
        passed,
        saved.cuisine == "thai" && saved.walk_min == Some(4) && saved.active,
        "restaurant round trip"
    );

    let prefs = UserPrefs {
        max_walk_min: Some(12),
        price_pref: Some(1),
        constraints: [DietTag::Vegetarian].into(),
        cuisine_weights: [("thai".to_string(), 2)].into(),
        favorites: [restaurant_id].into(),
        ratings: Default::default(),
    };
    store.replace_prefs(&user.id, &prefs).await?;
    check!(passed, store.prefs(&user.id).await? == prefs, "prefs round trip");

    let session = store.get_or_create_session(&channel, today, &user.id, now).await?;
    let again = store.get_or_create_session(&channel, today, &user.id, now).await?;
    check!(
        passed,
        session.id == again.id && session.date == today,
        "one session per channel and day"
    );

    store.set_participation(session.id, &user.id, true, now).await?;
    store.set_participation(session.id, &user.id, true, now).await?;
    let view = store.session_view(session.id).await?;
    check!(passed, view.participants == vec![user.clone()], "join is idempotent");

    let ctx = store.pick_context(session.id, today).await?;
    check!(
        passed,
        ctx.participants.len() == 1
            && ctx.participants[0].1 == prefs
            && ctx.restaurants.iter().any(|r| r.id == restaurant_id),
        "pick context"
    );

    let explanation = Explanation {
        walk_min: Some(4),
        reasons: vec![Reason::NotRecentlyVisited],
        warnings: vec![Warning::DietInfoUnknown],
    };
    let new_pick = NewPick {
        session_id: session.id,
        restaurant_id,
        kind: PickKind::Roll,
        explanation: explanation.clone(),
        created_at: now,
    };
    let pick_id = store.insert_pick(&new_pick).await?;
    let latest = store
        .session_view(session.id)
        .await?
        .latest_pick
        .ok_or_else(|| Error::Invalid("check failed: latest pick missing".into()))?;
    check!(
        passed,
        latest.pick_id == pick_id && latest.explanation == explanation && !latest.accepted,
        "pick is recorded"
    );

    store.accept_pick(session.id, pick_id).await?;
    let view = store.session_view(session.id).await?;
    check!(
        passed,
        view.session.status == SessionStatus::Decided && view.latest_pick.is_some_and(|p| p.accepted),
        "accept"
    );
    check!(
        passed,
        store.accept_pick(session.id, pick_id + 1_000_000).await.is_err(),
        "accept rejects a foreign pick"
    );

    let second = store.insert_pick(&new_pick).await?;
    let view = store.session_view(session.id).await?;
    check!(
        passed,
        view.session.status == SessionStatus::Open
            && view.latest_pick.is_some_and(|p| p.pick_id == second && !p.accepted),
        "a new pick un-decides"
    );
    check!(
        passed,
        store.accept_pick(session.id, pick_id).await.is_err(),
        "accept rejects a replaced pick"
    );
    store.accept_pick(session.id, second).await?;

    let ctx = store.pick_context(session.id, today).await?;
    check!(
        passed,
        ctx.already_shown.contains(&restaurant_id) && ctx.last_accepted.get(&restaurant_id) == Some(&today),
        "history feeds the pipeline"
    );

    let feedback = NewFeedback {
        session_id: session.id,
        user_id: user.id.clone(),
        restaurant_id,
        rating: Rating::Bad,
        created_at: now,
    };
    store.add_feedback(&feedback).await?;
    let ctx = store.pick_context(session.id, today).await?;
    check!(
        passed,
        ctx.participants[0].1.ratings.get(&restaurant_id) == Some(&Rating::Bad),
        "ratings feed participant opinions"
    );

    let other = store
        .save_restaurant(
            &RestaurantInput {
                name: format!("Test Bistro 2 {suffix}"),
                ..input.clone()
            },
            now,
        )
        .await?;
    store
        .add_feedback(&NewFeedback {
            restaurant_id: other,
            rating: Rating::Good,
            ..feedback.clone()
        })
        .await?;
    let rows = store
        .db()
        .all::<IdRow>(
            "SELECT COUNT(*) AS id FROM feedback WHERE user_id = ?1 AND session_id = ?2",
            vec![user.id.as_str().into(), Value::Int(session.id)],
        )
        .await?;
    check!(
        passed,
        rows.first().map(|r| r.id) == Some(2),
        "rating another place keeps the first rating"
    );

    store
        .add_feedback(&NewFeedback {
            rating: Rating::Ok,
            ..feedback.clone()
        })
        .await?;
    check!(
        passed,
        store.prefs(&user.id).await?.ratings.get(&restaurant_id) == Some(&Rating::Ok),
        "latest rating wins"
    );

    store.set_message_ts(session.id, "1700000000.000100").await?;
    store.set_participation(session.id, &user.id, false, now).await?;
    let view = store.session_view(session.id).await?;
    check!(
        passed,
        view.participants.is_empty() && view.session.message_ts.as_deref() == Some("1700000000.000100"),
        "leave and message ts"
    );

    check!(
        passed,
        store.pick_context(session.id, today).await?.participants.is_empty(),
        "non-participants are ignored"
    );

    Ok(passed)
}

async fn cleanup<D: Db>(store: &Store<D>, suffix: &str) -> Result<()> {
    let user = Value::from(format!("U_TEST_{suffix}"));
    store
        .db()
        .batch(vec![
            Stmt::new("DELETE FROM feedback WHERE user_id = ?1", vec![user.clone()]),
            Stmt::new(
                "DELETE FROM lunch_sessions WHERE channel_id = ?1",
                vec![format!("C_TEST_{suffix}").into()],
            ),
            Stmt::new("DELETE FROM users WHERE id = ?1", vec![user]),
            Stmt::new(
                "DELETE FROM restaurants WHERE name = ?1",
                vec![format!("Test Bistro {suffix}").into()],
            ),
            Stmt::new(
                "DELETE FROM restaurants WHERE name = ?1",
                vec![format!("Test Bistro 2 {suffix}").into()],
            ),
        ])
        .await
}
