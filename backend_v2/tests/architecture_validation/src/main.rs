//! Architecture validation test runner
//!
//! This binary runs all architecture validation tests and provides
//! clear output for CI/CD integration.

use anyhow::Result;
use architecture_validation::run_all_validations;
use clap::Parser;

#[derive(Parser)]
#[command(name = "arch-validate")]
#[command(about = "AlphaPulse architecture validation tool")]
struct Args {
    /// Run only critical violations (blocks deployment)
    #[arg(long)]
    critical_only: bool,
    
    /// Run quick checks only (for pre-commit)
    #[arg(long)]
    quick_check: bool,
    
    /// Output violations to log file
    #[arg(long, value_name = "FILE")]
    output_log: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Set up colored output
    colored::control::set_override(true);

    // Run validation tests based on mode
    let result = if args.critical_only {
        run_critical_validations()
    } else if args.quick_check {
        run_quick_validations()
    } else {
        run_all_validations()
    };

    match result {
        Ok(()) => {
            println!("\n🎉 Architecture validation passed!");
            std::process::exit(0);
        }
        Err(e) => {
            // Log to file if requested
            if let Some(log_file) = args.output_log {
                std::fs::write(&log_file, format!("{}", e))?;
                eprintln!("Violations logged to: {}", log_file);
            }
            
            eprintln!("\n❌ Architecture validation failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_critical_validations() -> Result<()> {
    // Only run violations that should block deployment
    println!("Running critical architecture validations...");
    
    use architecture_validation::*;
    use colored::Colorize;
    
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .context("Failed to load workspace metadata")?;
    
    let mut failed = false;
    
    // Critical validations that MUST pass for deployment
    let critical_tests = vec![
        ("Bijective ID Usage", typed_id_usage::validate_bijective_id_usage(&metadata)),
        ("TLV Protocol Compliance", advanced_validations::validate_tlv_construction_patterns(&metadata)),
        ("Precision Handling", advanced_validations::validate_precision_handling(&metadata)),
        ("Zerocopy Performance", advanced_validations::validate_zerocopy_usage(&metadata)),
    ];
    
    for (name, result) in critical_tests {
        match result {
            Ok(_) => println!("{} {}", "✓".green(), name.green()),
            Err(e) => {
                println!("{} {}", "✗".red(), name.red());
                eprintln!("  {}", e);
                failed = true;
            }
        }
    }
    
    if failed {
        return Err(anyhow::anyhow!("Critical architecture validations failed"));
    }
    
    Ok(())
}

fn run_quick_validations() -> Result<()> {
    // Lightweight checks for pre-commit (fast subset)
    println!("Running quick architecture validations...");
    
    use architecture_validation::*;
    use colored::Colorize;
    
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .context("Failed to load workspace metadata")?;
    
    let mut failed = false;
    
    // Quick validations for pre-commit (most important, fastest)
    let quick_tests = vec![
        ("Codec Usage", dependency_validation::validate_codec_usage(&metadata)),
        ("Service Boundaries", advanced_validations::validate_service_boundaries(&metadata)),
    ];
    
    for (name, result) in quick_tests {
        match result {
            Ok(_) => println!("{} {}", "✓".green(), name.green()),
            Err(e) => {
                println!("{} {}", "✗".red(), name.red());
                eprintln!("  {}", e);
                failed = true;
            }
        }
    }
    
    if failed {
        return Err(anyhow::anyhow!("Quick architecture validations failed"));
    }
    
    Ok(())
}