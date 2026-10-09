//! The only storage seam: SQLite (native) and D1 (worker) implement [`Db`].

use serde::de::DeserializeOwned;

use crate::Result;

/// A bind parameter that maps cleanly onto both SQLite and D1.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::Int(i64::from(v))
    }
}

impl From<u8> for Value {
    fn from(v: u8) -> Self {
        Value::Int(i64::from(v))
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Int(i64::from(v))
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Real(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        v.map_or(Value::Null, Into::into)
    }
}

/// One write statement inside an atomic [`Db::batch`].
#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub sql: &'static str,
    pub params: Vec<Value>,
}

impl Stmt {
    pub fn new(sql: &'static str, params: Vec<Value>) -> Self {
        Stmt { sql, params }
    }
}

#[allow(async_fn_in_trait)]
pub trait Db {
    /// Runs a query (including `INSERT … RETURNING`) and deserializes each row by column name.
    async fn all<T: DeserializeOwned>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>>;

    /// Runs write statements atomically: all succeed or none are applied.
    async fn batch(&self, stmts: Vec<Stmt>) -> Result<()>;
}
