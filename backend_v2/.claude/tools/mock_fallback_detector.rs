//! Mock and Fallback Pattern Detection for Torq
//!
//! Zero-tolerance detection system for patterns that violate Torq's
//! "NO MOCKS EVER" policy and fail-fast philosophy. This tool scans code
//! for mock usage, error swallowing, and fallback patterns that could
//! hide system failures or provide deceptive behavior.

use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DetectionError {
    #[error("Failed to read file {file}: {error}")]
    FileReadError { file: String, error: String },
    
    #[error("Failed to compile regex pattern: {error}")]
    RegexError { error: String },
    
    #[error("Invalid pattern configuration")]
    ConfigError,
}

/// Severity levels for detected patterns
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    /// Critical violations that must be fixed (mocks, error swallowing)
    Critical,
    /// Warning patterns that should be reviewed (potential fallbacks)
    Warning,
    /// Informational patterns for awareness
    Info,
}

/// Category of detected pattern
#[derive(Debug, Clone, PartialEq)]
pub enum Category {
    /// Mock usage patterns
    Mock,
    /// Error swallowing and fallback patterns
    Fallback,
    /// Security-related patterns
    Security,
    /// Policy violations
    Policy,
}

/// A detected pattern match in source code
#[derive(Debug, Clone)]
pub struct Detection {
    /// File path where pattern was found
    pub file_path: String,
    /// Line number (1-based)
    pub line_number: usize,
    /// Column position (0-based)
    pub column: usize,
    /// Severity level of this detection
    pub severity: Severity,
    /// Category of the pattern
    pub category: Category,
    /// Description of what was detected
    pub description: String,
    /// The actual text that matched
    pub matched_text: String,
    /// Suggestion for fixing the issue
    pub suggestion: String,
    /// Pattern that triggered this detection
    pub pattern_id: String,
}

/// Pattern definition for detection
#[derive(Debug, Clone)]
pub struct DetectionPattern {
    /// Unique identifier for this pattern
    pub id: String,
    /// Regex pattern to match
    pub regex: Regex,
    /// Severity of matches
    pub severity: Severity,
    /// Category of the pattern
    pub category: Category,
    /// Human-readable description
    pub description: String,
    /// Suggestion for fixing
    pub suggestion: String,
    /// File extensions this pattern applies to (empty = all files)
    pub file_extensions: Vec<String>,
}

/// Mock and fallback pattern detector
pub struct MockFallbackDetector {
    /// All detection patterns organized by category
    patterns: HashMap<Category, Vec<DetectionPattern>>,
}

impl MockFallbackDetector {
    /// Create a new detector with Torq-specific patterns
    pub fn new() -> Result<Self, DetectionError> {
        let mut detector = Self {
            patterns: HashMap::new(),
        };
        
        detector.load_mock_patterns()?;
        detector.load_fallback_patterns()?;
        detector.load_security_patterns()?;
        detector.load_policy_patterns()?;
        
        Ok(detector)
    }
    
    /// Load mock detection patterns (CRITICAL - violates NO MOCKS policy)
    fn load_mock_patterns(&mut self) -> Result<(), DetectionError> {
        let patterns = vec![
            // Direct mock usage
            ("mock-usage-direct", r"(?i)\bmock\w*\b", "Direct mock usage detected", "Replace with real implementation"),
            ("fake-usage", r"(?i)\bfake\w*\b", "Fake implementation detected", "Replace with real implementation"),
            ("stub-usage", r"(?i)\bstub\w*\b", "Stub implementation detected", "Replace with real implementation"),
            ("test-exchange", r"(?i)\btest\w*exchange\b", "Test exchange detected", "Replace with real exchange connection"),
            ("mock-exchange", r"(?i)\bmock\w*exchange\b", "Mock exchange detected", "Replace with real exchange connection"),
            
            // Test attributes with mock
            ("test-cfg-mock", r"#\[cfg\(test\)\].*(?i)mock", "Mock in test configuration", "Remove mock from production code"),
            ("test-attr-mock", r"#\[test\].*(?i)mock", "Mock in test attribute", "Remove mock from production code"),
            
            // Mock libraries
            ("mockall-import", r"use.*mockall", "Mockall library usage", "Remove mock library, use real implementations"),
            ("mockito-import", r"use.*mockito", "Mockito library usage", "Remove mock library, use real implementations"),
            ("wiremock-import", r"use.*wiremock", "WireMock library usage", "Remove mock library, use real implementations"),
            
            // Mock struct definitions
            ("mock-struct", r"struct.*(?i)Mock\w+", "Mock struct definition", "Replace with real implementation"),
            ("mock-impl", r"impl.*(?i)Mock\w+", "Mock implementation", "Replace with real implementation"),
            
            // Method names suggesting mocks
            ("mock-method", r"fn.*(?i)mock_", "Mock method detected", "Replace with real method"),
            ("fake-method", r"fn.*(?i)fake_", "Fake method detected", "Replace with real method"),
            ("test-exchange-method", r"fn.*(?i)test_\w*exchange", "Test exchange method", "Replace with real exchange method"),
        ];
        
        let mut mock_patterns = Vec::new();
        for (id, pattern, desc, suggestion) in patterns {
            mock_patterns.push(DetectionPattern {
                id: id.to_string(),
                regex: Regex::new(pattern).map_err(|e| DetectionError::RegexError { 
                    error: e.to_string() 
                })?,
                severity: Severity::Critical,
                category: Category::Mock,
                description: desc.to_string(),
                suggestion: suggestion.to_string(),
                file_extensions: vec!["rs".to_string(), "py".to_string()],
            });
        }
        
        self.patterns.insert(Category::Mock, mock_patterns);
        Ok(())
    }
    
    /// Load fallback detection patterns (CRITICAL - violates fail-fast philosophy)
    fn load_fallback_patterns(&mut self) -> Result<(), DetectionError> {
        let patterns = vec![
            // Unwrap_or patterns (fail-fast violations)
            ("unwrap-or", r"\.unwrap_or\(", "Fallback pattern detected", "Remove fallback, let system fail-fast"),
            ("unwrap-or-else", r"\.unwrap_or_else\(", "Fallback pattern detected", "Remove fallback, let system fail-fast"),
            ("unwrap-or-default", r"\.unwrap_or_default\(", "Default fallback detected", "Remove default fallback"),
            
            // Error swallowing
            ("error-swallow-if", r"if.*\.is_err\(\).*\{.*return.*Ok\(", "Error swallowing detected", "Propagate error instead of swallowing"),
            ("error-swallow-match", r"match.*\{.*Err\(_\).*=>.*Ok\(", "Error swallowing in match", "Propagate error instead of swallowing"),
            ("error-swallow-unwrap-or", r"\.unwrap_or_else\(\|\|.*\{.*default.*\}\)", "Error swallowing with default", "Propagate error instead of defaulting"),
            
            // Default fallbacks
            ("default-fallback", r"\.unwrap_or\(.*default.*\)", "Default value fallback", "Remove default fallback"),
            ("zero-fallback", r"\.unwrap_or\(0\)", "Zero fallback detected", "Remove zero fallback"),
            ("empty-fallback", r"\.unwrap_or\(\s*\"\"\s*\)", "Empty string fallback", "Remove empty fallback"),
            
            // Panic prevention (should crash instead)
            ("catch-unwind", r"catch_unwind", "Panic catching detected", "Remove panic catching, let system crash"),
            ("panic-hook", r"panic::set_hook", "Panic hook detected", "Remove panic prevention"),
            ("std-catch-unwind", r"std::panic::catch_unwind", "Standard panic catching", "Remove panic catching"),
            
            // Option/Result hiding
            ("option-unwrap-or", r"Option.*\.unwrap_or", "Option fallback", "Handle None case explicitly or propagate"),
            ("result-unwrap-or", r"Result.*\.unwrap_or", "Result fallback", "Handle error case explicitly or propagate"),
        ];
        
        let mut fallback_patterns = Vec::new();
        for (id, pattern, desc, suggestion) in patterns {
            fallback_patterns.push(DetectionPattern {
                id: id.to_string(),
                regex: Regex::new(pattern).map_err(|e| DetectionError::RegexError { 
                    error: e.to_string() 
                })?,
                severity: Severity::Critical,
                category: Category::Fallback,
                description: desc.to_string(),
                suggestion: suggestion.to_string(),
                file_extensions: vec!["rs".to_string()],
            });
        }
        
        self.patterns.insert(Category::Fallback, fallback_patterns);
        Ok(())
    }
    
    /// Load security-related patterns
    fn load_security_patterns(&mut self) -> Result<(), DetectionError> {
        let patterns = vec![
            // Hardcoded credentials
            ("hardcoded-password", r#"(?i)(password|secret|key|token|api_key)\s*=\s*"[^"]{3,}""#, "Hardcoded credential", "Use environment variable or secure config"),
            ("hardcoded-auth", r#"(?i)(password|secret|key|token):\s*"[^"]{3,}""#, "Hardcoded auth value", "Use environment variable or secure config"),
            
            // Suspicious hardcoded values
            ("bearer-token", r#"(?i)(auth|bearer|basic)\s*"[^"]{10,}""#, "Hardcoded auth token", "Use environment variable"),
            ("todo-credentials", r"(?i)todo.*(?i)(password|key|secret)", "TODO with credentials", "Implement secure credential handling"),
            ("fixme-credentials", r"(?i)fixme.*(?i)(password|key|secret)", "FIXME with credentials", "Implement secure credential handling"),
            
            // Unsafe patterns without justification
            ("unsafe-no-safety", r"unsafe\s*\{[^}]*\}(?!\s*//.*SAFETY)", "Unsafe block without SAFETY comment", "Add SAFETY comment explaining why unsafe is necessary"),
            ("transmute-no-safety", r"transmute(?!\s*//.*SAFETY)", "Transmute without SAFETY comment", "Add SAFETY comment or use safe alternative"),
        ];
        
        let mut security_patterns = Vec::new();
        for (id, pattern, desc, suggestion) in patterns {
            security_patterns.push(DetectionPattern {
                id: id.to_string(),
                regex: Regex::new(pattern).map_err(|e| DetectionError::RegexError { 
                    error: e.to_string() 
                })?,
                severity: Severity::Critical,
                category: Category::Security,
                description: desc.to_string(),
                suggestion: suggestion.to_string(),
                file_extensions: vec!["rs".to_string(), "py".to_string()],
            });
        }
        
        self.patterns.insert(Category::Security, security_patterns);
        Ok(())
    }
    
    /// Load Torq policy patterns
    fn load_policy_patterns(&mut self) -> Result<(), DetectionError> {
        let patterns = vec![
            // Unwrap patterns (should be used carefully)
            ("unwrap-no-context", r"\.unwrap\(\)(?!\s*//.*SAFETY|//.*VERIFIED)", "Unwrap without context", "Add safety comment or use expect() with context"),
            ("expect-no-context", r"\.expect\([^)]*\)(?!\s*//.*CONTEXT)", "Expect without context comment", "Add context comment explaining why panic is acceptable"),
            
            // Production TODOs/FIXMEs
            ("todo-production", r"(?i)todo(?!\s*test)", "TODO in production code", "Complete TODO or create tracking task"),
            ("fixme-production", r"(?i)fixme(?!\s*test)", "FIXME in production code", "Fix issue or create tracking task"),
            
            // Deprecated patterns
            ("deprecated-usage", r"#\[deprecated\]", "Deprecated API usage", "Update to current API"),
        ];
        
        let mut policy_patterns = Vec::new();
        for (id, pattern, desc, suggestion) in patterns {
            policy_patterns.push(DetectionPattern {
                id: id.to_string(),
                regex: Regex::new(pattern).map_err(|e| DetectionError::RegexError { 
                    error: e.to_string() 
                })?,
                severity: Severity::Warning,
                category: Category::Policy,
                description: desc.to_string(),
                suggestion: suggestion.to_string(),
                file_extensions: vec!["rs".to_string(), "py".to_string()],
            });
        }
        
        self.patterns.insert(Category::Policy, policy_patterns);
        Ok(())
    }
    
    /// Scan a single file for mock and fallback patterns
    pub fn scan_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<Detection>, DetectionError> {
        let path = file_path.as_ref();
        let path_str = path.to_string_lossy().to_string();
        
        // Get file extension
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Read file content
        let content = fs::read_to_string(path)
            .map_err(|e| DetectionError::FileReadError {
                file: path_str.clone(),
                error: e.to_string(),
            })?;
        
        let mut detections = Vec::new();
        
        // Scan with all applicable patterns
        for patterns in self.patterns.values() {
            for pattern in patterns {
                // Check if pattern applies to this file type
                if !pattern.file_extensions.is_empty() && 
                   !pattern.file_extensions.contains(&extension) {
                    continue;
                }
                
                detections.extend(self.scan_content_with_pattern(&content, &path_str, pattern)?);
            }
        }
        
        Ok(detections)
    }
    
    /// Scan content with a specific pattern
    fn scan_content_with_pattern(
        &self,
        content: &str,
        file_path: &str,
        pattern: &DetectionPattern,
    ) -> Result<Vec<Detection>, DetectionError> {
        let mut detections = Vec::new();
        
        for (line_num, line) in content.lines().enumerate() {
            for mat in pattern.regex.find_iter(line) {
                detections.push(Detection {
                    file_path: file_path.to_string(),
                    line_number: line_num + 1,
                    column: mat.start(),
                    severity: pattern.severity.clone(),
                    category: pattern.category.clone(),
                    description: pattern.description.clone(),
                    matched_text: mat.as_str().to_string(),
                    suggestion: pattern.suggestion.clone(),
                    pattern_id: pattern.id.clone(),
                });
            }
        }
        
        Ok(detections)
    }
    
    /// Scan multiple files and return aggregated results
    pub fn scan_files<P: AsRef<Path>>(&self, file_paths: &[P]) -> Result<Vec<Detection>, DetectionError> {
        let mut all_detections = Vec::new();
        
        for path in file_paths {
            match self.scan_file(path) {
                Ok(detections) => all_detections.extend(detections),
                Err(e) => {
                    eprintln!("Warning: Failed to scan {}: {}", path.as_ref().display(), e);
                }
            }
        }
        
        Ok(all_detections)
    }
    
    /// Get detection summary by category and severity
    pub fn summarize_detections(&self, detections: &[Detection]) -> DetectionSummary {
        let mut summary = DetectionSummary::new();
        
        for detection in detections {
            summary.total_count += 1;
            
            match detection.severity {
                Severity::Critical => summary.critical_count += 1,
                Severity::Warning => summary.warning_count += 1,
                Severity::Info => summary.info_count += 1,
            }
            
            *summary.by_category.entry(detection.category.clone()).or_insert(0) += 1;
            summary.files.insert(detection.file_path.clone());
        }
        
        summary
    }
}

/// Summary of detection results
#[derive(Debug)]
pub struct DetectionSummary {
    pub total_count: usize,
    pub critical_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub by_category: HashMap<Category, usize>,
    pub files: std::collections::HashSet<String>,
}

impl DetectionSummary {
    fn new() -> Self {
        Self {
            total_count: 0,
            critical_count: 0,
            warning_count: 0,
            info_count: 0,
            by_category: HashMap::new(),
            files: std::collections::HashSet::new(),
        }
    }
    
    /// Format summary for display
    pub fn format_summary(&self) -> String {
        format!(
            "🔍 **Mock/Fallback Detection Results**\n\
             📊 **Summary**: {} files scanned, {} total detections\n\
             🚨 **Critical**: {} violations (must fix)\n\
             ⚠️  **Warnings**: {} issues (should review)\n\
             ℹ️  **Info**: {} suggestions\n\n\
             **By Category**:\n\
             🎭 Mock violations: {}\n\
             🔄 Fallback violations: {}\n\
             🔒 Security issues: {}\n\
             📝 Policy violations: {}",
            self.files.len(),
            self.total_count,
            self.critical_count,
            self.warning_count,
            self.info_count,
            self.by_category.get(&Category::Mock).unwrap_or(&0),
            self.by_category.get(&Category::Fallback).unwrap_or(&0),
            self.by_category.get(&Category::Security).unwrap_or(&0),
            self.by_category.get(&Category::Policy).unwrap_or(&0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_mock_detection() {
        let detector = MockFallbackDetector::new().unwrap();
        
        // Create test file with mock usage
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "use mockall::predicate::*;").unwrap();
        writeln!(file, "struct MockExchange {{}}").unwrap();
        writeln!(file, "fn mock_get_price() -> f64 {{ 100.0 }}").unwrap();
        
        let detections = detector.scan_file(file.path()).unwrap();
        
        // Should detect mock usage
        assert!(!detections.is_empty());
        assert!(detections.iter().any(|d| d.category == Category::Mock));
        assert!(detections.iter().any(|d| d.severity == Severity::Critical));
    }
    
    #[test]
    fn test_fallback_detection() {
        let detector = MockFallbackDetector::new().unwrap();
        
        // Create test file with fallback patterns
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "let price = get_price().unwrap_or(0.0);").unwrap();
        writeln!(file, "match result {{ Err(_) => Ok(()), _ => result }}").unwrap();
        
        let detections = detector.scan_file(file.path()).unwrap();
        
        // Should detect fallback patterns
        assert!(!detections.is_empty());
        assert!(detections.iter().any(|d| d.category == Category::Fallback));
        assert!(detections.iter().any(|d| d.severity == Severity::Critical));
    }
    
    #[test]
    fn test_security_detection() {
        let detector = MockFallbackDetector::new().unwrap();
        
        // Create test file with security issues
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"let password = "hardcoded_password123";"#).unwrap();
        writeln!(file, "unsafe {{ transmute(ptr) }}").unwrap();
        
        let detections = detector.scan_file(file.path()).unwrap();
        
        // Should detect security issues
        assert!(!detections.is_empty());
        assert!(detections.iter().any(|d| d.category == Category::Security));
        assert!(detections.iter().any(|d| d.severity == Severity::Critical));
    }
    
    #[test]
    fn test_summary_generation() {
        let detector = MockFallbackDetector::new().unwrap();
        
        let detections = vec![
            Detection {
                file_path: "test.rs".to_string(),
                line_number: 1,
                column: 0,
                severity: Severity::Critical,
                category: Category::Mock,
                description: "Test".to_string(),
                matched_text: "mock".to_string(),
                suggestion: "Fix".to_string(),
                pattern_id: "test".to_string(),
            },
            Detection {
                file_path: "test.rs".to_string(),
                line_number: 2,
                column: 0,
                severity: Severity::Warning,
                category: Category::Policy,
                description: "Test".to_string(),
                matched_text: "todo".to_string(),
                suggestion: "Fix".to_string(),
                pattern_id: "test".to_string(),
            },
        ];
        
        let summary = detector.summarize_detections(&detections);
        
        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.critical_count, 1);
        assert_eq!(summary.warning_count, 1);
        assert_eq!(summary.info_count, 0);
        assert_eq!(summary.files.len(), 1);
    }
}