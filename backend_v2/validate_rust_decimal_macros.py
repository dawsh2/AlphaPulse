#!/usr/bin/env python3
"""
Simple validation that rust_decimal_macros workspace dependency is properly configured.
This runs independent of cargo to validate our BUILD-001 fix.
"""

import toml
import sys
import os

def load_toml(path):
    try:
        with open(path, 'r') as f:
            return toml.load(f)
    except Exception as e:
        print(f"Error loading {path}: {e}")
        return None

def main():
    base_path = "/Users/daws/alphapulse/backend_v2"
    
    print("🔍 BUILD-001 Validation: rust_decimal_macros Workspace Dependency")
    print("=" * 70)
    
    # Check workspace definition
    workspace_toml = load_toml(f"{base_path}/Cargo.toml")
    if not workspace_toml:
        print("❌ FAILED: Could not load workspace Cargo.toml")
        return False
        
    workspace_deps = workspace_toml.get("workspace", {}).get("dependencies", {})
    if "rust_decimal_macros" not in workspace_deps:
        print("❌ FAILED: rust_decimal_macros not in workspace dependencies")
        return False
    
    workspace_version = workspace_deps["rust_decimal_macros"]
    print(f"✅ Workspace defines: rust_decimal_macros = {workspace_version}")
    
    # Check packages that should use it
    packages = [
        ("libs/amm", "torq-amm"),
        ("services_v2/strategies", "torq-strategies")
    ]
    
    success = True
    for pkg_path, pkg_name in packages:
        full_path = f"{base_path}/{pkg_path}/Cargo.toml"
        if not os.path.exists(full_path):
            print(f"⚠️  WARNING: {full_path} not found")
            continue
            
        pkg_toml = load_toml(full_path)
        if not pkg_toml:
            continue
            
        deps = pkg_toml.get("dependencies", {})
        if "rust_decimal_macros" in deps:
            dep_config = deps["rust_decimal_macros"]
            if isinstance(dep_config, dict) and dep_config.get("workspace") is True:
                print(f"✅ {pkg_name}: Uses workspace rust_decimal_macros")
            else:
                print(f"❌ {pkg_name}: Does NOT use workspace (uses {dep_config})")
                success = False
        else:
            print(f"ℹ️  {pkg_name}: Does not use rust_decimal_macros")
    
    print("=" * 70)
    if success:
        print("🎉 BUILD-001 VALIDATION PASSED")
        print("   All rust_decimal_macros references use workspace dependency correctly")
        return True
    else:
        print("❌ BUILD-001 VALIDATION FAILED") 
        return False

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)