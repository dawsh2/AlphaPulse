mod comprehensive_test;
mod zerocopy_validation;
mod alphapulse_actual;

use zerocopy::{IntoBytes, FromBytes, KnownLayout, Immutable};

// Simple test struct for initial validation
#[repr(C)]
#[derive(Debug, Clone, Copy, IntoBytes, FromBytes, KnownLayout, Immutable, PartialEq)]
pub struct SimpleFixedVec<const N: usize> {
    len: u8,
    data: [u8; N],
}

impl<const N: usize> SimpleFixedVec<N> {
    pub fn new(slice: &[u8]) -> Result<Self, &'static str> {
        if slice.len() > N {
            return Err("slice too long");
        }
        let mut data = [0u8; N];
        data[..slice.len()].copy_from_slice(slice);
        Ok(Self { 
            len: slice.len() as u8,
            data,
        })
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
}

fn main() {
    println!("🧪 Comprehensive Zerocopy Const Generic Testing Suite");
    println!("====================================================\n");
    
    // Phase 1: Basic validation
    println!("Phase 1: Basic u8 validation...");
    test_basic_u8();
    
    // Phase 2: Critical AlphaPulse types
    println!("\nPhase 2: Testing AlphaPulse-critical types...");
    test_critical_types();
    
    // Phase 3: Edge cases and limits  
    println!("\nPhase 3: Edge case testing...");
    test_edge_cases();
    
    // Phase 4: Performance validation
    println!("\nPhase 4: Performance validation...");
    test_performance();
    
    // Phase 5: Memory safety validation
    println!("\nPhase 5: Memory safety validation...");
    test_memory_safety();
    
    println!("\n🎉 All zerocopy const generic tests completed!");
}

fn test_basic_u8() {
    let original = &[1u8, 2, 3, 4, 5];
    let fixed: SimpleFixedVec<8> = SimpleFixedVec::new(original).unwrap();

    let bytes: &[u8] = fixed.as_bytes();
    println!("  Basic u8 serialized to {} bytes", bytes.len());
    
    match SimpleFixedVec::<8>::ref_from_bytes(bytes) {
        Ok(recovered) => {
            println!("  ✅ Basic u8 zerocopy SUCCESS");
            assert_eq!(original, recovered.as_slice());
        }
        Err(e) => {
            println!("  ❌ Basic u8 FAILED: {:?}", e);
            panic!("Basic test failed");
        }
    }
}

fn test_critical_types() {
    use comprehensive_test::*;
    
    // Test u64 (critical for AlphaPulse timestamps, prices)
    println!("  Testing u64 arrays...");
    test_u64_arrays();
    
    // Test u128 (critical for DEX token amounts) 
    println!("  Testing u128 arrays...");
    test_u128_arrays();
    
    // Test InstrumentId-like structs
    println!("  Testing InstrumentId-like structs...");
    test_instrument_ids();
}

fn test_u64_arrays() {
    use comprehensive_test::U64Vec;
    
    let data = &[1234567890u64, 9876543210u64, 5555555555u64];
    
    // Test different sizes
    for &size in &[1, 4, 8, 16] {
        let test_data = &data[..data.len().min(size)];
        
        match size {
            1 => test_specific_u64_size::<1>(test_data),
            4 => test_specific_u64_size::<4>(test_data), 
            8 => test_specific_u64_size::<8>(test_data),
            16 => test_specific_u64_size::<16>(test_data),
            _ => continue,
        }
    }
}

fn test_specific_u64_size<const N: usize>(data: &[u64]) {
    use comprehensive_test::{U64Vec, FixedVecTest};
    
    match U64Vec::<N>::new(data) {
        Ok(vec) => {
            let bytes = vec.as_bytes();
            match U64Vec::<N>::ref_from_bytes(bytes) {
                Ok(recovered) => {
                    assert_eq!(vec.as_slice(), recovered.as_slice());
                    println!("    ✅ U64Vec<{}> SUCCESS", N);
                }
                Err(e) => {
                    println!("    ❌ U64Vec<{}> deserialization FAILED: {:?}", N, e);
                }
            }
        }
        Err(e) => {
            println!("    ❌ U64Vec<{}> construction FAILED: {}", N, e);
        }
    }
}

fn test_u128_arrays() {
    use comprehensive_test::U128Vec;
    
    let data = &[
        0x1234567890ABCDEFu128,
        0xFEDCBA0987654321u128,
    ];
    
    // Test u128 - this is the critical test for DEX token amounts
    for &size in &[1, 2, 8] {
        let test_data = &data[..data.len().min(size)];
        
        match size {
            1 => test_specific_u128_size::<1>(test_data),
            2 => test_specific_u128_size::<2>(test_data),
            8 => test_specific_u128_size::<8>(test_data),
            _ => continue,
        }
    }
}

fn test_specific_u128_size<const N: usize>(data: &[u128]) {
    use comprehensive_test::{U128Vec, FixedVecTest};
    
    match U128Vec::<N>::new(data) {
        Ok(vec) => {
            let bytes = vec.as_bytes();
            match U128Vec::<N>::ref_from_bytes(bytes) {
                Ok(recovered) => {
                    assert_eq!(vec.as_slice(), recovered.as_slice());
                    println!("    ✅ U128Vec<{}> SUCCESS - Critical for DEX amounts!", N);
                }
                Err(e) => {
                    println!("    ❌ U128Vec<{}> deserialization FAILED: {:?}", N, e);
                }
            }
        }
        Err(e) => {
            println!("    ❌ U128Vec<{}> construction FAILED: {}", N, e);
        }
    }
}

fn test_instrument_ids() {
    use comprehensive_test::{InstrumentVec, MockInstrumentId, FixedVecTest};
    
    let instruments = &[
        MockInstrumentId { venue: 1, asset_type: 1, reserved: 0, asset_id: 12345 },
        MockInstrumentId { venue: 2, asset_type: 2, reserved: 0, asset_id: 67890 },
        MockInstrumentId { venue: 3, asset_type: 3, reserved: 0, asset_id: 11111 },
    ];
    
    // Test InstrumentId arrays - critical for AlphaPulse state invalidation
    match InstrumentVec::<16>::new(instruments) {
        Ok(vec) => {
            let bytes = vec.as_bytes();
            match InstrumentVec::<16>::ref_from_bytes(bytes) {
                Ok(recovered) => {
                    assert_eq!(vec.as_slice(), recovered.as_slice());
                    println!("    ✅ InstrumentVec<16> SUCCESS - Critical for state invalidation!");
                }
                Err(e) => {
                    println!("    ❌ InstrumentVec<16> deserialization FAILED: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("    ❌ InstrumentVec<16> construction FAILED: {}", e);
        }
    }
}

fn test_edge_cases() {
    println!("  Testing empty arrays...");
    test_empty_arrays();
    
    println!("  Testing single elements...");
    test_single_elements();
    
    println!("  Testing capacity limits...");
    test_capacity_limits();
}

fn test_empty_arrays() {
    use comprehensive_test::{U64Vec, FixedVecTest};
    
    let empty_data: &[u64] = &[];
    
    match U64Vec::<8>::new(empty_data) {
        Ok(vec) => {
            assert_eq!(vec.len(), 0);
            assert_eq!(vec.as_slice(), &[]);
            
            let bytes = vec.as_bytes();
            match U64Vec::<8>::ref_from_bytes(bytes) {
                Ok(recovered) => {
                    assert_eq!(recovered.as_slice(), &[]);
                    println!("    ✅ Empty array test SUCCESS");
                }
                Err(e) => println!("    ❌ Empty array deserialization FAILED: {:?}", e),
            }
        }
        Err(e) => println!("    ❌ Empty array construction FAILED: {}", e),
    }
}

fn test_single_elements() {
    use comprehensive_test::{U128Vec, FixedVecTest};
    
    let single_element = &[0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFu128];
    
    match U128Vec::<1>::new(single_element) {
        Ok(vec) => {
            let bytes = vec.as_bytes();
            match U128Vec::<1>::ref_from_bytes(bytes) {
                Ok(recovered) => {
                    assert_eq!(recovered.as_slice(), single_element);
                    println!("    ✅ Single element test SUCCESS");
                }
                Err(e) => println!("    ❌ Single element deserialization FAILED: {:?}", e),
            }
        }
        Err(e) => println!("    ❌ Single element construction FAILED: {}", e),
    }
}

fn test_capacity_limits() {
    use comprehensive_test::{U64Vec, FixedVecTest};
    
    // Test exceeding capacity
    let too_much_data = &[1u64; 10];  // 10 elements
    
    match U64Vec::<8>::new(too_much_data) {  // Capacity only 8
        Ok(_) => println!("    ❌ Capacity limit test FAILED - should have rejected"),
        Err(_) => println!("    ✅ Capacity limit test SUCCESS - correctly rejected"),
    }
}

fn test_performance() {
    use std::time::Instant;
    use comprehensive_test::{U64Vec, FixedVecTest};
    
    let data = &[1u64, 2, 3, 4, 5, 6, 7, 8];
    let iterations = 10000;
    
    // Test construction performance
    let start = Instant::now();
    for _ in 0..iterations {
        let _vec = U64Vec::<8>::new(data).unwrap();
    }
    let construction_time = start.elapsed();
    
    // Test serialization performance
    let vec = U64Vec::<8>::new(data).unwrap();
    let start = Instant::now();
    for _ in 0..iterations {
        let _bytes = vec.as_bytes();
    }
    let serialization_time = start.elapsed();
    
    // Test deserialization performance
    let bytes = vec.as_bytes();
    let start = Instant::now();
    for _ in 0..iterations {
        let _recovered = U64Vec::<8>::ref_from_bytes(bytes).unwrap();
    }
    let deserialization_time = start.elapsed();
    
    println!("  Construction: {:.2}μs per operation", 
             construction_time.as_nanos() as f64 / iterations as f64 / 1000.0);
    println!("  Serialization: {:.2}ns per operation", 
             serialization_time.as_nanos() as f64 / iterations as f64);
    println!("  Deserialization: {:.2}ns per operation", 
             deserialization_time.as_nanos() as f64 / iterations as f64);
             
    // Verify we meet AlphaPulse's <35μs hot path requirement
    let total_ns = (serialization_time.as_nanos() + deserialization_time.as_nanos()) / iterations as u128;
    let total_us = total_ns as f64 / 1000.0;
    
    if total_us < 35.0 {
        println!("  ✅ Performance target MET: {:.2}μs < 35μs", total_us);
    } else {
        println!("  ❌ Performance target MISSED: {:.2}μs >= 35μs", total_us);
    }
}

fn test_memory_safety() {
    use std::mem;
    use comprehensive_test::*;
    
    println!("  Validating memory layouts...");
    
    // Test that all our types have reasonable memory layouts
    println!("    U8Vec<8>: size={}, align={}", 
             mem::size_of::<U8Vec<8>>(), mem::align_of::<U8Vec<8>>());
    println!("    U64Vec<8>: size={}, align={}", 
             mem::size_of::<U64Vec<8>>(), mem::align_of::<U64Vec<8>>());
    println!("    U128Vec<8>: size={}, align={}", 
             mem::size_of::<U128Vec<8>>(), mem::align_of::<U128Vec<8>>());
    println!("    InstrumentVec<16>: size={}, align={}", 
             mem::size_of::<InstrumentVec<16>>(), mem::align_of::<InstrumentVec<16>>());
    
    // Verify alignments are sensible
    assert!(mem::align_of::<U64Vec<8>>() >= 8, "U64Vec should be 8-byte aligned");
    assert!(mem::align_of::<U128Vec<8>>() >= 16, "U128Vec should be 16-byte aligned");
    
    println!("  ✅ Memory layout validation SUCCESS");
}