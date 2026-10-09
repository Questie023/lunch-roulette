use lunch_core::clock::SystemClock;
use lunch_core::service::LunchService;
use lunch_core::store::Store;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde_json::json;
use worker::{Env, Error, Response, Result};

use crate::d1::D1Db;

pub type Service = LunchService<D1Db, SystemClock, StdRng>;

/// `true` when the env var is exactly `"1"`.
pub fn flag(env: &Env, name: &str) -> bool {
    env.var(name).map(|v| v.to_string() == "1").unwrap_or(false)
}

pub fn core_err(e: lunch_core::Error) -> Error {
    Error::RustError(e.to_string())
}

pub fn store(env: &Env) -> Result<Store<D1Db>> {
    Ok(Store::new(D1Db(env.d1("DB")?)))
}

pub fn service(env: &Env) -> Result<Service> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|e| Error::RustError(e.to_string()))?;
    Ok(LunchService::new(store(env)?, SystemClock, StdRng::from_seed(seed)))
}

/// Runs the portable store scenario against D1. Local dev only (`SELFTEST=1`).
pub async fn selftest(env: &Env) -> Result<Response> {
    let suffix = jiff::Timestamp::now().as_millisecond().to_string();
    match lunch_core::testkit::store_scenario(&store(env)?, &suffix).await {
        Ok(passed) => Response::from_json(&json!({"ok": true, "passed": passed})),
        Err(e) => Ok(Response::from_json(&json!({"ok": false, "error": e.to_string()}))?.with_status(500)),
    }
}
