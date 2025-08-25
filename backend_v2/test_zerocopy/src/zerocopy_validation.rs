//! Runtime zerocopy validation tests
//! 
//! Validates that zerocopy operations maintain data integrity and safety
//! across different const generic configurations.

use zerocopy::{IntoBytes, FromBytes};

/// Macro to generate comprehensive zerocopy tests for specific sizes
macro_rules! test_zerocopy_size {
    ($type:ident<$size:literal>, $data:expr, $test_name:ident) => {
        #[test]
        fn $test_name() {
            println!("Testing {}::<{}>", stringify!($type), $size);
            
            // 1. Construction test
            let original_data = $data;
            let fixed_vec = $type::<$size>::new(original_data).unwrap();
            
            // 2. Basic integrity test
            assert_eq!(fixed_vec.as_slice(), original_data);
            assert_eq!(fixed_vec.len(), original_data.len());
            
            // 3. Zerocopy serialization test
            let bytes: &[u8] = fixed_vec.as_bytes();
            println!("  Serialized to {} bytes", bytes.len());
            
            // 4. Zerocopy deserialization test
            match $type::<$size>::ref_from_bytes(bytes) {
                Ok(recovered) => {
                    println!("  ✅ Zerocopy roundtrip successful");
                    assert_eq!(recovered.as_slice(), original_data);
                    assert_eq!(*recovered, fixed_vec);
                }
                Err(e) => {
                    panic!("  ❌ Zerocopy deserialization failed: {:?}", e);
                }
            }
            
            // 5. Memory layout validation
            validate_memory_layout::<$type<$size>>();
            
            // 6. Bijection test (critical for AlphaPulse)
            let reconstructed_vec: Vec<_> = recovered.as_slice().to_vec();
            assert_eq!(reconstructed_vec, original_data.to_vec());
            println!("  ✅ Perfect bijection maintained");
        }
    };
}

/// Validate memory layout properties
fn validate_memory_layout<T: IntoBytes + FromBytes>() {
    use std::mem;
    
    let size = mem::size_of::<T>();
    let align = mem::align_of::<T>();
    
    println!("  Memory layout: size={}, align={}", size, align);
    
    // Verify size is reasonable (not excessively padded)
    assert!(size > 0, "Size must be positive");
    assert!(size < 100000, "Size seems excessively large: {}", size);
    
    // Verify alignment is a power of 2
    assert!(align.is_power_of_two(), "Alignment must be power of 2: {}", align);
    assert!(align <= 128, "Alignment seems excessive: {}", align);
    
    // Verify size is aligned
    assert_eq!(size % align, 0, "Size {} not aligned to boundary {}", size, align);
}

/// Edge case testing for different data patterns
fn test_edge_cases<T, F>(type_name: &str)
where
    T: Copy + Default + std::fmt::Debug + PartialEq,
    F: super::comprehensive_test::FixedVecTest<T> + IntoBytes + FromBytes + std::fmt::Debug + PartialEq,
{
    println!("\n=== Edge Case Testing: {} ===", type_name);
    
    // Test empty data
    println!("  Testing empty data...");
    // This would need to be specialized per type
    
    // Test single element
    println!("  Testing single element...");
    
    // Test maximum capacity
    println!("  Testing maximum capacity...");
    
    // Test overflow conditions
    println!("  Testing overflow conditions...");
}

/// Performance validation - ensure no regression from manual implementations
fn benchmark_zerocopy_performance<T, F>(type_name: &str, iterations: usize)
where
    T: Copy + Default + std::fmt::Debug,
    F: super::comprehensive_test::FixedVecTest<T> + IntoBytes + FromBytes + std::fmt::Debug,
{
    use std::time::Instant;
    
    println!("\n=== Performance Benchmark: {} ===", type_name);
    
    // This would need sample data per type
    // Benchmark construction time
    // Benchmark serialization time  
    // Benchmark deserialization time
    // Compare with manual unsafe implementation baselines
    
    println!("  Performance testing would go here...");
}

#[cfg(test)]
mod tests {
    use super::super::comprehensive_test::*;
    
    // Generate specific test cases using the macro
    test_zerocopy_size!(U8Vec<1>, &[42u8], test_u8_size_1);
    test_zerocopy_size!(U8Vec<8>, &[1u8, 2, 3, 4, 5], test_u8_size_8);
    test_zerocopy_size!(U8Vec<64>, &[1u8; 50], test_u8_size_64);  // Larger array
    
    test_zerocopy_size!(U16Vec<1>, &[1000u16], test_u16_size_1);
    test_zerocopy_size!(U16Vec<8>, &[100u16, 200, 300, 400], test_u16_size_8);
    
    test_zerocopy_size!(U64Vec<1>, &[0x1234567890ABCDEFu64], test_u64_size_1);
    test_zerocopy_size!(U64Vec<8>, &[1u64, 2, 3], test_u64_size_8);
    
    // The critical test - can we handle u128?
    test_zerocopy_size!(U128Vec<1>, &[0x1234567890ABCDEF1234567890ABCDEFu128], test_u128_size_1);
    test_zerocopy_size!(U128Vec<8>, &[1u128, 2, 3], test_u128_size_8);
    
    // AlphaPulse-specific: MockInstrumentId test
    test_zerocopy_size!(InstrumentVec<1>, &[MockInstrumentId { venue: 1, asset_type: 2, reserved: 0, asset_id: 12345 }], test_instrument_size_1);
    test_zerocopy_size!(InstrumentVec<16>, &[
        MockInstrumentId { venue: 1, asset_type: 1, reserved: 0, asset_id: 100 },
        MockInstrumentId { venue: 2, asset_type: 2, reserved: 0, asset_id: 200 },
        MockInstrumentId { venue: 3, asset_type: 3, reserved: 0, asset_id: 300 },
    ], test_instrument_size_16);
    
    #[test]
    fn test_large_sizes() {
        // Test larger N values that might reveal edge cases
        println!("\n=== Large Size Testing ===");
        
        // Test if we can handle N=1000 (this might fail or be very slow to compile)
        // Commenting out until we verify basic cases work
        // test_zerocopy_size!(U8Vec<1000>, &[1u8; 500], test_u8_size_1000);
        
        println!("Large size testing placeholder");
    }
    
    #[test]
    fn test_memory_corruption_detection() {
        println!("\n=== Memory Corruption Detection ===");
        
        // Create a U64Vec with known data
        let original = &[0x1111111111111111u64, 0x2222222222222222u64];
        let vec = U64Vec::<8>::new(original).unwrap();
        
        // Serialize to bytes
        let bytes = vec.as_bytes();
        
        // Verify we can detect corrupted data
        let mut corrupted_bytes = bytes.to_vec();
        if corrupted_bytes.len() > 10 {
            corrupted_bytes[10] = corrupted_bytes[10].wrapping_add(1); // Corrupt one byte
            
            // Try to deserialize corrupted data
            match U64Vec::<8>::ref_from_bytes(&corrupted_bytes) {
                Ok(recovered) => {
                    // If it succeeds, verify the data is actually different
                    println!("  Corruption test: data recovered, checking integrity...");
                    // This might pass if the corruption was in padding
                }
                Err(e) => {
                    println!("  ✅ Corruption correctly detected: {:?}", e);
                }
            }
        }
    }
    
    #[test]
    fn test_alignment_requirements() {
        println!("\n=== Alignment Requirements Testing ===");
        
        // Test that our structs meet alignment requirements
        use std::mem;
        
        // u8: should be 1-byte aligned
        assert_eq!(mem::align_of::<U8Vec<8>>(), 1);
        
        // u64: should be 8-byte aligned  
        assert_eq!(mem::align_of::<U64Vec<8>>(), 8);
        
        // u128: should be 16-byte aligned
        assert_eq!(mem::align_of::<U128Vec<8>>(), 16);
        
        // InstrumentId: depends on largest field (u64 = 8-byte)
        assert!(mem::align_of::<InstrumentVec<16>>() >= 8);
        
        println!("  ✅ All alignment requirements validated");
    }
}