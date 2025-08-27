//! Common configuration utilities for strategy services

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Resolve configuration file path with workspace-aware logic
pub fn resolve_config_path(env_var: &str, default_relative_path: &str) -> PathBuf {
    // First check environment variable
    if let Ok(path) = std::env::var(env_var) {
        return PathBuf::from(path);
    }

    // Try to find workspace root and use absolute path
    if let Some(workspace_root) = find_workspace_root() {
        let mut config_path = workspace_root;
        config_path.push("services_v2/strategies");
        config_path.push(default_relative_path);
        
        if config_path.exists() {
            return config_path;
        }
    }

    // Fall back to relative path from current directory
    PathBuf::from(default_relative_path)
}

/// Find the workspace root by looking for Cargo.toml with [workspace]
fn find_workspace_root() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;
    
    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(contents) = std::fs::read_to_string(&cargo_toml) {
                if contents.contains("[workspace]") {
                    return Some(current);
                }
            }
        }
        
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }
    
    None
}

/// Load configuration file with proper error handling
pub fn load_config_file<T>(config_path: &Path, default_config: T) -> Result<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    if !config_path.exists() {
        tracing::info!("Config file {:?} not found, using defaults", config_path);
        return Ok(default_config);
    }

    let config_str = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

    let config: T = toml::from_str(&config_str)
        .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;

    tracing::info!("Loaded configuration from {:?}", config_path);
    Ok(config)
}