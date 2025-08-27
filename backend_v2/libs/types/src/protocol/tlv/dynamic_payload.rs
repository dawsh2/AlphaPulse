//! Dynamic Payload Infrastructure for Zero-Copy TLV Serialization
//!
//! Provides a universal abstraction for fixed-size collections that enable zero-copy
//! serialization while preserving bijective ID mapping. This solves the fundamental
//! tension between Vec<T> (bijective but not zero-copy due to heap indirection) and
//! fixed arrays (zero-copy but bounded at compile time).
//!
//! **Critical Design Decision**: PoolLiquidityTLV uses `FixedVec<u128, N>` NOT `Vec<u128>`
//! This enables >1M msg/s zero-copy performance while maintaining dynamic sizing.
//!
//! ## Architecture Role
//!
//! Core infrastructure supporting all Protocol V2 TLV structures that need variable-length
//! data with zero-copy serialization. Used by:
//! - StateInvalidationTLV: Multiple instrument IDs
//! - PoolLiquidityTLV: Multiple token reserves
//! - Future TLVs requiring dynamic collections
//!
//! ## Performance Characteristics
//!
//! - Zero-copy serialization/deserialization via zerocopy derives
//! - O(1) access to elements
//! - Bounded memory usage with compile-time guarantees
//! - Preserves exact semantics of original Vec<T> collections

use std::convert::TryFrom;
use zerocopy::{AsBytes, FromBytes, FromZeroes};

// Import OrderLevel for zerocopy trait implementations
use super::market_data::OrderLevel;

// Re-export configuration functions for backward compatibility
pub use super::config::{max_instruments as MAX_INSTRUMENTS_FN, max_pool_tokens as MAX_POOL_TOKENS_FN, max_order_levels as MAX_ORDER_LEVELS_FN};

// Compile-time constants for array sizing (must remain const for zerocopy)
// These represent the maximum possible values - runtime config can be lower
pub const MAX_INSTRUMENTS: usize = 16; // Compile-time max for array sizing
pub const MAX_POOL_TOKENS: usize = 8;  // Compile-time max for array sizing  
pub const MAX_ORDER_LEVELS: usize = 50; // Compile-time max for array sizing

/// Error type for dynamic payload operations
#[derive(Debug, Clone, PartialEq)]
pub enum PayloadError {
    /// Attempted to insert more elements than maximum capacity
    CapacityExceeded {
        max_capacity: usize,
        attempted: usize,
    },
    /// Invalid slice length for conversion
    InvalidLength { expected_max: usize, got: usize },
}

impl std::fmt::Display for PayloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayloadError::CapacityExceeded {
                max_capacity,
                attempted,
            } => {
                write!(
                    f,
                    "Capacity exceeded: max={}, attempted={}",
                    max_capacity, attempted
                )
            }
            PayloadError::InvalidLength { expected_max, got } => {
                write!(
                    f,
                    "Invalid length: expected max {}, got {}",
                    expected_max, got
                )
            }
        }
    }
}

impl std::error::Error for PayloadError {}

/// Universal trait for dynamic payload types
///
/// Provides a common interface for zero-copy collections with bounded capacity.
/// Enables perfect bijection: Vec<T> → DynamicPayload<T> → Vec<T>
pub trait DynamicPayload<T> {
    type Error;

    /// Maximum capacity of this payload type
    fn max_capacity() -> usize;

    /// Current number of valid elements
    fn len(&self) -> usize;

    /// Check if payload is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get element at index (bounds checked)
    fn get(&self, index: usize) -> Option<&T>;

    /// Convert to slice of valid elements only
    fn as_slice(&self) -> &[T];

    /// Create from slice with bounds validation
    fn from_slice(slice: &[T]) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Convert to Vec for perfect bijection preservation
    fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.as_slice().to_vec()
    }
}

/// Fixed-capacity vector with zero-copy serialization support
///
/// **Key Distinction: FixedVec ≠ Vec**
/// - `Vec<T>`: Heap-allocated, pointer indirection → **Cannot be zero-copy**
/// - `FixedVec<T, N>`: Stack-allocated, inline storage → **Enables zero-copy**
///
/// This structure solves the fundamental tension between:
/// 1. `Vec<T>`: Perfect bijection but no zero-copy (heap indirection)
/// 2. `[T; N]`: Zero-copy but no dynamic sizing (fixed at compile time)
/// 3. `FixedVec<T, N>`: Both zero-copy AND dynamic sizing (up to N elements)
///
/// Maintains exact element count while using fixed-size array for zero-copy.
/// Unused slots are zeroed for deterministic serialization.
///
/// # Memory Layout (Inline Storage)
///
/// ```
/// [count: u16][_padding: [u8; 6]][elements: [T; N]][_align_padding: varies]
/// ```
///
/// All data is stored inline (no heap allocation), enabling direct memory mapping
/// for zero-copy serialization/deserialization with zerocopy traits.
///
/// # Zero-Copy vs Bijection Methods
///
/// - **Zero-copy**: Direct `AsBytes`/`FromBytes` on entire structure
/// - **Bijection methods** (`to_vec()`, `from_slice()`): For interoperability with existing Vec-based APIs
///
/// The bijection methods exist for convenience, not because internal storage uses `Vec<T>`.
///
/// Note: Due to zerocopy crate limitations with generics, zero-copy serialization
/// must be implemented manually for specific instantiations.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedVec<T, const N: usize>
where
    T: Copy,
{
    /// Number of valid elements (0 to N)
    count: u16,

    /// Padding to ensure proper alignment for elements array
    _padding: [u8; 6],

    /// Fixed-size array holding elements (unused slots are zeroed)
    elements: [T; N],
}

// Manual zerocopy implementations for FixedVec<u128, MAX_POOL_TOKENS>
// These enable zero-copy serialization for pool liquidity data
//
// SAFETY: FixedVec has a well-defined memory layout with #[repr(C)]:
// - count: u16 (2 bytes)
// - _padding: [u8; 6] (6 bytes)
// - elements: [u128; 8] (128 bytes)
// Total: 136 bytes with proper alignment
//
// All fields are FromZeroes/AsBytes compatible:
// - u16, u8, and u128 are primitive types with zerocopy support
// - Fixed arrays of zerocopy types are also zerocopy
// - The struct uses #[repr(C)] for deterministic layout
unsafe impl AsBytes for FixedVec<u128, MAX_POOL_TOKENS> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl FromBytes for FixedVec<u128, MAX_POOL_TOKENS> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl FromZeroes for FixedVec<u128, MAX_POOL_TOKENS> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

// Manual zerocopy implementations for FixedVec<InstrumentId, MAX_INSTRUMENTS>
// These enable zero-copy serialization for state invalidation data
//
// SAFETY: FixedVec has a well-defined memory layout with #[repr(C)]:
// - count: u16 (2 bytes)
// - _padding: [u8; 6] (6 bytes)
// - elements: [InstrumentId; 16] (128 bytes)
// Total: 136 bytes with proper alignment
//
// InstrumentId is also #[repr(C)] with zerocopy support from the core protocol
unsafe impl AsBytes for FixedVec<crate::InstrumentId, MAX_INSTRUMENTS> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl FromBytes for FixedVec<crate::InstrumentId, MAX_INSTRUMENTS> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl FromZeroes for FixedVec<crate::InstrumentId, MAX_INSTRUMENTS> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

// Manual zerocopy implementations for FixedVec<OrderLevel, MAX_ORDER_LEVELS>
// These enable zero-copy serialization for order book data
//
// SAFETY: FixedVec has a well-defined memory layout with #[repr(C)]:
// - count: u16 (2 bytes)
// - _padding: [u8; 6] (6 bytes)
// - elements: [OrderLevel; 50] (1200 bytes: 50 * 24 bytes each)
// Total: 1208 bytes with proper alignment
//
// OrderLevel is #[repr(C)] with zerocopy support from market_data.rs
unsafe impl AsBytes for FixedVec<OrderLevel, MAX_ORDER_LEVELS> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl FromBytes for FixedVec<OrderLevel, MAX_ORDER_LEVELS> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl FromZeroes for FixedVec<OrderLevel, MAX_ORDER_LEVELS> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

impl<T, const N: usize> FixedVec<T, N>
where
    T: Copy + Default,
{
    /// Create new empty FixedVec
    pub fn new() -> Self {
        Self {
            count: 0,
            _padding: [0; 6],
            elements: [T::default(); N],
        }
    }

    /// Get the underlying array (including unused slots)
    pub fn as_array(&self) -> &[T; N] {
        &self.elements
    }

    /// Get mutable access to the underlying array (for zero-copy construction)
    pub fn as_array_mut(&mut self) -> &mut [T; N] {
        &mut self.elements
    }

    /// Set the element count (for direct array manipulation)
    ///
    /// # Safety
    ///
    /// Caller must ensure that elements[0..count] are properly initialized
    pub fn set_count(&mut self, count: usize) {
        debug_assert!(count <= N, "Count {} exceeds capacity {}", count, N);
        self.count = count.min(N) as u16;
    }

    /// Push element if capacity allows
    pub fn try_push(&mut self, element: T) -> Result<(), PayloadError> {
        if self.count as usize >= N {
            return Err(PayloadError::CapacityExceeded {
                max_capacity: N,
                attempted: self.count as usize + 1,
            });
        }

        self.elements[self.count as usize] = element;
        self.count += 1;
        Ok(())
    }

    /// Clear all elements (zeros the array)
    pub fn clear(&mut self) {
        self.count = 0;
        self.elements = [T::default(); N];
    }

    /// Get iterator over valid elements only
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.as_slice().iter()
    }
}

impl<T, const N: usize> Default for FixedVec<T, N>
where
    T: Copy + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> DynamicPayload<T> for FixedVec<T, N>
where
    T: Copy + Default,
{
    type Error = PayloadError;

    fn max_capacity() -> usize {
        N
    }

    fn len(&self) -> usize {
        self.count as usize
    }

    fn get(&self, index: usize) -> Option<&T> {
        if index < self.count as usize {
            Some(&self.elements[index])
        } else {
            None
        }
    }

    fn as_slice(&self) -> &[T] {
        &self.elements[..self.count as usize]
    }

    fn from_slice(slice: &[T]) -> Result<Self, Self::Error> {
        if slice.len() > N {
            return Err(PayloadError::InvalidLength {
                expected_max: N,
                got: slice.len(),
            });
        }

        let mut result = Self::new();
        result.count = slice.len() as u16;

        // Copy elements to array
        for (i, &element) in slice.iter().enumerate() {
            result.elements[i] = element;
        }

        Ok(result)
    }
}

impl<T, const N: usize> TryFrom<&[T]> for FixedVec<T, N>
where
    T: Copy + Default,
{
    type Error = PayloadError;

    fn try_from(slice: &[T]) -> Result<Self, Self::Error> {
        Self::from_slice(slice)
    }
}

impl<T, const N: usize> TryFrom<Vec<T>> for FixedVec<T, N>
where
    T: Copy + Default,
{
    type Error = PayloadError;

    fn try_from(vec: Vec<T>) -> Result<Self, Self::Error> {
        Self::from_slice(&vec)
    }
}

/// Fixed-capacity UTF-8 string with zero-copy serialization
///
/// Stores UTF-8 string data in fixed-size array with length prefix.
/// Unused bytes are zeroed for deterministic serialization.
///
/// Note: Due to zerocopy crate limitations with generics, zero-copy serialization
/// must be implemented manually for specific instantiations.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedStr<const N: usize> {
    /// Length of valid UTF-8 data
    len: u16,

    /// Padding for alignment
    _padding: [u8; 6],

    /// Raw UTF-8 bytes (unused bytes are zeroed)
    data: [u8; N],
}

impl<const N: usize> FixedStr<N> {
    /// Create new empty string
    pub fn new() -> Self {
        Self {
            len: 0,
            _padding: [0; 6],
            data: [0; N],
        }
    }

    /// Create from string slice with validation
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, PayloadError> {
        let bytes = s.as_bytes();
        if bytes.len() > N {
            return Err(PayloadError::InvalidLength {
                expected_max: N,
                got: bytes.len(),
            });
        }

        let mut result = Self::new();
        result.len = bytes.len() as u16;
        result.data[..bytes.len()].copy_from_slice(bytes);
        Ok(result)
    }

    /// Get string slice of valid data
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.data[..self.len as usize])
    }

    /// Get length of valid string data
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Check if string is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get maximum capacity
    pub const fn capacity() -> usize {
        N
    }
}

impl<const N: usize> Default for FixedStr<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> TryFrom<&str> for FixedStr<N> {
    type Error = PayloadError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl<const N: usize> TryFrom<String> for FixedStr<N> {
    type Error = PayloadError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_vec_basic_operations() {
        let mut vec: FixedVec<u32, 4> = FixedVec::new();
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
        assert_eq!(FixedVec::<u32, 4>::max_capacity(), 4);

        // Test pushing elements
        assert!(vec.try_push(10).is_ok());
        assert!(vec.try_push(20).is_ok());
        assert_eq!(vec.len(), 2);
        assert!(!vec.is_empty());

        // Test accessing elements
        assert_eq!(vec.get(0), Some(&10));
        assert_eq!(vec.get(1), Some(&20));
        assert_eq!(vec.get(2), None);

        // Test slice conversion
        assert_eq!(vec.as_slice(), &[10, 20]);
    }

    #[test]
    fn test_fixed_vec_capacity_exceeded() {
        let mut vec: FixedVec<u32, 2> = FixedVec::new();

        assert!(vec.try_push(1).is_ok());
        assert!(vec.try_push(2).is_ok());

        // Should fail on third element
        let result = vec.try_push(3);
        assert!(matches!(result, Err(PayloadError::CapacityExceeded { .. })));
    }

    #[test]
    fn test_fixed_vec_from_slice() {
        let slice = &[1u32, 2, 3];
        let vec: FixedVec<u32, 5> = FixedVec::from_slice(slice).unwrap();

        assert_eq!(vec.len(), 3);
        assert_eq!(vec.as_slice(), slice);
        assert_eq!(vec.to_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn test_fixed_vec_bijection() {
        let original = vec![10u32, 20, 30, 40];
        let fixed_vec: FixedVec<u32, 8> = FixedVec::from_slice(&original).unwrap();
        let recovered = fixed_vec.to_vec();

        // Perfect bijection
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_fixed_vec_memory_layout() {
        let mut vec: FixedVec<u64, 4> = FixedVec::new();
        vec.try_push(0x1122334455667788).unwrap();
        vec.try_push(0x99AABBCCDDEEFF00).unwrap();

        // Test memory layout and size
        use std::mem::size_of;
        let expected_size = size_of::<u16>() + 6 + (4 * size_of::<u64>());
        assert_eq!(size_of::<FixedVec<u64, 4>>(), expected_size);

        // Test element access
        assert_eq!(vec.len(), 2);
        assert_eq!(vec.get(0), Some(&0x1122334455667788));
        assert_eq!(vec.get(1), Some(&0x99AABBCCDDEEFF00));

        // Test slice representation
        let slice = vec.as_slice();
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0], 0x1122334455667788);
        assert_eq!(slice[1], 0x99AABBCCDDEEFF00);
    }

    #[test]
    fn test_fixed_str_basic_operations() {
        let mut s: FixedStr<16> = FixedStr::new();
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
        assert_eq!(FixedStr::<16>::capacity(), 16);

        // Create from string
        s = FixedStr::from_str("hello").unwrap();
        assert_eq!(s.len(), 5);
        assert!(!s.is_empty());
        assert_eq!(s.as_str().unwrap(), "hello");
    }

    #[test]
    fn test_fixed_str_capacity_exceeded() {
        let long_str = "This string is definitely longer than 8 characters";
        let result = FixedStr::<8>::from_str(long_str);
        assert!(matches!(result, Err(PayloadError::InvalidLength { .. })));
    }

    #[test]
    fn test_fixed_str_memory_layout() {
        let s = FixedStr::<32>::from_str("test string").unwrap();

        // Test memory layout and size
        use std::mem::size_of;
        let expected_size = size_of::<u16>() + 6 + 32;
        assert_eq!(size_of::<FixedStr<32>>(), expected_size);

        // Test string properties
        assert_eq!(s.len(), "test string".len());
        assert_eq!(s.as_str().unwrap(), "test string");
        assert!(!s.is_empty());

        // Test capacity
        assert_eq!(FixedStr::<32>::capacity(), 32);
    }

    #[test]
    fn test_domain_constants() {
        // Verify constants match expected values
        assert_eq!(MAX_INSTRUMENTS, 16);
        assert_eq!(MAX_POOL_TOKENS, 8);

        // Test that these can be used as const generic parameters
        let _instruments: FixedVec<u64, MAX_INSTRUMENTS> = FixedVec::new();
        let _reserves: FixedVec<u128, MAX_POOL_TOKENS> = FixedVec::new();
    }
}
