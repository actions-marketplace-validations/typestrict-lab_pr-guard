//! Service configuration: one TOML file per profile (`dev.toml`, `prod.toml`, ...).

pub mod config;
mod duration;
pub mod loader;

pub use config::Config;
pub use loader::{load, LoadError};
