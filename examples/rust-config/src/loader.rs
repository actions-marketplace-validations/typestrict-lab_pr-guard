use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::Config;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LoadError {
    #[error("invalid profile name {0:?}")]
    InvalidProfile(String),
    #[error("cannot read {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("cannot parse {path}: {source}")]
    Parse { path: PathBuf, source: toml::de::Error },
}

/// Loads `<dir>/<profile>.toml`. Profile names may only contain ASCII letters, digits, `-` and `_`.
pub fn load(dir: &Path, profile: &str) -> Result<Config, LoadError> {
    if profile.is_empty() || !profile.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(LoadError::InvalidProfile(profile.to_string()));
    }
    let path = dir.join(format!("{profile}.toml"));
    let text = std::fs::read_to_string(&path).map_err(|source| LoadError::Io { path: path.clone(), source })?;
    toml::from_str(&text).map_err(|source| LoadError::Parse { path, source })
}
