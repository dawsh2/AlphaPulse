//! Strategy configuration utilities

use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::Result;

/// Load configuration from TOML file
pub fn load_config<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T> {
    let content = std::fs::read_to_string(path)?;
    let config = toml::from_str(&content)?;
    Ok(config)
}

/// Common strategy configuration fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseStrategyConfig {
    pub name: String,
    pub enabled: bool,
    pub log_level: Option<String>,
}

impl Default for BaseStrategyConfig {
    fn default() -> Self {
        Self {
            name: "unnamed_strategy".to_string(),
            enabled: true,
            log_level: Some("info".to_string()),
        }
    }
}