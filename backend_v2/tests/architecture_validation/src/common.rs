//! Common utilities for architecture validation tests

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;
use walkdir::WalkDir;
use anyhow::{Context, Result};
use syn::{File, ItemFn, ItemStruct, ItemEnum, visit::Visit};

/// Represents a code violation found during validation
#[derive(Debug, Clone)]
pub struct Violation {
    pub file_path: PathBuf,
    pub line: Option<usize>,
    pub violation_type: ViolationType,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViolationType {
    ProtocolV2Compliance,
    PrecisionViolation,
    MockUsage,
    FileOrganization,
    DuplicateImplementation,
    Performance,
    BreakingChange,
    Documentation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

impl Violation {
    pub fn new(
        file_path: PathBuf,
        violation_type: ViolationType,
        message: String,
        severity: Severity,
    ) -> Self {
        Self {
            file_path,
            line: None,
            violation_type,
            message,
            severity,
        }
    }

    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }
}

/// Get all Rust files in the backend_v2 directory
pub fn get_rust_files(base_path: &Path) -> Result<Vec<PathBuf>> {
    let mut rust_files = Vec::new();
    
    for entry in WalkDir::new(base_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(extension) = entry.path().extension() {
                if extension == "rs" {
                    rust_files.push(entry.into_path());
                }
            }
        }
    }
    
    Ok(rust_files)
}

/// Parse a Rust file into a syn AST
pub fn parse_rust_file(file_path: &Path) -> Result<File> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
    
    syn::parse_file(&content)
        .with_context(|| format!("Failed to parse Rust file: {}", file_path.display()))
}

/// Check if a file path matches any of the given patterns
pub fn matches_patterns(file_path: &Path, patterns: &[&str]) -> bool {
    let path_str = file_path.to_string_lossy();
    patterns.iter().any(|pattern| {
        if let Ok(regex) = Regex::new(pattern) {
            regex.is_match(&path_str)
        } else {
            false
        }
    })
}

/// Get the project root directory (backend_v2)
pub fn get_project_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    
    // Walk up the directory tree looking for backend_v2
    let mut path = current_dir.as_path();
    loop {
        if path.file_name() == Some(std::ffi::OsStr::new("backend_v2")) {
            return Ok(path.to_path_buf());
        }
        
        if let Some(parent) = path.parent() {
            path = parent;
        } else {
            break;
        }
    }
    
    // Fallback: look for Cargo.toml with workspace definition
    let mut path = current_dir.as_path();
    loop {
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") && path.ends_with("backend_v2") {
                    return Ok(path.to_path_buf());
                }
            }
        }
        
        if let Some(parent) = path.parent() {
            path = parent;
        } else {
            break;
        }
    }
    
    anyhow::bail!("Could not find backend_v2 project root")
}

/// Financial keywords that should not be used with floating-point types
pub const FINANCIAL_KEYWORDS: &[&str] = &[
    "price", "bid", "ask", "spread", "cost", "value", "worth",
    "trade", "order", "position", "quantity", "amount", "volume",
    "profit", "loss", "fee", "commission", "interest", "yield", "return",
    "portfolio", "asset", "balance", "equity", "capital", "fund",
    "reserve", "liquidity", "swap", "mint", "burn", "slippage",
    "usd", "eth", "btc", "token", "coin", "currency", "money", "wei"
];

/// Magic numbers that should be present in Protocol V2
pub const PROTOCOL_V2_MAGIC_NUMBERS: &[u32] = &[
    0xDEADBEEF, // MessageHeader magic
];

/// TLV type ranges for different domains
pub const MARKET_DATA_TLV_RANGE: std::ops::RangeInclusive<u16> = 1..=19;
pub const SIGNAL_TLV_RANGE: std::ops::RangeInclusive<u16> = 20..=39;
pub const EXECUTION_TLV_RANGE: std::ops::RangeInclusive<u16> = 40..=79;

/// Check if a string contains mock-related keywords
pub fn contains_mock_keywords(content: &str) -> bool {
    let mock_patterns = [
        r"\bmock\b",
        r"\bfake\b",
        r"\bdummy\b",
        r"\bstub\b",
        r"\bsimulate\b",
        r"MockService",
        r"FakeData",
        r"DummyResponse",
        r"TestDouble",
    ];
    
    mock_patterns.iter().any(|pattern| {
        if let Ok(regex) = Regex::new(pattern) {
            regex.is_match(content)
        } else {
            false
        }
    })
}

/// Visitor to collect function and struct names from AST
pub struct NameCollector {
    pub functions: HashSet<String>,
    pub structs: HashSet<String>,
    pub enums: HashSet<String>,
}

impl NameCollector {
    pub fn new() -> Self {
        Self {
            functions: HashSet::new(),
            structs: HashSet::new(),
            enums: HashSet::new(),
        }
    }
}

impl<'ast> Visit<'ast> for NameCollector {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        self.functions.insert(node.sig.ident.to_string());
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        self.structs.insert(node.ident.to_string());
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast ItemEnum) {
        self.enums.insert(node.ident.to_string());
        syn::visit::visit_item_enum(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_mock_keywords() {
        assert!(contains_mock_keywords("let mock_service = MockService::new()"));
        assert!(contains_mock_keywords("fake_data.clone()"));
        assert!(contains_mock_keywords("dummy response"));
        assert!(!contains_mock_keywords("service.authenticate()"));
    }

    #[test]
    fn test_financial_keywords() {
        assert!(FINANCIAL_KEYWORDS.contains(&"price"));
        assert!(FINANCIAL_KEYWORDS.contains(&"trade"));
        assert!(FINANCIAL_KEYWORDS.contains(&"wei"));
    }
}