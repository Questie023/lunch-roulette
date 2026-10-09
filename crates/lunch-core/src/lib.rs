//! Lunch Roulette domain: model, pick pipeline, storage-agnostic store and business flows.

pub mod clock;
pub mod db;
pub mod error;
pub mod explain;
pub mod geo;
pub mod model;
pub mod pipeline;
pub mod rows;
pub mod rules;
pub mod service;
pub mod sql;
pub mod store;
#[cfg(feature = "testkit")]
pub mod testkit;

pub use error::{Error, Result};
