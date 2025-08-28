//! Protocol V2 Compliance Validator for Torq
//!
//! Comprehensive validation system ensuring code compliance with Torq's
//! Protocol V2 TLV architecture, including magic number validation, precision
//! preservation, type registry compliance, and architectural patterns.

use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Failed to read file {file}: {error}")]
    FileReadError { file: String, error: String },
    
    #[error("Failed to compile regex pattern: {error}")]
    RegexError { error: String },
    
    #[error("Protocol validation failed: {message}")]
    ProtocolError { message: String },
}

/// Severity levels for validation findings
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationSeverity {
    /// Critical protocol violations that break system functionality
    Critical,
    /// Warning patterns that may indicate issues
    Warning,
    /// Informational suggestions for improvement
    Info,
}

/// Categories of Protocol V2 validation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValidationCategory {
    /// TLV message format compliance
    TLVFormat,
    /// Magic number and header validation
    MagicNumber,
    /// Precision preservation (no floating point for prices)
    Precision,
    /// TLV type registry compliance
    TypeRegistry,
    /// Codec usage (must use codec, not manual parsing)
    Codec,
    /// Architecture patterns (service boundaries, etc.)
    Architecture,
    /// Performance patterns (hot path requirements)
    Performance,
}

/// A Protocol V2 validation finding
#[derive(Debug, Clone)]
pub struct ValidationFinding {
    /// File path where issue was found
    pub file_path: String,
    /// Line number (1-based)
    pub line_number: usize,
    /// Column position (0-based)
    pub column: usize,
    /// Severity of this finding
    pub severity: ValidationSeverity,
    /// Category of validation
    pub category: ValidationCategory,
    /// Description of the issue
    pub description: String,
    /// The actual text that triggered the finding
    pub matched_text: String,
    /// Suggestion for fixing the issue
    pub suggestion: String,
    /// Pattern ID that triggered this finding
    pub pattern_id: String,
    /// Additional context if available
    pub context: Option<String>,
}

/// Validation pattern definition
#[derive(Debug, Clone)]
pub struct ValidationPattern {
    /// Unique identifier
    pub id: String,
    /// Regex pattern
    pub regex: Regex,
    /// Severity level
    pub severity: ValidationSeverity,
    /// Validation category
    pub category: ValidationCategory,
    /// Description of what this pattern checks
    pub description: String,
    /// Fix suggestion
    pub suggestion: String,
    /// File extensions this applies to
    pub file_extensions: Vec<String>,
}

/// Protocol V2 compliance validator
pub struct ProtocolV2Validator {
    /// Validation patterns organized by category
    patterns: HashMap<ValidationCategory, Vec<ValidationPattern>>,
    /// Known TLV type numbers to detect conflicts
    known_tlv_types: HashSet<u8>,
    /// Expected magic number
    expected_magic: u32,
}

impl ProtocolV2Validator {
    /// Create new validator with Protocol V2 patterns
    pub fn new() -> Result<Self, ValidationError> {
        let mut validator = Self {
            patterns: HashMap::new(),
            known_tlv_types: HashSet::new(),
            expected_magic: 0xDEADBEEF,
        };
        
        validator.load_tlv_format_patterns()?;
        validator.load_magic_number_patterns()?;
        validator.load_precision_patterns()?;
        validator.load_type_registry_patterns()?;
        validator.load_codec_patterns()?;
        validator.load_architecture_patterns()?;
        validator.load_performance_patterns()?;
        validator.load_known_tlv_types();
        
        Ok(validator)
    }
    
    /// Load TLV format validation patterns
    fn load_tlv_format_patterns(&mut self) -> Result<(), ValidationError> {
        let patterns = vec![
            // Message header structure validation
            ("header-size", r"MessageHeader.*32.*byte", "Good: 32-byte header mentioned", "Continue using 32-byte MessageHeader"),
            ("tlv-header", r"SimpleTLVHeader|ExtendedTLVHeader", "Good: Using proper TLV headers", "Continue using standard TLV headers"),
            ("payload-size", r"payload_size.*u16|payload_size.*payload.*len", "Good: Proper payload size handling", "Continue using u16 for payload size"),
            
            // Header structure violations  
            ("custom-header", r"struct.*Header.*\{(?!.*magic.*payload_size)", "Custom header without standard fields", "Use MessageHeader from protocol_v2"),
            ("wrong-header-size", r"header.*(?:8|16|64).*byte", "Wrong header size", "Use 32-byte MessageHeader"),
            ("manual-header", r"&bytes\[0\.\.(?:8|16|64)\]", "Manual header parsing", "Use parse_header() from codec"),
        ];
        
        self.add_patterns(ValidationCategory::TLVFormat, patterns)?;
        Ok(())
    }
    
    /// Load magic number validation patterns
    fn load_magic_number_patterns(&mut self) -> Result<(), ValidationError> {
        let patterns = vec![
            // Correct magic number usage
            ("correct-magic", r"0xDEADBEEF|MESSAGE_MAGIC", "Good: Correct magic number", "Continue using 0xDEADBEEF"),
            ("magic-check", r"header\.magic.*==.*0xDEADBEEF", "Good: Magic number validation", "Continue validating magic number"),
            
            // Wrong magic numbers
            ("wrong-magic-hex", r"0x[0-9A-Fa-f]{8}(?!DEADBEEF)", "Incorrect magic number", "Use 0xDEADBEEF for Protocol V2"),
            ("hardcoded-magic", r"magic.*=.*0x(?!DEADBEEF)[0-9A-Fa-f]{8}", "Hardcoded wrong magic", "Use MESSAGE_MAGIC constant (0xDEADBEEF)"),
            ("no-magic-check", r"parse_header.*\?(?!.*magic)", "Header parsed without magic validation", "Add magic number validation after parsing"),
            
            // Byte order issues
            ("little-endian", r"to_le_bytes|from_le_bytes.*magic", "Little endian magic number", "Use big endian for network byte order"),
            ("magic-bytes", r"magic.*as_bytes|magic.*to_bytes", "Manual magic byte conversion", "Use network byte order conversion"),
        ];
        
        self.add_patterns(ValidationCategory::MagicNumber, patterns)?;
        Ok(())
    }
    
    /// Load precision preservation patterns
    fn load_precision_patterns(&mut self) -> Result<(), ValidationError> {
        let patterns = vec![
            // Good precision patterns
            ("native-precision", r"18.*decimal|6.*decimal|native.*precision", "Good: Native token precision", "Continue using native precision"),
            ("fixed-point", r"8.*decimal.*fixed.*point|100_000_000", "Good: 8-decimal fixed-point for USD", "Continue using fixed-point for traditional prices"),
            
            // Precision loss patterns (CRITICAL)
            ("float-price", r"(?i)price.*:\s*f(32|64)", "Floating point price detected", "Use native precision (i64) for DEX or 8-decimal fixed-point for USD"),
            ("float-amount", r"(?i)amount.*:\s*f(32|64)", "Floating point amount detected", "Use native token precision (i64)"),
            ("as-f64-price", r"\.as_f64\(\).*(?i)price", "Price converted to f64", "Keep prices in integer representation"),
            ("division-precision", r"price.*\/.*(?!100_000_000)", "Price division may lose precision", "Use integer arithmetic with proper scaling"),
            
            // Scaling violations
            ("wrong-scaling", r"price.*\*.*1000(?!_000_000)", "Wrong price scaling", "Use 100_000_000 for 8-decimal fixed-point"),
            ("truncation", r"as.*u64.*price|as.*u32.*price", "Potential price truncation", "Ensure no precision loss in conversion"),
            ("rounding", r"round\(\)|floor\(\)|ceil\(\).*price", "Price rounding detected", "Avoid rounding financial data"),
        ];
        
        self.add_patterns(ValidationCategory::Precision, patterns)?;
        Ok(())
    }
    
    /// Load TLV type registry patterns
    fn load_type_registry_patterns(&mut self) -> Result<(), ValidationError> {
        let patterns = vec![
            // Good type registry usage
            ("tlv-type-enum", r"TLVType::\w+", "Good: Using TLVType enum", "Continue using TLVType enum"),
            ("type-from-u8", r"TLVType::try_from.*u8", "Good: Safe type conversion", "Continue using try_from for type conversion"),
            ("expected-size", r"expected_payload_size.*match|expected_payload_size\(\)", "Good: Size validation", "Continue validating payload sizes"),
            
            // Type registry violations
            ("hardcoded-type", r"tlv_type.*=.*\d+(?!.*TLVType)", "Hardcoded TLV type number", "Use TLVType enum instead of raw numbers"),
            ("duplicate-type", r"TLVType::\w+.*=.*(\d+).*TLVType::\w+.*=.*\1", "Duplicate TLV type number", "Ensure all TLV types have unique numbers"),
            ("missing-size", r"fn expected_payload_size.*todo!|fn expected_payload_size.*unimplemented!", "Missing payload size implementation", "Implement expected_payload_size() for all TLV types"),
            
            // Domain violations
            ("wrong-domain", r"TLVType::\w+.*=.*(?:0|[2-9]\d|1[0-9]|[5-9]\d)", "TLV type outside valid domain", "Use correct domain ranges: MarketData(1-19), Signal(20-39), Execution(40-59)"),
        ];
        
        self.add_patterns(ValidationCategory::TypeRegistry, patterns)?;
        Ok(())
    }
    
    /// Load codec usage patterns
    fn load_codec_patterns(&mut self) -> Result<(), ValidationError> {
        let patterns = vec![
            // Good codec usage
            ("tlv-builder", r"TLVMessageBuilder::new", "Good: Using TLVMessageBuilder", "Continue using codec for message construction"),
            ("parse-header", r"parse_header\(", "Good: Using codec for header parsing", "Continue using codec functions"),
            ("parse-tlv", r"parse_tlv_extensions\(", "Good: Using codec for TLV parsing", "Continue using codec functions"),
            ("codec-import", r"use.*codec::", "Good: Importing codec", "Continue using codec crate"),
            
            // Manual parsing violations (CRITICAL)
            ("manual-tlv", r"&payload\[\d+\.\.\d*\](?!.*//.*codec)", "Manual TLV parsing", "Use parse_tlv_extensions() from codec"),
            ("manual-header", r"&bytes\[0\.\.32\](?!.*//.*header)", "Manual header extraction", "Use parse_header() from codec"),
            ("custom-builder", r"fn.*build.*tlv(?!.*//.*delegate)", "Custom TLV builder", "Use TLVMessageBuilder from codec"),
            ("direct-serialization", r"as_bytes.*tlv|tlv.*serialize", "Manual TLV serialization", "Use codec for serialization"),
            
            // Protocol duplication
            ("duplicate-parser", r"fn parse_tlv_message", "Duplicate TLV parser", "Use codec functions instead of reimplementing"),
            ("duplicate-builder", r"fn build_message.*header", "Duplicate message builder", "Use TLVMessageBuilder from codec"),
        ];
        
        self.add_patterns(ValidationCategory::Codec, patterns)?;
        Ok(())
    }
    
    /// Load architecture compliance patterns
    fn load_architecture_patterns(&mut self) -> Result<(), ValidationError> {
        let patterns = vec![
            // Good architecture patterns
            ("service-boundary", r"use.*libs::|use.*crate::libs", "Good: Using shared libraries", "Continue using libs/ for shared functionality"),
            ("relay-domain", r"RelayDomain::(MarketData|Signal|Execution)", "Good: Using relay domains", "Continue using proper relay domains"),
            ("source-type", r"SourceType::\w+", "Good: Source attribution", "Continue using SourceType for message attribution"),
            
            // Architecture violations
            ("cross-service", r"use.*services\d*.*::", "Cross-service import", "Use shared libraries instead of direct service imports"),
            ("relative-service", r"use.*\.\..*services.*", "Relative service import", "Use absolute imports through libs/"),
            ("circular-dep", r"extern crate.*self", "Potential circular dependency", "Review dependency structure"),
            
            // Service boundary violations  
            ("direct-service-call", r"services\d*::\w+::", "Direct service access", "Use message passing or shared libraries"),
            ("bypassing-relay", r"socket\.write.*(?!.*relay)", "Bypassing relay system", "Send messages through appropriate relay"),
        ];
        
        self.add_patterns(ValidationCategory::Architecture, patterns)?;
        Ok(())
    }
    
    /// Load performance validation patterns
    fn load_performance_patterns(&mut self) -> Result<(), ValidationError> {
        let patterns = vec![
            // Good performance patterns
            ("zero-copy", r"zerocopy::|AsBytes|FromBytes", "Good: Zero-copy operations", "Continue using zerocopy traits"),
            ("hot-path-buffer", r"with_hot_path_buffer|HotPathBuffer", "Good: Pre-allocated buffers", "Continue using hot path buffers"),
            ("fast-timestamp", r"fast_timestamp_ns", "Good: Fast timestamp", "Continue using cached timestamps"),
            
            // Performance violations in hot paths
            ("hot-path-alloc", r"Vec::new\(\).*hot.*path|HashMap::new\(\).*hot.*path", "Allocation in hot path", "Pre-allocate buffers or use hot path helpers"),
            ("hot-path-string", r"String::from.*hot.*path|format!.*hot.*path", "String allocation in hot path", "Use pre-allocated buffers or static strings"),
            ("hot-path-await", r"\.await.*hot.*path", "Async operation in hot path", "Use non-blocking operations for <35μs requirement"),
            ("hot-path-blocking", r"block_on.*hot.*path", "Blocking operation in hot path", "Remove blocking operations from hot path"),
            
            // Inefficient operations
            ("clone-unnecessary", r"\.clone\(\)(?!.*//.*required)", "Unnecessary clone", "Avoid cloning unless required, document if needed"),
            ("collect-vec", r"\.collect::<Vec<_>>\(\)(?!.*//.*required)", "Collection to Vec", "Consider streaming or iterator chains"),
            ("linear-search", r"\.contains.*Vec|\.find.*Vec.*len", "Linear search in Vec", "Use HashMap or HashSet for O(1) lookups"),
            
            // Latency violations
            ("millisecond-delay", r"Duration::from_millis\([1-9]\d*\)", "Millisecond delay", "Use microsecond precision for trading operations"),
            ("sleep-hot-path", r"sleep.*hot.*path|delay.*hot.*path", "Sleep in hot path", "Remove delays from performance-critical code"),
        ];
        
        self.add_patterns(ValidationCategory::Performance, patterns)?;
        Ok(())
    }
    
    /// Load known TLV type numbers to detect conflicts
    fn load_known_tlv_types(&mut self) {
        // Market Data domain (1-19)
        for i in 1..=19 {
            self.known_tlv_types.insert(i);
        }
        
        // Signal domain (20-39)
        for i in 20..=39 {
            self.known_tlv_types.insert(i);
        }
        
        // Execution domain (40-59)
        for i in 40..=59 {
            self.known_tlv_types.insert(i);
        }
        
        // System domain (100-119)
        for i in 100..=119 {
            self.known_tlv_types.insert(i);
        }
    }
    
    /// Helper to add patterns to a category
    fn add_patterns(
        &mut self,
        category: ValidationCategory,
        pattern_specs: Vec<(&str, &str, &str, &str)>,
    ) -> Result<(), ValidationError> {
        let mut patterns = Vec::new();
        
        for (id, regex_str, description, suggestion) in pattern_specs {
            let severity = if description.starts_with("Good:") {
                ValidationSeverity::Info
            } else if description.contains("CRITICAL") || 
                      regex_str.contains("f(32|64)") ||
                      regex_str.contains("manual.*tlv") {
                ValidationSeverity::Critical
            } else {
                ValidationSeverity::Warning
            };
            
            patterns.push(ValidationPattern {
                id: id.to_string(),
                regex: Regex::new(regex_str).map_err(|e| ValidationError::RegexError {
                    error: e.to_string(),
                })?,
                severity,
                category: category.clone(),
                description: description.to_string(),
                suggestion: suggestion.to_string(),
                file_extensions: vec!["rs".to_string()],
            });
        }
        
        self.patterns.insert(category, patterns);
        Ok(())
    }
    
    /// Validate a single file for Protocol V2 compliance
    pub fn validate_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<ValidationFinding>, ValidationError> {
        let path = file_path.as_ref();
        let path_str = path.to_string_lossy().to_string();
        
        // Skip non-Rust files
        if !path_str.ends_with(".rs") {
            return Ok(Vec::new());
        }
        
        let content = fs::read_to_string(path).map_err(|e| ValidationError::FileReadError {
            file: path_str.clone(),
            error: e.to_string(),
        })?;
        
        let mut findings = Vec::new();
        
        // Apply all validation patterns
        for patterns in self.patterns.values() {
            for pattern in patterns {
                findings.extend(self.validate_content_with_pattern(&content, &path_str, pattern)?);
            }
        }
        
        // Additional context-aware validations
        findings.extend(self.validate_tlv_type_conflicts(&content, &path_str)?);
        findings.extend(self.validate_precision_context(&content, &path_str)?);
        
        Ok(findings)
    }
    
    /// Validate content with a specific pattern
    fn validate_content_with_pattern(
        &self,
        content: &str,
        file_path: &str,
        pattern: &ValidationPattern,
    ) -> Result<Vec<ValidationFinding>, ValidationError> {
        let mut findings = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            for mat in pattern.regex.find_iter(line) {
                findings.push(ValidationFinding {
                    file_path: file_path.to_string(),
                    line_number: line_num + 1,
                    column: mat.start(),
                    severity: pattern.severity.clone(),
                    category: pattern.category.clone(),
                    description: pattern.description.clone(),
                    matched_text: mat.as_str().to_string(),
                    suggestion: pattern.suggestion.clone(),
                    pattern_id: pattern.id.clone(),
                    context: None,
                });
            }
        }
        
        Ok(findings)
    }
    
    /// Validate TLV type number conflicts
    fn validate_tlv_type_conflicts(
        &self,
        content: &str,
        file_path: &str,
    ) -> Result<Vec<ValidationFinding>, ValidationError> {
        let mut findings = Vec::new();
        let type_regex = Regex::new(r"TLVType::\w+\s*=\s*(\d+)").unwrap();
        let mut found_types: HashMap<u8, (usize, String)> = HashMap::new();
        
        for (line_num, line) in content.lines().enumerate() {
            for cap in type_regex.captures_iter(line) {
                if let Ok(type_num) = cap[1].parse::<u8>() {
                    if let Some((existing_line, existing_name)) = found_types.get(&type_num) {
                        findings.push(ValidationFinding {
                            file_path: file_path.to_string(),
                            line_number: line_num + 1,
                            column: cap.get(0).unwrap().start(),
                            severity: ValidationSeverity::Critical,
                            category: ValidationCategory::TypeRegistry,
                            description: "Duplicate TLV type number".to_string(),
                            matched_text: cap.get(0).unwrap().as_str().to_string(),
                            suggestion: format!(
                                "Choose different type number (conflicts with {} at line {})",
                                existing_name, existing_line
                            ),
                            pattern_id: "duplicate-tlv-type".to_string(),
                            context: Some(format!(
                                "Type {} conflicts with {} at line {}",
                                type_num, existing_name, existing_line
                            )),
                        });
                    } else {
                        found_types.insert(type_num, (line_num + 1, cap.get(0).unwrap().as_str().to_string()));
                    }
                }
            }
        }
        
        Ok(findings)
    }
    
    /// Validate precision handling in context
    fn validate_precision_context(
        &self,
        content: &str,
        file_path: &str,
    ) -> Result<Vec<ValidationFinding>, ValidationError> {
        let mut findings = Vec::new();
        
        // Look for price/amount variables with floating point types
        let float_var_regex = Regex::new(r"let\s+(price|amount|balance|quantity)\s*:\s*f(32|64)").unwrap();
        
        for (line_num, line) in content.lines().enumerate() {
            if let Some(mat) = float_var_regex.find(line) {
                findings.push(ValidationFinding {
                    file_path: file_path.to_string(),
                    line_number: line_num + 1,
                    column: mat.start(),
                    severity: ValidationSeverity::Critical,
                    category: ValidationCategory::Precision,
                    description: "Financial data using floating point type".to_string(),
                    matched_text: mat.as_str().to_string(),
                    suggestion: "Use i64 with native precision (DEX) or 8-decimal fixed-point (traditional)".to_string(),
                    pattern_id: "float-financial-data".to_string(),
                    context: Some("Financial data must preserve precision".to_string()),
                });
            }
        }
        
        Ok(findings)
    }
    
    /// Validate multiple files
    pub fn validate_files<P: AsRef<Path>>(&self, file_paths: &[P]) -> Result<Vec<ValidationFinding>, ValidationError> {
        let mut all_findings = Vec::new();
        
        for path in file_paths {
            match self.validate_file(path) {
                Ok(findings) => all_findings.extend(findings),
                Err(e) => {
                    eprintln!("Warning: Failed to validate {}: {}", path.as_ref().display(), e);
                }
            }
        }
        
        Ok(all_findings)
    }
    
    /// Generate validation summary
    pub fn summarize_findings(&self, findings: &[ValidationFinding]) -> ValidationSummary {
        let mut summary = ValidationSummary::new();
        
        for finding in findings {
            summary.total_count += 1;
            
            match finding.severity {
                ValidationSeverity::Critical => summary.critical_count += 1,
                ValidationSeverity::Warning => summary.warning_count += 1,
                ValidationSeverity::Info => summary.info_count += 1,
            }
            
            *summary.by_category.entry(finding.category.clone()).or_insert(0) += 1;
            summary.files.insert(finding.file_path.clone());
        }
        
        summary
    }
}

/// Summary of Protocol V2 validation results
#[derive(Debug)]
pub struct ValidationSummary {
    pub total_count: usize,
    pub critical_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub by_category: HashMap<ValidationCategory, usize>,
    pub files: HashSet<String>,
}

impl ValidationSummary {
    fn new() -> Self {
        Self {
            total_count: 0,
            critical_count: 0,
            warning_count: 0,
            info_count: 0,
            by_category: HashMap::new(),
            files: HashSet::new(),
        }
    }
    
    /// Format summary for display
    pub fn format_summary(&self) -> String {
        format!(
            "🔍 **Protocol V2 Validation Results**\n\
             📊 **Summary**: {} files validated, {} total findings\n\
             🚨 **Critical**: {} violations (must fix)\n\
             ⚠️  **Warnings**: {} issues (should review)\n\
             ℹ️  **Info**: {} suggestions\n\n\
             **By Category**:\n\
             📦 TLV Format: {}\n\
             🎯 Magic Number: {}\n\
             💰 Precision: {}\n\
             🏷️  Type Registry: {}\n\
             ⚙️  Codec: {}\n\
             🏗️  Architecture: {}\n\
             ⚡ Performance: {}",
            self.files.len(),
            self.total_count,
            self.critical_count,
            self.warning_count,
            self.info_count,
            self.by_category.get(&ValidationCategory::TLVFormat).unwrap_or(&0),
            self.by_category.get(&ValidationCategory::MagicNumber).unwrap_or(&0),
            self.by_category.get(&ValidationCategory::Precision).unwrap_or(&0),
            self.by_category.get(&ValidationCategory::TypeRegistry).unwrap_or(&0),
            self.by_category.get(&ValidationCategory::Codec).unwrap_or(&0),
            self.by_category.get(&ValidationCategory::Architecture).unwrap_or(&0),
            self.by_category.get(&ValidationCategory::Performance).unwrap_or(&0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_magic_number_validation() {
        let validator = ProtocolV2Validator::new().unwrap();
        
        // Test file with correct magic number
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "const MESSAGE_MAGIC: u32 = 0xDEADBEEF;").unwrap();
        writeln!(file, "if header.magic == 0xDEADBEEF {{").unwrap();
        
        let findings = validator.validate_file(file.path()).unwrap();
        
        // Should find positive patterns but no violations
        assert!(findings.iter().any(|f| f.pattern_id == "correct-magic"));
        assert!(!findings.iter().any(|f| f.severity == ValidationSeverity::Critical));
    }
    
    #[test]
    fn test_precision_validation() {
        let validator = ProtocolV2Validator::new().unwrap();
        
        // Test file with floating point price (violation)
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "let price: f64 = 123.45;").unwrap();
        writeln!(file, "let amount = price.as_f64();").unwrap();
        
        let findings = validator.validate_file(file.path()).unwrap();
        
        // Should detect precision violations
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.category == ValidationCategory::Precision));
        assert!(findings.iter().any(|f| f.severity == ValidationSeverity::Critical));
    }
    
    #[test]
    fn test_codec_validation() {
        let validator = ProtocolV2Validator::new().unwrap();
        
        // Test file with manual parsing (violation)
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "let header = &bytes[0..32];").unwrap();
        writeln!(file, "let tlv = &payload[2..10];").unwrap();
        
        let findings = validator.validate_file(file.path()).unwrap();
        
        // Should detect manual parsing violations
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.category == ValidationCategory::Codec));
        assert!(findings.iter().any(|f| f.severity == ValidationSeverity::Critical));
    }
    
    #[test]
    fn test_summary_generation() {
        let validator = ProtocolV2Validator::new().unwrap();
        
        let findings = vec![
            ValidationFinding {
                file_path: "test.rs".to_string(),
                line_number: 1,
                column: 0,
                severity: ValidationSeverity::Critical,
                category: ValidationCategory::Precision,
                description: "Test".to_string(),
                matched_text: "f64".to_string(),
                suggestion: "Use i64".to_string(),
                pattern_id: "float-price".to_string(),
                context: None,
            },
            ValidationFinding {
                file_path: "test.rs".to_string(),
                line_number: 2,
                column: 0,
                severity: ValidationSeverity::Warning,
                category: ValidationCategory::Architecture,
                description: "Test".to_string(),
                matched_text: "clone".to_string(),
                suggestion: "Avoid".to_string(),
                pattern_id: "unnecessary-clone".to_string(),
                context: None,
            },
        ];
        
        let summary = validator.summarize_findings(&findings);
        
        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.critical_count, 1);
        assert_eq!(summary.warning_count, 1);
        assert_eq!(summary.info_count, 0);
        assert_eq!(summary.files.len(), 1);
    }
}