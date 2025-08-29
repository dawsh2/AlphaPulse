//! Quick test to verify validation reorganization is working

use codec::validation::{ValidationConfig, TLVValidator};

fn main() {
    println!("Testing validation reorganization...");
    
    // Test 1: Can we create a validator?
    let config = ValidationConfig::default();
    let validator = TLVValidator::with_config(config);
    
    println!("✅ ValidationConfig created successfully");
    println!("✅ TLVValidator created successfully");
    println!("✅ Validation reorganization appears to be working!");
}