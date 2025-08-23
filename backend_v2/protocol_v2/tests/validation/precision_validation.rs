//! Precision validation tests for rust_decimal vs f64
//! 
//! Demonstrates that our protocol preserves financial precision
//! while f64 operations lose precision

use rust_decimal::Decimal;
use std::str::FromStr;

/// Test precision preservation with fixed-point arithmetic
#[test]
fn test_decimal_precision_preservation() {
    let test_cases = vec![
        // Values that are problematic with floating point
        "0.00000001",      // 1 satoshi
        "0.12345678",      // 8 decimal places
        "999999.99999999", // Large value with precision
        "0.10000000",      // Should not become 0.9999999
        "1234567.12345678", // Mixed large and small
    ];
    
    for case in test_cases {
        let original_decimal = Decimal::from_str(case).unwrap();
        
        // Convert to fixed point (8 decimal places = multiply by 10^8)
        let scale_factor = Decimal::from(100_000_000);
        let scaled = original_decimal * scale_factor;
        let fixed_point = scaled.to_i64().expect("Should fit in i64");
        
        // Convert back to decimal
        let recovered_decimal = Decimal::from(fixed_point) / scale_factor;
        
        assert_eq!(
            original_decimal, recovered_decimal,
            "Precision lost for value: {}", case
        );
        
        // Test binary serialization
        let bytes = fixed_point.to_le_bytes();
        let recovered_fixed = i64::from_le_bytes(bytes);
        let final_decimal = Decimal::from(recovered_fixed) / scale_factor;
        
        assert_eq!(
            original_decimal, final_decimal,
            "Binary serialization precision lost for: {}", case
        );
        
        println!("✅ {} -> {} -> {} (preserved)", case, fixed_point, final_decimal);
    }
}

/// Demonstrate f64 precision problems
#[test]
fn test_f64_precision_problems() {
    let test_values = vec![
        "0.10000000",
        "0.12345678",
        "999999.99999999",
    ];
    
    for value_str in test_values {
        let original_decimal = Decimal::from_str(value_str).unwrap();
        
        // The WRONG way using f64 (precision loss)
        let as_f64: f64 = value_str.parse().unwrap();
        let f64_fixed = (as_f64 * 1e8).round() as i64;
        let f64_recovered = f64_fixed as f64 / 1e8;
        
        // The RIGHT way using rust_decimal (precise)
        let scale_factor = Decimal::from(100_000_000);
        let decimal_fixed = (original_decimal * scale_factor).to_i64().unwrap();
        let decimal_recovered = Decimal::from(decimal_fixed) / scale_factor;
        
        println!("Value: {}", value_str);
        println!("  f64 path:     {:.8} -> {} -> {:.8}", as_f64, f64_fixed, f64_recovered);
        println!("  decimal path: {} -> {} -> {}", original_decimal, decimal_fixed, decimal_recovered);
        
        // Verify our decimal path never loses precision
        assert_eq!(original_decimal, decimal_recovered, 
                  "rust_decimal path should never lose precision");
        
        // Show difference
        let f64_as_decimal = Decimal::try_from(f64_recovered).unwrap_or_default();
        if original_decimal != f64_as_decimal {
            println!("  ⚠️  f64 precision loss: {} vs {}", original_decimal, f64_as_decimal);
        } else {
            println!("  ✅ f64 happened to preserve precision for this value");
        }
        println!();
    }
}

/// Test extreme values that definitely break f64
#[test]
fn test_extreme_precision_cases() {
    // These values are known to have precision issues with f64
    let problematic_values = vec![
        "0.1",              // Classic f64 precision issue
        "0.2",              // Another classic case
        "0.30000000",       // Should stay exact
        "12345678.12345678", // Large number with 8 decimals
    ];
    
    for value_str in problematic_values {
        let decimal = Decimal::from_str(value_str).unwrap();
        let f64_val: f64 = value_str.parse().unwrap();
        
        // Test if f64 can represent it exactly
        let f64_string = format!("{:.8}", f64_val);
        let f64_back = Decimal::from_str(&f64_string).unwrap_or_default();
        
        println!("Testing: {}", value_str);
        println!("  Decimal: {}", decimal);
        println!("  f64:     {:.8} -> {}", f64_val, f64_back);
        
        if decimal != f64_back {
            println!("  ❌ f64 precision loss confirmed");
        } else {
            println!("  ✅ f64 preserved precision (rare)");
        }
        
        // Show our approach always works
        let scale = Decimal::from(100_000_000);
        let fixed = (decimal * scale).to_i64().unwrap();
        let recovered = Decimal::from(fixed) / scale;
        
        assert_eq!(decimal, recovered, "Our approach should always preserve precision");
        println!("  ✅ Our method: {} -> {} -> {} (perfect)", decimal, fixed, recovered);
        println!();
    }
}

trait ToI64 {
    fn to_i64(&self) -> Option<i64>;
}

impl ToI64 for Decimal {
    fn to_i64(&self) -> Option<i64> {
        if *self >= Decimal::from(i64::MIN) && *self <= Decimal::from(i64::MAX) {
            let mantissa_value = self.mantissa() / 10_i128.pow(self.scale() as u32);
            Some(mantissa_value as i64)
        } else {
            None
        }
    }
}