//! Shared engine configuration, backed by GSettings (dconf).
//!
//! Lives in its own crate so the engine binary and the preferences GUI read and
//! write exactly the same settings through one schema, with no risk of drift.

pub mod config;
pub mod flags;

pub use config::{Config, SCHEMA_ID, keys};
pub use flags::{IBFlags, InputMode};
