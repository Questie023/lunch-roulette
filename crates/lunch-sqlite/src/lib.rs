//! SQLite implementation of [`lunch_core::db::Db`] plus the shared migrations.

use std::path::Path;

use lunch_core::db::{Db, Stmt, Value};
use lunch_core::{Error, Result};
use rusqlite::Connection;
use rusqlite::types::{Value as SqlValue, ValueRef};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value as Json};

const MIGRATIONS: &[&str] = &[
    include_str!("../../../migrations/0001_init.sql"),
    include_str!("../../../migrations/0002_seed.sql"),
    include_str!("../../../migrations/0003_fair_feedback.sql"),
];

pub const MIGRATION_COUNT: usize = MIGRATIONS.len();

/// Applies migrations newer than `PRAGMA user_version`, each in its own transaction.
pub fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "foreign_keys", true)?;
    let current: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

pub struct SqliteDb {
    conn: Connection,
}

impl SqliteDb {
    pub fn open(path: &Path) -> Result<Self> {
        Self::init(Connection::open(path).map_err(storage)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory().map_err(storage)?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        migrate(&mut conn).map_err(storage)?;
        Ok(SqliteDb { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

fn storage(e: impl std::fmt::Display) -> Error {
    Error::Storage(e.to_string())
}

fn to_sql(v: &Value) -> SqlValue {
    match v {
        Value::Null => SqlValue::Null,
        Value::Int(i) => SqlValue::Integer(*i),
        Value::Real(f) => SqlValue::Real(*f),
        Value::Text(s) => SqlValue::Text(s.clone()),
    }
}

fn to_json(v: ValueRef<'_>) -> Result<Json> {
    Ok(match v {
        ValueRef::Null => Json::Null,
        ValueRef::Integer(i) => Json::from(i),
        ValueRef::Real(f) => serde_json::Number::from_f64(f).map_or(Json::Null, Json::Number),
        ValueRef::Text(t) => Json::String(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(_) => return Err(Error::Storage("unexpected BLOB column".into())),
    })
}

impl Db for SqliteDb {
    async fn all<T: DeserializeOwned>(&self, sql: &str, params: Vec<Value>) -> Result<Vec<T>> {
        let mut stmt = self.conn.prepare_cached(sql).map_err(storage)?;
        let names: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
        let mut rows = stmt
            .query(rusqlite::params_from_iter(params.iter().map(to_sql)))
            .map_err(storage)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(storage)? {
            let mut object = Map::new();
            for (i, name) in names.iter().enumerate() {
                object.insert(name.clone(), to_json(row.get_ref(i).map_err(storage)?)?);
            }
            out.push(serde_json::from_value(Json::Object(object)).map_err(storage)?);
        }
        Ok(out)
    }

    async fn batch(&self, stmts: Vec<Stmt>) -> Result<()> {
        let tx = self.conn.unchecked_transaction().map_err(storage)?;
        for s in &stmts {
            tx.execute(s.sql, rusqlite::params_from_iter(s.params.iter().map(to_sql)))
                .map_err(storage)?;
        }
        tx.commit().map_err(storage)
    }
}
