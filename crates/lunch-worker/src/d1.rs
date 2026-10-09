//! D1 implementation of [`Db`]. D1 takes JS numbers (f64) for integers and returns 0/1 for booleans.

use lunch_core::db::{Db, Stmt, Value};
use lunch_core::{Error, Result};
use serde::de::DeserializeOwned;
use worker::D1Database;
use worker::wasm_bindgen::JsValue;

pub struct D1Db(pub D1Database);

fn storage(e: impl std::fmt::Display) -> Error {
    Error::Storage(e.to_string())
}

/// Our integers (ids, unix seconds) stay far below 2^53, so f64 is exact.
fn to_js(params: &[Value]) -> Vec<JsValue> {
    params
        .iter()
        .map(|v| match v {
            Value::Null => JsValue::NULL,
            Value::Int(i) => JsValue::from_f64(*i as f64),
            Value::Real(f) => JsValue::from_f64(*f),
            Value::Text(s) => JsValue::from_str(s),
        })
        .collect()
}

impl Db for D1Db {
    async fn all<T: DeserializeOwned>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>> {
        let stmt = self.0.prepare(sql).bind(&to_js(&params)).map_err(storage)?;
        stmt.all().await.map_err(storage)?.results::<T>().map_err(storage)
    }

    async fn batch(&self, stmts: Vec<Stmt>) -> Result<()> {
        if stmts.is_empty() {
            return Ok(());
        }
        let prepared = stmts
            .iter()
            .map(|s| self.0.prepare(s.sql).bind(&to_js(&s.params)))
            .collect::<worker::Result<Vec<_>>>()
            .map_err(storage)?;
        for result in self.0.batch(prepared).await.map_err(storage)? {
            if let Some(e) = result.error() {
                return Err(Error::Storage(e));
            }
        }
        Ok(())
    }
}
