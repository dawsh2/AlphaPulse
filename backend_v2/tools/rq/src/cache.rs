use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Simple file-based cache using rustdoc JSON
pub struct SimpleCache {
    workspace_root: PathBuf,
}

impl SimpleCache {
    pub fn new() -> Result<Self> {
        let workspace_root = Self::find_workspace_root()?;

        if std::env::var("RQ_DEBUG").is_ok() {
            eprintln!("Found workspace root: {}", workspace_root.display());
        }

        Ok(Self { workspace_root })
    }

    fn find_workspace_root() -> Result<PathBuf> {
        let current = std::env::current_dir()?;
        let mut path = current.as_path();
        let mut fallback_single_crate = None;

        loop {
            if path.join("Cargo.toml").exists() {
                let content = fs::read_to_string(path.join("Cargo.toml"))?;
                if content.contains("[workspace]") {
                    return Ok(path.to_path_buf());
                }
                // Remember first single crate as fallback, but keep searching for workspace
                if fallback_single_crate.is_none() && path.join("src").exists() {
                    fallback_single_crate = Some(path.to_path_buf());
                }
            }

            match path.parent() {
                Some(parent) => path = parent,
                None => break,
            }
        }

        // If we found a workspace, that takes precedence; otherwise use single crate fallback
        Ok(fallback_single_crate.unwrap_or(current))
    }

    /// Check if rustdoc needs updating for a crate
    pub fn needs_update(&self, crate_path: &Path) -> Result<bool> {
        let json_path = self.get_json_path(crate_path);

        if !json_path.exists() {
            return Ok(true);
        }

        // Check if any source files are newer than the JSON
        let json_modified = fs::metadata(&json_path)?.modified()?;

        let src_dir = crate_path.join("src");
        if !src_dir.exists() {
            return Ok(false);
        }

        for entry in WalkDir::new(src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if modified > json_modified {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Generate rustdoc JSON for a crate
    pub fn update_crate(&self, crate_path: &Path) -> Result<()> {
        println!("Updating {}...", crate_path.display());

        // Run cargo doc from the individual crate directory to generate JSON in its own target
        let mut cmd = std::process::Command::new("cargo");
        cmd.env("RUSTDOCFLAGS", "-Z unstable-options --output-format json");
        cmd.arg("+nightly");
        cmd.arg("doc");
        cmd.arg("--lib");
        cmd.arg("--no-deps");
        cmd.current_dir(crate_path);

        let output = cmd.output()?;

        if !output.status.success() {
            eprintln!(
                "Warning: Failed to generate rustdoc JSON for {}: {}",
                crate_path.display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Load rustdoc JSON for a crate
    pub fn load_rustdoc(&self, crate_path: &Path) -> Result<Value> {
        let json_path = self.get_json_path(crate_path);

        let content = fs::read_to_string(&json_path)
            .with_context(|| format!("Failed to read rustdoc JSON: {}", json_path.display()))?;

        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON from {}", json_path.display()))
    }

    fn get_json_path(&self, crate_path: &Path) -> PathBuf {
        // Get actual crate name from Cargo.toml
        let crate_name = self.get_crate_name(crate_path).unwrap_or_else(|_| {
            crate_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace('-', "_")
        });

        // Try workspace-level first (simpler names)
        let workspace_json = self
            .workspace_root
            .join("target")
            .join("doc")
            .join(format!("{}.json", crate_name.replace('-', "_")));

        if workspace_json.exists() {
            return workspace_json;
        }

        // Fall back to individual crate's target directory (prefixed names)
        crate_path
            .join("target")
            .join("doc")
            .join(format!("alphapulse_{}.json", crate_name.replace('-', "_")))
    }

    /// Get the actual crate name from Cargo.toml
    fn get_crate_name(&self, crate_path: &Path) -> Result<String> {
        let cargo_toml = crate_path.join("Cargo.toml");
        let content = fs::read_to_string(cargo_toml)?;

        // Simple TOML parsing to get package name
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("name = ") {
                let name = line
                    .strip_prefix("name = ")
                    .and_then(|s| s.strip_prefix("\""))
                    .and_then(|s| s.strip_suffix("\""))
                    .unwrap_or("unknown");
                return Ok(name.to_string());
            }
        }

        Err(anyhow::anyhow!("No package name found in Cargo.toml"))
    }

    /// Find all crates in workspace
    pub fn find_crates(&self) -> Result<Vec<PathBuf>> {
        let mut crates = Vec::new();

        // Check if this is a single crate project (has src/ but no [workspace])
        let workspace_cargo = self.workspace_root.join("Cargo.toml");
        if workspace_cargo.exists() {
            let content = fs::read_to_string(&workspace_cargo)?;
            if !content.contains("[workspace]") && self.workspace_root.join("src").exists() {
                // Single crate project
                crates.push(self.workspace_root.clone());
                return Ok(crates);
            }
        }

        // Otherwise, find all workspace member crates
        for entry in WalkDir::new(&self.workspace_root)
            .max_depth(4) // Increased depth to find nested crates
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() == "Cargo.toml")
        {
            if let Some(parent) = entry.path().parent() {
                // Skip the workspace root Cargo.toml itself
                if parent == self.workspace_root {
                    continue;
                }
                // Must have a src directory to be a crate
                if parent.join("src").exists() {
                    crates.push(parent.to_path_buf());
                }
            }
        }

        if std::env::var("RQ_DEBUG").is_ok() && !crates.is_empty() {
            eprintln!("Found {} crates in workspace:", crates.len());
            for crate_path in &crates {
                eprintln!("  - {}", crate_path.display());
            }
        }

        Ok(crates)
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }
}
