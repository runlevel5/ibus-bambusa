//! Shared engine configuration: the on-disk settings plus the feature flags.
//!
//! Lives in its own crate so the engine binary and the preferences GUI read and
//! write exactly the same `Config`, with no risk of the two drifting apart.

pub mod config;
pub mod flags;

pub use config::Config;
pub use flags::{IBFlags, InputMode};
