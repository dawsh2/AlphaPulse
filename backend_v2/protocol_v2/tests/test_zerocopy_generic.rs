//! Test zerocopy derive support for const generic types
//!
//! This test validates whether we can use zerocopy derives directly on FixedVec<T, N>
//! instead of manual unsafe implementations.

use zerocopy::{AsBytes, FromBytes, FromZeroes};

#[repr(C)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes, PartialEq)]
pub struct TestFixedVec<T: Copy + Default + AsBytes + FromBytes + FromZeroes, const N: usize> {
    len: u16,          // actual number of elements
    _padding: [u8; 6], // align to 8 bytes
    data: [T; N],      // fixed-size storage
}

impl<T: Copy + Default + AsBytes + FromBytes + FromZeroes, const N: usize> TestFixedVec<T, N> {
    pub fn new(slice: &[T]) -> Result<Self, &'static str> {
        if slice.len() > N {
            return Err("slice too long");
        }
        let mut data = [T::default(); N];
        data[..slice.len()].copy_from_slice(slice);
        Ok(Self { 
            len: slice.len() as u16, 
            _padding: [0; 6],
            data,
        })
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data[..self.len as usize]
    }

    pub fn to_vec(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u64_zerocopy_roundtrip() {
        let original = &[1u64, 2, 3, 4, 5];
        let fixed: TestFixedVec<u64, 8> = TestFixedVec::new(original).unwrap();

        // Zero-copy cast to bytes
        let bytes: &[u8] = fixed.as_bytes();
        
        // Zero-copy recovery
        let recovered: &TestFixedVec<u64, 8> = TestFixedVec::ref_from(bytes).unwrap();
        
        // Verify perfect bijection
        assert_eq!(fixed, *recovered);
        assert_eq!(original, recovered.as_slice());
        assert_eq!(original.to_vec(), recovered.to_vec());
        
        println!("✅ u64 zerocopy test passed: {:?}", fixed.as_slice());
    }

    #[test] 
    fn test_u128_zerocopy_roundtrip() {
        let original = &[100u128, 200, 300];
        let fixed: TestFixedVec<u128, 8> = TestFixedVec::new(original).unwrap();

        // Zero-copy operations
        let bytes: &[u8] = fixed.as_bytes();
        let recovered: &TestFixedVec<u128, 8> = TestFixedVec::ref_from(bytes).unwrap();
        
        // Perfect bijection validation
        assert_eq!(original, recovered.as_slice());
        
        println!("✅ u128 zerocopy test passed: {:?}", fixed.as_slice());
    }

    #[test]
    fn test_empty_vec() {
        let empty: &[u64] = &[];
        let fixed: TestFixedVec<u64, 8> = TestFixedVec::new(empty).unwrap();
        
        assert_eq!(fixed.len, 0);
        assert_eq!(fixed.as_slice(), empty);
        
        // Zero-copy with empty
        let bytes: &[u8] = fixed.as_bytes();
        let recovered: &TestFixedVec<u64, 8> = TestFixedVec::ref_from(bytes).unwrap();
        assert_eq!(recovered.as_slice(), empty);
        
        println!("✅ Empty vec zerocopy test passed");
    }

    #[test]
    fn test_capacity_limit() {
        let too_big = &[1u64; 10]; // Exceeds capacity of 8
        let result = TestFixedVec::<u64, 8>::new(too_big);
        assert!(result.is_err());
        
        println!("✅ Capacity limit test passed");
    }

    #[test]
    fn test_size_and_alignment() {
        // Verify expected memory layout
        assert_eq!(std::mem::size_of::<TestFixedVec<u64, 8>>(), 8 + 64); // 2 + 6 + 64
        assert_eq!(std::mem::align_of::<TestFixedVec<u64, 8>>(), 8);
        
        assert_eq!(std::mem::size_of::<TestFixedVec<u128, 8>>(), 8 + 128); // 2 + 6 + 128  
        assert_eq!(std::mem::align_of::<TestFixedVec<u128, 8>>(), 16);
        
        println!("✅ Size and alignment test passed");
    }
}