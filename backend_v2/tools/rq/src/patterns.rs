//! Architectural pattern detection for rq
//!
//! This module provides specialized queries for detecting architectural patterns
//! and anti-patterns in the codebase.

use crate::query::QueryEngine;
use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;

/// Architectural patterns to detect
pub struct PatternDetector<'a> {
    engine: &'a QueryEngine,
}

impl<'a> PatternDetector<'a> {
    pub fn new(engine: &'a QueryEngine) -> Self {
        Self { engine }
    }

    /// Find references to canonical ABI implementation
    pub fn find_canonical_abi(&self) -> Result<()> {
        println!("{}", "🔍 Canonical ABI References:".bold().green());
        println!("  Finding all uses of libs/dex for event decoding...\n");

        // Search for alphapulse_dex usage
        let results = self.engine.find("alphapulse_dex", None, None, false, None, false)?;
        
        if results.is_empty() {
            println!("  {} No references to canonical ABI found!", "⚠️".yellow());
            println!("  Consider using alphapulse_dex for event decoding.");
        } else {
            let count = results.len();
            for result in results {
                println!("  ✓ {} in {}", 
                    result.name.green(), 
                    result.module.as_deref().unwrap_or("").dimmed());
                    
                if let Some(ref docs) = result.docs {
                    if docs.contains("event") || docs.contains("decode") {
                        println!("    → {}", docs.lines().next().unwrap_or("").dimmed());
                    }
                }
            }
            println!("\n  {} references found to canonical ABI", count);
        }
        
        // Also check for old/duplicate ABI patterns
        self.check_duplicate_abi()?;
        
        Ok(())
    }

    /// Find unified collector patterns (direct relay integration)
    pub fn find_unified_collectors(&self) -> Result<()> {
        println!("{}", "🔍 Unified Collector Pattern:".bold().green());
        println!("  Finding collectors with direct relay integration...\n");

        // Search for RelayOutput usage (indicates unified pattern)
        let relay_outputs = self.engine.find("RelayOutput", Some("struct"), None, false, None, false)?;
        let relay_usage = self.engine.find("relay_output", None, None, false, None, false)?;
        
        println!("  {} Direct Relay Integration:", "✓".green());
        for result in relay_usage.iter().take(5) {
            if result.module.as_deref().unwrap_or("").contains("bin/") {
                println!("    • {} (unified collector)", 
                    result.module.as_deref().unwrap_or("").replace("alphapulse_adapter_service::", "").green());
            }
        }
        
        // Check for legacy patterns
        self.check_legacy_collectors()?;
        
        Ok(())
    }

    /// Check for MPSC channel usage (anti-pattern)
    pub fn check_mpsc_usage(&self) -> Result<()> {
        println!("{}", "🔍 MPSC Channel Detection:".bold().yellow());
        println!("  Checking for MPSC usage (architectural anti-pattern)...\n");

        // Search for mpsc usage
        let mpsc_imports = self.engine.find("mpsc", None, None, false, None, false)?;
        let channel_usage = self.engine.find("channel", Some("function"), None, false, None, false)?;
        
        let mut mpsc_locations = HashSet::new();
        
        for result in &mpsc_imports {
            if result.name.contains("mpsc") || result.module.as_deref().unwrap_or("").contains("mpsc") {
                mpsc_locations.insert(result.module.clone().unwrap_or_default());
            }
        }
        
        for result in &channel_usage {
            if result.name.contains("channel") || result.name.contains("unbounded") {
                mpsc_locations.insert(result.module.clone().unwrap_or_default());
            }
        }
        
        if mpsc_locations.is_empty() {
            println!("  {} No MPSC usage detected - architecture is clean!", "✅".green());
        } else {
            println!("  {} MPSC usage found in {} locations:", "⚠️".yellow(), mpsc_locations.len());
            for location in mpsc_locations.iter().take(10) {
                // Filter out acceptable uses
                if location.contains("dashboard") || location.contains("websocket_server") {
                    println!("    • {} {}", location.dimmed(), "(OK - client management)".green());
                } else if location.contains("test") {
                    println!("    • {} {}", location.dimmed(), "(OK - test code)".dimmed());
                } else {
                    println!("    • {} {}", location.red(), "❌ Consider removing!".red());
                }
            }
            
            println!("\n  💡 MPSC adds latency - use direct relay integration instead");
        }
        
        Ok(())
    }

    /// Check for duplicate ABI implementations
    fn check_duplicate_abi(&self) -> Result<()> {
        println!("\n  {} Checking for duplicate ABI implementations...", "🔎".blue());
        
        // Look for potential duplicate event definitions
        let event_structs = self.engine.find("Event", Some("struct"), None, false, None, false)?;
        let swap_events = self.engine.find("Swap", Some("struct"), None, false, None, false)?;
        
        let mut abi_modules = HashSet::new();
        for result in event_structs.iter().chain(swap_events.iter()) {
            if result.name.contains("Event") || result.name.contains("Swap") {
                // Extract module base
                if let Some(ref module_str) = result.module {
                    let module = module_str.split("::").take(2).collect::<Vec<_>>().join("::");
                    if !module.contains("test") {
                        abi_modules.insert(module);
                    }
                }
            }
        }
        
        if abi_modules.len() > 1 {
            println!("    {} Multiple ABI modules detected:", "⚠️".yellow());
            for module in &abi_modules {
                if module.contains("alphapulse_dex") || module.contains("libs::dex") {
                    println!("      • {} ✓", module.green());
                } else {
                    println!("      • {} (consider removing)", module.yellow());
                }
            }
        } else if abi_modules.len() == 1 && abi_modules.iter().next().unwrap().contains("dex") {
            println!("    {} Single canonical ABI source", "✓".green());
        }
        
        Ok(())
    }

    /// Check for legacy collector patterns
    fn check_legacy_collectors(&self) -> Result<()> {
        println!("\n  {} Checking for legacy patterns...", "🔎".blue());
        
        // Look for old collector/publisher separation
        let publishers = self.engine.find("publisher", None, None, false, None, false)?;
        let collectors = self.engine.find("collector", Some("struct"), None, false, None, false)?;
        
        let mut legacy_count = 0;
        for result in &publishers {
            if result.name.to_lowercase().contains("publisher") && 
               !result.module.as_deref().unwrap_or("").contains("bin/") {
                println!("    {} Legacy publisher: {}", "⚠️".yellow(), result.name);
                legacy_count += 1;
            }
        }
        
        if legacy_count == 0 {
            println!("    {} No legacy publisher/collector separation found", "✓".green());
        } else {
            println!("    Found {} legacy patterns - consider unifying", legacy_count);
        }
        
        Ok(())
    }

    /// Generate architectural health report
    pub fn health_check(&self) -> Result<()> {
        println!("{}", "=" .repeat(60));
        println!("{}", "🏥 Architectural Health Check".bold().cyan());
        println!("{}", "=" .repeat(60));
        
        println!("\n{}", "1. ABI Implementation".bold());
        self.find_canonical_abi()?;
        
        println!("\n{}", "2. Collector Architecture".bold());
        self.find_unified_collectors()?;
        
        println!("\n{}", "3. Anti-Patterns".bold());
        self.check_mpsc_usage()?;
        
        println!("\n{}", "=" .repeat(60));
        println!("{}", "📊 Summary:".bold());
        println!("  Use 'rq patterns --fix' to see remediation suggestions");
        println!("{}", "=" .repeat(60));
        
        Ok(())
    }
}

/// Quick pattern checks for common queries
pub fn quick_check(engine: &QueryEngine, pattern: &str) -> Result<()> {
    match pattern.to_lowercase().as_str() {
        "canonical abi" | "canonical_abi" => {
            let detector = PatternDetector::new(engine);
            detector.find_canonical_abi()
        }
        "unified collector" | "unified_collector" | "direct relay" => {
            let detector = PatternDetector::new(engine);
            detector.find_unified_collectors()
        }
        "mpsc" | "channels" => {
            let detector = PatternDetector::new(engine);
            detector.check_mpsc_usage()
        }
        "health" | "architectural health" => {
            let detector = PatternDetector::new(engine);
            detector.health_check()
        }
        _ => {
            println!("Unknown architectural pattern: {}", pattern);
            println!("Available patterns:");
            println!("  • canonical abi    - Find canonical ABI usage");
            println!("  • unified collector - Find direct relay patterns");
            println!("  • mpsc             - Check for MPSC anti-patterns");
            println!("  • health           - Full architectural health check");
            Ok(())
        }
    }
}