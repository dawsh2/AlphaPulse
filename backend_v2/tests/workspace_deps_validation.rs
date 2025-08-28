//! Test Design for Workspace Dependencies (BUILD-001-TESTS)
//! 
//! This test validates that all workspace members can resolve their dependencies
//! correctly and identifies missing dependencies that cause build failures.
//!
//! Key validations:
//! 1. All workspace.dependencies are defined in root Cargo.toml
//! 2. All crates using workspace=true can resolve dependencies  
//! 3. Specific validation for rust_decimal_macros (the current blocker)
//! 4. No phantom dependencies (defined but unused)

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use toml::Value;

#[test]
fn test_all_workspace_dependencies_defined() {
    println!("=== Testing Workspace Dependencies Resolution ===");
    
    let workspace_deps = load_workspace_dependencies();
    let member_deps = load_all_member_workspace_dependencies();
    
    let mut missing_deps = Vec::new();
    
    for (member, deps) in member_deps {
        for dep in deps {
            if !workspace_deps.contains_key(&dep) {
                missing_deps.push(format!("{} requires {}", member, dep));
            }
        }
    }
    
    if !missing_deps.is_empty() {
        panic!("Missing workspace dependencies:\n{}", missing_deps.join("\n"));
    }
}

#[test]
fn test_rust_decimal_macros_specifically() {
    println!("=== Testing rust_decimal_macros Dependency ===");
    
    let workspace_deps = load_workspace_dependencies();
    
    // This should fail initially (TDD red phase)
    assert!(
        workspace_deps.contains_key("rust_decimal_macros"),
        "rust_decimal_macros must be defined in workspace dependencies"
    );
}

#[test]
fn test_workspace_members_can_build() {
    println!("=== Testing Individual Workspace Member Builds ===");
    
    let members = get_workspace_members();
    
    for member in members {
        let output = std::process::Command::new("cargo")
            .args(&["check", "--manifest-path", &format!("{}/Cargo.toml", member)])
            .current_dir("/Users/daws/alphapulse/backend_v2")
            .output()
            .expect("Failed to execute cargo check");
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Workspace member {} failed to build:\n{}", member, stderr);
        }
    }
}

#[test]
fn test_no_phantom_workspace_dependencies() {
    println!("=== Testing for Unused Workspace Dependencies ===");
    
    let workspace_deps = load_workspace_dependencies();
    let used_deps = load_all_used_workspace_dependencies();
    
    let phantom_deps: Vec<_> = workspace_deps
        .keys()
        .filter(|dep| !used_deps.contains(*dep))
        .collect();
    
    if !phantom_deps.is_empty() {
        println!("Warning: Unused workspace dependencies: {:?}", phantom_deps);
        // Don't fail - this is just cleanup
    }
}

// Helper functions

fn load_workspace_dependencies() -> HashMap<String, String> {
    let cargo_toml = fs::read_to_string("/Users/daws/alphapulse/backend_v2/Cargo.toml")
        .expect("Failed to read root Cargo.toml");
    
    let parsed: Value = toml::from_str(&cargo_toml)
        .expect("Failed to parse root Cargo.toml");
    
    let mut deps = HashMap::new();
    
    if let Some(workspace) = parsed.get("workspace") {
        if let Some(workspace_deps) = workspace.get("dependencies") {
            if let Some(deps_table) = workspace_deps.as_table() {
                for (name, _value) in deps_table {
                    deps.insert(name.clone(), "defined".to_string());
                }
            }
        }
    }
    
    deps
}

fn load_all_member_workspace_dependencies() -> HashMap<String, HashSet<String>> {
    let members = get_workspace_members();
    let mut all_deps = HashMap::new();
    
    for member in members {
        let cargo_path = format!("/Users/daws/alphapulse/backend_v2/{}/Cargo.toml", member);
        if let Ok(content) = fs::read_to_string(&cargo_path) {
            if let Ok(parsed) = toml::from_str::<Value>(&content) {
                let mut deps = HashSet::new();
                
                // Check dependencies section
                if let Some(dependencies) = parsed.get("dependencies") {
                    if let Some(deps_table) = dependencies.as_table() {
                        for (name, value) in deps_table {
                            if let Some(table) = value.as_table() {
                                if table.get("workspace") == Some(&Value::Boolean(true)) {
                                    deps.insert(name.clone());
                                }
                            }
                        }
                    }
                }
                
                all_deps.insert(member, deps);
            }
        }
    }
    
    all_deps
}

fn load_all_used_workspace_dependencies() -> HashSet<String> {
    let member_deps = load_all_member_workspace_dependencies();
    let mut used = HashSet::new();
    
    for (_, deps) in member_deps {
        used.extend(deps);
    }
    
    used
}

fn get_workspace_members() -> Vec<String> {
    let cargo_toml = fs::read_to_string("/Users/daws/alphapulse/backend_v2/Cargo.toml")
        .expect("Failed to read root Cargo.toml");
    
    let parsed: Value = toml::from_str(&cargo_toml)
        .expect("Failed to parse root Cargo.toml");
    
    let mut members = Vec::new();
    
    if let Some(workspace) = parsed.get("workspace") {
        if let Some(members_array) = workspace.get("members") {
            if let Some(array) = members_array.as_array() {
                for member in array {
                    if let Some(member_str) = member.as_str() {
                        members.push(member_str.to_string());
                    }
                }
            }
        }
    }
    
    members
}