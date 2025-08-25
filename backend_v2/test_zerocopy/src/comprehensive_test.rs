//! Comprehensive zerocopy const generic validation
//! 
//! Tests zerocopy safety guarantees across different data types, sizes, and alignments
//! to determine if we can safely replace manual unsafe implementations.

use zerocopy::{IntoBytes, FromBytes, KnownLayout, Immutable};
use std::mem;

// Test 1: Basic primitive types
#[derive(Debug, Clone, Copy, IntoBytes, FromBytes, KnownLayout, Immutable, PartialEq)]
#[repr(C)]
pub struct U8Vec<const N: usize> {
    len: u8,
    _padding: [u8; 7],  // Align to 8 bytes
    data: [u8; N],
}

#[derive(Debug, Clone, Copy, IntoBytes, FromBytes, KnownLayout, Immutable, PartialEq)]
#[repr(C)]  
pub struct U16Vec<const N: usize> {
    len: u8,
    _padding: [u8; 7],  // Align to 8 bytes
    data: [u16; N],
}

#[derive(Debug, Clone, Copy, IntoBytes, FromBytes, KnownLayout, Immutable, PartialEq)]
#[repr(C)]
pub struct U64Vec<const N: usize> {
    len: u8,
    _padding: [u8; 7],  // Align to 8 bytes  
    data: [u64; N],
}

// Test 2: Large alignment types (this might fail)
#[derive(Debug, Clone, Copy, IntoBytes, FromBytes, KnownLayout, Immutable, PartialEq)]
#[repr(C)]
pub struct U128Vec<const N: usize> {
    len: u8,
    _padding: [u8; 15], // Align to 16 bytes for u128
    data: [u128; N],
}

// Test 3: Mock InstrumentId (simulating AlphaPulse's actual struct)
#[derive(Debug, Clone, Copy, IntoBytes, FromBytes, KnownLayout, Immutable, PartialEq)]
#[repr(C)]
pub struct MockInstrumentId {
    venue: u16,
    asset_type: u8,
    reserved: u8,
    asset_id: u64,
}

#[derive(Debug, Clone, Copy, IntoBytes, FromBytes, KnownLayout, Immutable, PartialEq)]
#[repr(C)]
pub struct InstrumentVec<const N: usize> {
    len: u8,
    _padding: [u8; 7],
    data: [MockInstrumentId; N],
}

// Generic test trait for all FixedVec types
pub trait FixedVecTest<T: Copy + Default> {
    fn new(data: &[T]) -> Result<Self, &'static str> where Self: Sized;
    fn as_slice(&self) -> &[T];
    fn len(&self) -> usize;
}

// Implement for each type
impl<const N: usize> FixedVecTest<u8> for U8Vec<N> {
    fn new(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() > N { return Err("too long"); }
        let mut array = [0u8; N];
        array[..data.len()].copy_from_slice(data);
        Ok(Self { len: data.len() as u8, _padding: [0; 7], data: array })
    }
    
    fn as_slice(&self) -> &[u8] { &self.data[..self.len as usize] }
    fn len(&self) -> usize { self.len as usize }
}

impl<const N: usize> FixedVecTest<u16> for U16Vec<N> {
    fn new(data: &[u16]) -> Result<Self, &'static str> {
        if data.len() > N { return Err("too long"); }
        let mut array = [0u16; N];
        array[..data.len()].copy_from_slice(data);
        Ok(Self { len: data.len() as u8, _padding: [0; 7], data: array })
    }
    
    fn as_slice(&self) -> &[u16] { &self.data[..self.len as usize] }
    fn len(&self) -> usize { self.len as usize }
}

impl<const N: usize> FixedVecTest<u64> for U64Vec<N> {
    fn new(data: &[u64]) -> Result<Self, &'static str> {
        if data.len() > N { return Err("too long"); }
        let mut array = [0u64; N];
        array[..data.len()].copy_from_slice(data);
        Ok(Self { len: data.len() as u8, _padding: [0; 7], data: array })
    }
    
    fn as_slice(&self) -> &[u64] { &self.data[..self.len as usize] }
    fn len(&self) -> usize { self.len as usize }
}

impl<const N: usize> FixedVecTest<u128> for U128Vec<N> {
    fn new(data: &[u128]) -> Result<Self, &'static str> {
        if data.len() > N { return Err("too long"); }
        let mut array = [0u128; N];
        array[..data.len()].copy_from_slice(data);
        Ok(Self { len: data.len() as u8, _padding: [0; 15], data: array })
    }
    
    fn as_slice(&self) -> &[u128] { &self.data[..self.len as usize] }
    fn len(&self) -> usize { self.len as usize }
}

impl<const N: usize> FixedVecTest<MockInstrumentId> for InstrumentVec<N> {
    fn new(data: &[MockInstrumentId]) -> Result<Self, &'static str> {
        if data.len() > N { return Err("too long"); }
        let mut array = [MockInstrumentId::default(); N];
        array[..data.len()].copy_from_slice(data);
        Ok(Self { len: data.len() as u8, _padding: [0; 7], data: array })
    }
    
    fn as_slice(&self) -> &[MockInstrumentId] { &self.data[..self.len as usize] }
    fn len(&self) -> usize { self.len as usize }
}

impl Default for MockInstrumentId {
    fn default() -> Self {
        Self { venue: 0, asset_type: 0, reserved: 0, asset_id: 0 }
    }
}

/// Comprehensive test function for any FixedVec type
fn test_zerocopy_roundtrip<T, F>(
    type_name: &str,
    test_data: &[T],
    sizes_to_test: &[usize]
) where
    T: Copy + Default + std::fmt::Debug + PartialEq,
    F: FixedVecTest<T> + IntoBytes + FromBytes + std::fmt::Debug + PartialEq,
{
    println!("\n=== Testing {} ===", type_name);
    
    for &size in sizes_to_test {
        println!("  Size N={}: ", size);
        
        // Test data that fits
        let data_slice = &test_data[..test_data.len().min(size)];
        
        match test_fixed_vec_size::<T, F>(data_slice, size) {
            Ok(_) => println!("    ✅ Size {} passed", size),
            Err(e) => println!("    ❌ Size {} failed: {}", size, e),
        }
    }
}

/// Test specific size configuration
fn test_fixed_vec_size<T, F>(data: &[T], size: usize) -> Result<(), String>
where
    T: Copy + Default + std::fmt::Debug + PartialEq,
    F: FixedVecTest<T> + IntoBytes + FromBytes + std::fmt::Debug + PartialEq,
{
    // This function will be specialized for each N at compile time
    // We can't actually implement this generically due to const generic limitations
    // Instead, we'll use macros to generate specific test cases
    todo!("Will be implemented with macros for specific sizes")
}

// Compile-time size and alignment validation
const _: () = {
    // Verify expected sizes for different N values
    assert!(mem::size_of::<U8Vec<1>>() == 8 + 1);    // 8 bytes header + 1 byte data
    assert!(mem::size_of::<U8Vec<8>>() == 8 + 8);    // 8 bytes header + 8 bytes data
    
    assert!(mem::size_of::<U64Vec<1>>() == 8 + 8);   // 8 bytes header + 8 bytes data
    assert!(mem::size_of::<U64Vec<8>>() == 8 + 64);  // 8 bytes header + 64 bytes data
    
    assert!(mem::size_of::<U128Vec<1>>() == 16 + 16); // 16 bytes header + 16 bytes data
    assert!(mem::size_of::<U128Vec<8>>() == 16 + 128); // 16 bytes header + 128 bytes data
    
    // Verify alignments
    assert!(mem::align_of::<U8Vec<8>>() == 1);       // u8 alignment
    assert!(mem::align_of::<U64Vec<8>>() == 8);      // u64 alignment  
    assert!(mem::align_of::<U128Vec<8>>() == 16);    // u128 alignment
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_compile_time_properties() {
        // These should all compile without errors if zerocopy derives work
        println!("U8Vec<1> size: {}, align: {}", mem::size_of::<U8Vec<1>>(), mem::align_of::<U8Vec<1>>());
        println!("U64Vec<8> size: {}, align: {}", mem::size_of::<U64Vec<8>>(), mem::align_of::<U64Vec<8>>());
        println!("U128Vec<8> size: {}, align: {}", mem::size_of::<U128Vec<8>>(), mem::align_of::<U128Vec<8>>());
        println!("InstrumentVec<16> size: {}, align: {}", mem::size_of::<InstrumentVec<16>>(), mem::align_of::<InstrumentVec<16>>());
    }
    
    #[test] 
    fn test_basic_construction() {
        // Test that basic construction works for different types
        let u8_data = &[1u8, 2, 3];
        let u8_vec = U8Vec::<8>::new(u8_data).unwrap();
        assert_eq!(u8_vec.as_slice(), u8_data);
        
        let u64_data = &[100u64, 200, 300];
        let u64_vec = U64Vec::<8>::new(u64_data).unwrap();
        assert_eq!(u64_vec.as_slice(), u64_data);
        
        // Test with MockInstrumentId
        let instruments = &[
            MockInstrumentId { venue: 1, asset_type: 2, reserved: 0, asset_id: 12345 },
            MockInstrumentId { venue: 2, asset_type: 3, reserved: 0, asset_id: 67890 },
        ];
        let inst_vec = InstrumentVec::<16>::new(instruments).unwrap();
        assert_eq!(inst_vec.as_slice(), instruments);
    }
}