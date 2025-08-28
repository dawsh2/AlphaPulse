//! Simplified workspace dependency validation that doesn't require cargo build
//! This test can run even when there are build failures in the workspace.

use std::collections::{HashMap, HashSet};
use std::fs;

#[test]
fn test_rust_decimal_macros_missing_from_workspace() {
    println!("=== Testing rust_decimal_macros Workspace Dependency ===");
    
    // Load workspace dependencies
    let workspace_deps = load_workspace_dependencies();
    
    // This should fail initially (TDD red phase) - rust_decimal_macros is missing
    assert!(
        workspace_deps.contains_key("rust_decimal_macros"),
        "rust_decimal_macros is missing from [workspace.dependencies] but required by:\n- services_v2/strategies/Cargo.toml\n- libs/amm/Cargo.toml"
    );
    
    println!("✅ rust_decimal_macros is properly defined in workspace dependencies");
}

#[test] 
fn test_workspace_dependencies_exist() {
    println!("=== Testing Workspace Dependencies Section Exists ===");
    
    let workspace_deps = load_workspace_dependencies();
    
    assert!(!workspace_deps.is_empty(), "No workspace dependencies found - workspace section may be malformed");
    
    // Verify some known dependencies exist
    let required_deps = ["tokio", "serde", "anyhow", "thiserror"];
    
    for dep in required_deps {
        assert!(
            workspace_deps.contains_key(dep),
            "Required workspace dependency '{}' is missing",
            dep
        );
    }
    
    println!("✅ Workspace dependencies section properly configured");
    println!("Found {} workspace dependencies", workspace_deps.len());
}

#[test]
fn test_members_with_workspace_true_dependencies() {
    println!("=== Finding Members That Use workspace = true ===");
    
    let members = get_workspace_members();
    let mut members_with_workspace_deps = Vec::new();
    
    for member in &members {
        let cargo_path = format!("/Users/daws/alphapulse/backend_v2/{}/Cargo.toml", member);
        if let Ok(content) = fs::read_to_string(&cargo_path) {
            if content.contains("workspace = true") {
                members_with_workspace_deps.push(member);
            }
        }
    }
    
    assert!(!members_with_workspace_deps.is_empty(), "No workspace members use workspace = true - this seems wrong");
    
    println!("✅ Found {} members using workspace dependencies:", members_with_workspace_deps.len());
    for member in &members_with_workspace_deps {
        println!("  - {}", member);
    }
}

// Helper functions
fn load_workspace_dependencies() -> HashMap<String, String> {
    let cargo_toml = fs::read_to_string("/Users/daws/alphapulse/backend_v2/Cargo.toml")
        .expect("Failed to read root Cargo.toml");
    
    let parsed: toml::Value = toml::from_str(&cargo_toml)
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

fn get_workspace_members() -> Vec<String> {
    let cargo_toml = fs::read_to_string("/Users/daws/alphapulse/backend_v2/Cargo.toml")
        .expect("Failed to read root Cargo.toml");
    
    let parsed: toml::Value = toml::from_str(&cargo_toml)
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