use anyhow::Result;
use colored::Colorize;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use walkdir::WalkDir;

use crate::cache::SimpleCache;

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub name: String,
    pub item_type: String,
    pub crate_name: String,
    pub module: Option<String>,
    pub file: Option<String>,
    pub visibility: String,
    pub signature: Option<String>,
    pub docs: Option<String>,
    pub similarity: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct CodeExample {
    pub file: String,
    pub code_lines: Vec<String>,
    pub context: String,
}

pub struct QueryEngine {
    cache: SimpleCache,
    fuzzy_matcher: SkimMatcherV2,
}

impl QueryEngine {
    pub fn new(cache: SimpleCache) -> Self {
        Self {
            cache,
            fuzzy_matcher: SkimMatcherV2::default(),
        }
    }

    pub fn init(&self) -> Result<()> {
        println!("Initializing rq for {} project...", "rust".cyan());

        // Just verify we can find crates
        let crates = self.cache.find_crates()?;
        if crates.is_empty() {
            println!("Warning: No Rust crates found in current directory");
        } else {
            println!("Found {} crate(s)", crates.len());
        }

        Ok(())
    }

    pub fn update(&self, crate_name: Option<&str>, force: bool) -> Result<usize> {
        let mut updated = 0;
        let mut failed_crates = Vec::new();

        if let Some(name) = crate_name {
            let crate_path = self.cache.workspace_root().join(name);
            if crate_path.exists() && (force || self.cache.needs_update(&crate_path)?) {
                match self.cache.update_crate(&crate_path) {
                    Ok(_) => updated += 1,
                    Err(e) => failed_crates.push((name.to_string(), e)),
                }
            }
        } else {
            let crates = self.cache.find_crates()?;
            for crate_path in crates {
                let crate_name = crate_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                if force || self.cache.needs_update(&crate_path)? {
                    match self.cache.update_crate(&crate_path) {
                        Ok(_) => updated += 1,
                        Err(e) => failed_crates.push((crate_name.to_string(), e)),
                    }
                }
            }
        }

        // Report failures but don't fail the whole operation
        if !failed_crates.is_empty() {
            eprintln!("⚠️  Failed to update {} crates:", failed_crates.len());
            for (name, error) in failed_crates {
                eprintln!("   • {}: {}", name, error);
            }
        }

        Ok(updated)
    }

    pub fn find(
        &self,
        pattern: &str,
        item_type: Option<&str>,
        module: Option<&str>,
        public_only: bool,
        crate_name: Option<&str>,
        use_regex: bool,
    ) -> Result<Vec<QueryResult>> {
        let mut results = Vec::new();
        let crates = self.cache.find_crates()?;

        for crate_path in crates {
            let current_crate_name = crate_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Filter by crate name if specified
            if let Some(filter) = crate_name {
                if current_crate_name != filter {
                    continue;
                }
            }

            // Try to load rustdoc JSON
            let doc = match self.cache.load_rustdoc(&crate_path) {
                Ok(doc) => doc,
                Err(e) => {
                    // Debug: show which crates are being skipped
                    if std::env::var("RQ_DEBUG").is_ok() {
                        eprintln!("Skipping {} - no rustdoc JSON: {}", crate_path.display(), e);
                    }
                    continue; // Skip crates that don't have rustdoc JSON
                }
            };

            let crate_results = self.search_in_doc(
                &doc,
                current_crate_name,
                pattern,
                item_type,
                module,
                public_only,
                use_regex,
            )?;

            results.extend(crate_results);
        }

        // Sort by name for consistent output
        results.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(results)
    }

    fn search_in_doc(
        &self,
        doc: &Value,
        crate_name: &str,
        pattern: &str,
        item_type_filter: Option<&str>,
        module_filter: Option<&str>,
        public_only: bool,
        use_regex: bool,
    ) -> Result<Vec<QueryResult>> {
        let mut results = Vec::new();

        let empty_map = serde_json::Map::new();
        let index = doc
            .get("index")
            .and_then(|v| v.as_object())
            .unwrap_or(&empty_map);

        // Compile regex if requested
        let regex_pattern = if use_regex && !pattern.is_empty() {
            Some(Regex::new(pattern).map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?)
        } else {
            None
        };

        for (_id, item) in index {
            let item_name = match item.get("name").and_then(|n| n.as_str()) {
                Some(name) => name,
                None => continue,
            };

            // Check if name matches pattern
            let matches = if let Some(ref regex) = regex_pattern {
                regex.is_match(item_name)
            } else if pattern.is_empty() {
                true // Empty pattern matches everything
            } else {
                // Case-insensitive substring match
                item_name.to_lowercase().contains(&pattern.to_lowercase())
            };

            if !matches {
                continue;
            }

            // Extract item type and other metadata
            let (item_type, signature) = self.extract_item_type_and_signature(item);

            // Filter by item type if specified
            if let Some(filter) = item_type_filter {
                if item_type.to_lowercase() != filter.to_lowercase() {
                    continue;
                }
            }

            // Extract visibility
            let visibility = item
                .get("visibility")
                .and_then(|v| v.as_str())
                .unwrap_or("private")
                .to_string();

            // Filter by visibility if requested
            if public_only && visibility != "public" {
                continue;
            }

            // Extract file path
            let file = item
                .get("span")
                .and_then(|s| s.get("filename"))
                .and_then(|f| f.as_str())
                .map(|path| {
                    // Extract just the filename from the full path
                    path.split('/').last().unwrap_or(path).to_string()
                });

            // Extract module (simplified - could be improved)
            let module = self.extract_module_path(item);

            // Filter by module if specified
            if let Some(filter) = module_filter {
                if let Some(ref mod_path) = module {
                    if !mod_path.contains(filter) {
                        continue;
                    }
                } else if !filter.is_empty() {
                    continue;
                }
            }

            // Extract documentation
            let docs = item
                .get("docs")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());

            results.push(QueryResult {
                name: item_name.to_string(),
                item_type,
                crate_name: crate_name.to_string(),
                module,
                file,
                visibility,
                signature,
                docs,
                similarity: None,
            });
        }

        Ok(results)
    }

    fn extract_item_type_and_signature(&self, item: &Value) -> (String, Option<String>) {
        let inner = match item.get("inner").and_then(|v| v.as_object()) {
            Some(inner) => inner,
            None => return ("unknown".to_string(), None),
        };

        // Check each possible type
        if inner.contains_key("struct") {
            ("struct".to_string(), self.extract_struct_signature(item))
        } else if inner.contains_key("enum") {
            ("enum".to_string(), self.extract_enum_signature(item))
        } else if inner.contains_key("function") {
            (
                "function".to_string(),
                self.extract_function_signature(item),
            )
        } else if inner.contains_key("trait") {
            ("trait".to_string(), None)
        } else if inner.contains_key("impl") {
            ("impl".to_string(), None)
        } else if inner.contains_key("module") {
            ("module".to_string(), None)
        } else if inner.contains_key("type_alias") {
            ("type".to_string(), None)
        } else if inner.contains_key("variant") {
            ("variant".to_string(), None)
        } else if inner.contains_key("struct_field") {
            ("field".to_string(), None)
        } else if inner.contains_key("constant") {
            ("const".to_string(), None)
        } else if inner.contains_key("static") {
            ("static".to_string(), None)
        } else if inner.contains_key("macro") {
            ("macro".to_string(), None)
        } else {
            // Debug: show what keys we have
            let keys: Vec<String> = inner.keys().cloned().collect();
            ("unknown".to_string(), Some(format!("keys: {:?}", keys)))
        }
    }

    fn extract_struct_signature(&self, item: &Value) -> Option<String> {
        // Try to extract struct fields for signature
        let fields = item
            .get("inner")?
            .get("struct")?
            .get("kind")?
            .get("plain")?
            .get("fields")?;
        if let Some(field_array) = fields.as_array() {
            Some(format!("struct with {} fields", field_array.len()))
        } else {
            Some("struct".to_string())
        }
    }

    fn extract_enum_signature(&self, item: &Value) -> Option<String> {
        // Try to extract enum variants for signature
        let variants = item.get("inner")?.get("enum")?.get("variants")?;
        if let Some(variant_array) = variants.as_array() {
            Some(format!("enum with {} variants", variant_array.len()))
        } else {
            Some("enum".to_string())
        }
    }

    fn extract_function_signature(&self, _item: &Value) -> Option<String> {
        // TODO: Extract proper function signature from rustdoc JSON
        Some("fn(...)".to_string())
    }

    fn extract_module_path(&self, item: &Value) -> Option<String> {
        // This is simplified - rustdoc JSON has module information but it's complex to extract
        item.get("path")
            .and_then(|p| p.as_str())
            .map(|s| s.to_string())
    }

    pub fn find_similar(&self, pattern: &str, threshold: f32) -> Result<Vec<QueryResult>> {
        // First do a broad search to get all items
        let all_results = self.find("", None, None, false, None, false)?;

        let mut similar_results = Vec::new();

        for result in all_results {
            if let Some(score) = self.fuzzy_matcher.fuzzy_match(&result.name, pattern) {
                let normalized_score = score as f32 / 100.0;
                if normalized_score >= threshold {
                    let mut similar_result = result;
                    similar_result.similarity = Some(normalized_score);
                    similar_results.push(similar_result);
                }
            }
        }

        // Sort by similarity score (highest first)
        similar_results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(similar_results)
    }

    pub fn check_exists(&self, name: &str) -> Result<bool> {
        let results = self.find(name, None, None, false, None, false)?;
        Ok(results.iter().any(|r| r.name == name))
    }

    pub fn search_docs(&self, pattern: &str) -> Result<Vec<QueryResult>> {
        let all_results = self.find("", None, None, false, None, false)?;

        let matching_results: Vec<QueryResult> = all_results
            .into_iter()
            .filter(|result| {
                result
                    .docs
                    .as_ref()
                    .map(|docs| docs.to_lowercase().contains(&pattern.to_lowercase()))
                    .unwrap_or(false)
            })
            .collect();

        Ok(matching_results)
    }

    pub fn get_stats(&self) -> Result<CacheStats> {
        let all_results = self.find("", None, None, false, None, false)?;

        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut by_crate: HashMap<String, usize> = HashMap::new();

        for result in &all_results {
            *by_type.entry(result.item_type.clone()).or_insert(0) += 1;
            *by_crate.entry(result.crate_name.clone()).or_insert(0) += 1;
        }

        Ok(CacheStats {
            total_items: all_results.len(),
            total_crates: by_crate.len(),
            by_type,
            by_crate,
        })
    }

    /// Find usage examples from test files
    pub fn find_examples(&self, name: &str) -> Result<Vec<CodeExample>> {
        let mut examples = Vec::new();
        let crates = self.cache.find_crates()?;

        for crate_path in crates {
            // Look for test files
            let test_dirs = [
                crate_path.join("tests"),
                crate_path.join("src").join("tests"),
                crate_path.join("benches"),
            ];

            for test_dir in &test_dirs {
                if test_dir.exists() {
                    let test_examples = self.search_test_files(test_dir, name)?;
                    examples.extend(test_examples);
                }
            }

            // Also search for test modules in regular files
            let src_dir = crate_path.join("src");
            if src_dir.exists() {
                let src_examples = self.search_rust_files_for_tests(&src_dir, name)?;
                examples.extend(src_examples);
            }
        }

        Ok(examples)
    }

    fn search_test_files(
        &self,
        test_dir: &std::path::Path,
        name: &str,
    ) -> Result<Vec<CodeExample>> {
        let mut examples = Vec::new();

        for entry in WalkDir::new(test_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let file_examples =
                    self.extract_examples_from_content(&content, name, entry.path())?;
                examples.extend(file_examples);
            }
        }

        Ok(examples)
    }

    fn search_rust_files_for_tests(
        &self,
        src_dir: &std::path::Path,
        name: &str,
    ) -> Result<Vec<CodeExample>> {
        let mut examples = Vec::new();

        for entry in WalkDir::new(src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                // Only look in test modules or functions
                if content.contains("#[test]") || content.contains("#[cfg(test)]") {
                    let file_examples =
                        self.extract_examples_from_content(&content, name, entry.path())?;
                    examples.extend(file_examples);
                }
            }
        }

        Ok(examples)
    }

    fn extract_examples_from_content(
        &self,
        content: &str,
        name: &str,
        file_path: &std::path::Path,
    ) -> Result<Vec<CodeExample>> {
        let mut examples = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if line.contains(name) {
                // Extract surrounding context
                let start = i.saturating_sub(2);
                let end = (i + 3).min(lines.len());
                let code_lines: Vec<String> =
                    lines[start..end].iter().map(|s| s.to_string()).collect();

                let context = if line.contains("#[test]")
                    || lines
                        .get(i.saturating_sub(1))
                        .map_or(false, |l| l.contains("#[test]"))
                {
                    "test function".to_string()
                } else if line.contains("assert") {
                    "assertion".to_string()
                } else {
                    "usage".to_string()
                };

                examples.push(CodeExample {
                    file: file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    code_lines,
                    context,
                });
            }
        }

        Ok(examples)
    }

    /// Find functions that call this item (simplified implementation)
    pub fn find_callers(&self, name: &str) -> Result<Vec<QueryResult>> {
        let mut callers = Vec::new();
        let crates = self.cache.find_crates()?;

        for crate_path in crates {
            let src_dir = crate_path.join("src");
            if src_dir.exists() {
                let file_callers = self.search_rust_files_for_usage(&src_dir, name)?;
                callers.extend(file_callers);
            }
        }

        Ok(callers)
    }

    /// Find functions that this item calls (simplified implementation)  
    pub fn find_calls(&self, name: &str) -> Result<Vec<QueryResult>> {
        // This is a simplified implementation
        // For a full implementation, we'd need to parse function bodies from rustdoc JSON
        // or analyze the source code directly

        // For now, return empty - this would require more complex analysis
        Ok(Vec::new())
    }

    fn search_rust_files_for_usage(
        &self,
        src_dir: &std::path::Path,
        name: &str,
    ) -> Result<Vec<QueryResult>> {
        let mut results = Vec::new();

        for entry in WalkDir::new(src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                // Simple heuristic: look for function definitions that mention the name
                for (line_num, line) in content.lines().enumerate() {
                    if (line.contains("fn ") || line.contains("pub fn ")) {
                        // Check if function body contains the name (simplified)
                        if content.contains(name) {
                            if let Some(fn_name) = self.extract_function_name(line) {
                                results.push(QueryResult {
                                    name: fn_name,
                                    item_type: "function".to_string(),
                                    crate_name: entry
                                        .path()
                                        .parent()
                                        .and_then(|p| p.file_name())
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("unknown")
                                        .to_string(),
                                    module: None,
                                    file: Some(
                                        entry
                                            .path()
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("unknown")
                                            .to_string(),
                                    ),
                                    visibility: "unknown".to_string(),
                                    signature: Some(format!("line {}", line_num + 1)),
                                    docs: None,
                                    similarity: None,
                                });
                                break; // Only add each function once per file
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    fn extract_function_name(&self, line: &str) -> Option<String> {
        // Simple function name extraction
        if let Some(start) = line.find("fn ") {
            let after_fn = &line[start + 3..];
            if let Some(end) = after_fn.find('(') {
                let name = after_fn[..end].trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
        None
    }

    /// Generate system overview from crate and module documentation
    pub fn get_system_overview(&self) -> Result<String> {
        let mut overview = String::new();
        let crates = self.cache.find_crates()?;

        overview.push_str("# System Overview\n\n");

        for crate_path in crates {
            let current_crate_name = crate_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Try to load rustdoc JSON
            let doc = match self.cache.load_rustdoc(&crate_path) {
                Ok(doc) => doc,
                Err(e) => {
                    if std::env::var("RQ_DEBUG").is_ok() {
                        eprintln!("Skipping {} for docs search: {}", crate_path.display(), e);
                    }
                    continue;
                }
            };

            // Get the root crate documentation
            if let Some(root_id) = doc.get("root").and_then(|r| r.as_str()) {
                if let Some(root_item) = doc
                    .get("index")
                    .and_then(|i| i.as_object())
                    .and_then(|index| index.get(root_id))
                {
                    if let Some(docs) = root_item.get("docs").and_then(|d| d.as_str()) {
                        overview.push_str(&format!("## {}\n\n", current_crate_name));
                        overview.push_str(docs);
                        overview.push_str("\n\n");
                    }
                }
            }

            // Also get major module documentation
            let empty_map = serde_json::Map::new();
            let index = doc
                .get("index")
                .and_then(|v| v.as_object())
                .unwrap_or(&empty_map);

            let mut modules: Vec<_> = index
                .values()
                .filter(|item| {
                    item.get("inner")
                        .and_then(|inner| inner.as_object())
                        .map(|obj| obj.contains_key("module"))
                        .unwrap_or(false)
                })
                .filter(|item| {
                    item.get("visibility")
                        .and_then(|v| v.as_str())
                        .map(|vis| vis == "public")
                        .unwrap_or(false)
                })
                .filter(|item| {
                    item.get("docs")
                        .and_then(|d| d.as_str())
                        .map(|docs| !docs.trim().is_empty())
                        .unwrap_or(false)
                })
                .collect();

            modules.sort_by_key(|item| item.get("name").and_then(|n| n.as_str()).unwrap_or(""));

            if !modules.is_empty() {
                overview.push_str(&format!("### {} Modules\n\n", current_crate_name));
                for module in modules.into_iter().take(5) {
                    // Limit to 5 modules per crate
                    if let (Some(name), Some(docs)) = (
                        module.get("name").and_then(|n| n.as_str()),
                        module.get("docs").and_then(|d| d.as_str()),
                    ) {
                        overview.push_str(&format!("**{}**: ", name));
                        let first_line = docs.lines().next().unwrap_or(docs);
                        overview.push_str(first_line);
                        overview.push_str("\n\n");
                    }
                }
            }
        }

        if overview == "# System Overview\n\n" {
            overview.push_str("No crate-level documentation found. Consider adding module-level docs (//!) to your lib.rs files.\n");
        }

        Ok(overview)
    }

    /// Find all trait definitions
    pub fn find_traits(&self, pattern: Option<&str>) -> Result<Vec<QueryResult>> {
        let trait_filter = Some("trait");
        self.find(
            pattern.unwrap_or(""),
            trait_filter,
            None,  // module
            false, // public_only (show all traits)
            None,  // crate_name
            false, // use_regex
        )
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_items: usize,
    pub total_crates: usize,
    pub by_type: HashMap<String, usize>,
    pub by_crate: HashMap<String, usize>,
}
