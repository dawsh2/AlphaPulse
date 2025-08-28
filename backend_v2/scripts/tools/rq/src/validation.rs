use anyhow::{bail, Result};
use regex::Regex;
use std::sync::LazyLock;

/// Maximum pattern length to prevent DoS
const MAX_PATTERN_LENGTH: usize = 256;

/// Maximum module path depth
const MAX_MODULE_DEPTH: usize = 10;

/// Valid type names for filtering
const VALID_TYPES: &[&str] = &[
    "struct", "enum", "function", "fn", "trait", "impl", "type", "const", "static", "macro",
    "module", "mod",
];

/// Regex for validating safe patterns (no regex bombs)
static SAFE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_\-\.\*\?\[\]\{\}\(\)\|\\/:, ]+$").unwrap());

/// Regex for validating crate names
static CRATE_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9_\-]*$").unwrap());

/// Regex for validating module paths
static MODULE_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*(::[a-zA-Z][a-zA-Z0-9_]*)*$").unwrap());

/// Input validation for user queries
pub struct QueryValidator;

impl QueryValidator {
    /// Validate and sanitize a search pattern
    pub fn validate_pattern(pattern: &str) -> Result<String> {
        // Check length
        if pattern.is_empty() {
            bail!("Search pattern cannot be empty");
        }

        if pattern.len() > MAX_PATTERN_LENGTH {
            bail!(
                "Pattern too long (max {} characters, got {})",
                MAX_PATTERN_LENGTH,
                pattern.len()
            );
        }

        // Check for dangerous regex patterns
        if pattern.contains("(?") {
            bail!("Advanced regex patterns are not supported for safety");
        }

        // Check for potential regex bombs
        let dangerous_patterns = [
            "(.*)*", "(.+)+", "(.*)+", "(.+)*", "(?:.*)*", "(?:.+)+", "(\\s*)*", "(\\w*)*",
        ];

        for dangerous in &dangerous_patterns {
            if pattern.contains(dangerous) {
                bail!("Pattern '{}' could cause performance issues", dangerous);
            }
        }

        // Check for excessive repetition
        if pattern.contains("{") {
            let re = Regex::new(r"\{(\d+),?(\d*)\}").unwrap();
            for cap in re.captures_iter(pattern) {
                let min: usize = cap[1].parse().unwrap_or(0);
                let max: usize = cap
                    .get(2)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(min);

                if min > 100 || max > 100 {
                    bail!("Repetition count too high (max 100)");
                }
            }
        }

        // Sanitize by escaping special characters if not a valid regex
        let sanitized = if SAFE_PATTERN.is_match(pattern) {
            pattern.to_string()
        } else {
            // Escape special regex characters for literal matching
            regex::escape(pattern)
        };

        Ok(sanitized)
    }

    /// Validate item type filter
    pub fn validate_type(item_type: &str) -> Result<String> {
        let normalized = item_type.to_lowercase();

        // Map common aliases
        let mapped = match normalized.as_str() {
            "fn" => "function",
            "mod" => "module",
            other => other,
        };

        if !VALID_TYPES.contains(&mapped) {
            bail!(
                "Invalid type '{}'. Valid types: {}",
                item_type,
                VALID_TYPES.join(", ")
            );
        }

        Ok(mapped.to_string())
    }

    /// Validate crate name
    pub fn validate_crate_name(name: &str) -> Result<String> {
        if name.is_empty() {
            bail!("Crate name cannot be empty");
        }

        if name.len() > 64 {
            bail!("Crate name too long (max 64 characters)");
        }

        if !CRATE_NAME.is_match(name) {
            bail!(
                "Invalid crate name '{}'. Must start with a letter and contain only letters, numbers, underscores, and hyphens",
                name
            );
        }

        Ok(name.to_string())
    }

    /// Validate module path
    pub fn validate_module_path(path: &str) -> Result<String> {
        if path.is_empty() {
            return Ok(String::new());
        }

        let depth = path.matches("::").count() + 1;
        if depth > MAX_MODULE_DEPTH {
            bail!(
                "Module path too deep (max {} levels, got {})",
                MAX_MODULE_DEPTH,
                depth
            );
        }

        if !MODULE_PATH.is_match(path) {
            bail!(
                "Invalid module path '{}'. Use format like 'module::submodule'",
                path
            );
        }

        Ok(path.to_string())
    }

    /// Validate similarity threshold
    pub fn validate_threshold(threshold: f32) -> Result<f32> {
        if !(0.0..=1.0).contains(&threshold) {
            bail!("Similarity threshold must be between 0.0 and 1.0");
        }
        Ok(threshold)
    }

    /// Validate depth parameter
    pub fn validate_depth(depth: usize) -> Result<usize> {
        if depth == 0 {
            bail!("Depth must be at least 1");
        }

        if depth > 10 {
            bail!("Depth too large (max 10 levels)");
        }

        Ok(depth)
    }

    /// Sanitize file path for display
    pub fn sanitize_path(path: &str) -> String {
        // Remove any path traversal attempts
        path.replace("..", "")
            .replace("~", "")
            .trim_start_matches('/')
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_pattern() {
        // Valid patterns
        assert!(QueryValidator::validate_pattern("test").is_ok());
        assert!(QueryValidator::validate_pattern("test_function").is_ok());
        assert!(QueryValidator::validate_pattern("Test::Module").is_ok());

        // Invalid patterns
        assert!(QueryValidator::validate_pattern("").is_err());
        assert!(QueryValidator::validate_pattern(&"x".repeat(300)).is_err());
        assert!(QueryValidator::validate_pattern("(.*)* ").is_err());
        assert!(QueryValidator::validate_pattern("(?:test)").is_err());
    }

    #[test]
    fn test_validate_type() {
        assert_eq!(QueryValidator::validate_type("struct").unwrap(), "struct");
        assert_eq!(QueryValidator::validate_type("fn").unwrap(), "function");
        assert_eq!(QueryValidator::validate_type("FN").unwrap(), "function");
        assert!(QueryValidator::validate_type("invalid").is_err());
    }

    #[test]
    fn test_validate_crate_name() {
        assert!(QueryValidator::validate_crate_name("my_crate").is_ok());
        assert!(QueryValidator::validate_crate_name("my-crate").is_ok());
        assert!(QueryValidator::validate_crate_name("crate123").is_ok());

        assert!(QueryValidator::validate_crate_name("").is_err());
        assert!(QueryValidator::validate_crate_name("123crate").is_err());
        assert!(QueryValidator::validate_crate_name("my.crate").is_err());
    }

    #[test]
    fn test_validate_module_path() {
        assert!(QueryValidator::validate_module_path("").is_ok());
        assert!(QueryValidator::validate_module_path("module").is_ok());
        assert!(QueryValidator::validate_module_path("module::submodule").is_ok());

        assert!(QueryValidator::validate_module_path("::module").is_err());
        assert!(QueryValidator::validate_module_path("module::").is_err());

        // Test depth limit
        let deep = "a::b::c::d::e::f::g::h::i::j::k";
        assert!(QueryValidator::validate_module_path(deep).is_err());
    }
}
