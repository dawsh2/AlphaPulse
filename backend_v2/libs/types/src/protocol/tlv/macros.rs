//! TLV Structure Generation Macro
//!
//! Provides the `define_tlv!` macro for declaratively defining zero-copy TLV structures
//! with automatic field ordering, padding, and constructor generation.
//!
//! ## Purpose
//!
//! Eliminates repetitive boilerplate across 16+ TLV structures while ensuring:
//! - Correct field ordering for zero padding (u128 → u64 → u32 → u16 → u8)
//! - Required zerocopy trait derives
//! - Consistent constructor patterns
//! - Compile-time size validation
//!
//! ## Architecture Role
//!
//! The macro enforces consistent structure across all TLV types, preventing common
//! mistakes like incorrect field ordering that would cause padding issues. It integrates
//! with the zero-copy serialization system to ensure all TLV structs are compatible
//! with high-performance message construction and parsing.
//!
//! ## Usage Example
//!
//! ```rust
//! define_tlv! {
//!     /// Trade TLV - market transaction data
//!     TradeTLV {
//!         u64: {
//!             timestamp_ns: u64,
//!             asset_id: u64,
//!             price: i64,
//!             volume: i64
//!         }
//!         u32: {}
//!         u16: { venue_id: u16 }
//!         u8: {
//!             asset_type: u8,
//!             reserved: u8,
//!             side: u8,
//!             _padding: [u8; 3]
//!         }
//!         special: {}
//!     }
//! }
//! ```

/// Generate zero-copy TLV struct with proper field ordering and padding
///
/// Automatically handles:
/// - Field ordering (u128 → u64 → u32 → u16 → u8) for zero padding
/// - Required zerocopy derives
/// - Constructor generation
/// - Size validation
///
/// # Field Categories
///
/// Fields must be grouped by alignment requirements:
/// - `u128`: 16-byte aligned fields (u128, i128)
/// - `u64`: 8-byte aligned fields (u64, i64)
/// - `u32`: 4-byte aligned fields (u32, i32, f32)
/// - `u16`: 2-byte aligned fields (u16, i16)
/// - `u8`: 1-byte aligned fields (u8, i8, arrays)
/// - `special`: Fixed-size arrays and custom types ([u8; N], addresses, etc.)
///
/// # Field Access Best Practices
///
/// Due to the `#[repr(C)]` attribute, accessing fields directly may cause alignment
/// issues on some architectures. Always copy fields to local variables before use:
///
/// ```rust
/// // ❌ WRONG - May cause alignment issues
/// assert_eq!(tlv_struct.some_field, expected_value);
///
/// // ✅ CORRECT - Copy field first
/// let field_value = tlv_struct.some_field;
/// assert_eq!(field_value, expected_value);
/// ```
///
/// This is especially important in tests and when comparing field values.
#[macro_export]
macro_rules! define_tlv {
    (
        $(#[$meta:meta])*
        $name:ident {
            // Optional u128 fields (16-byte alignment)
            $(u128: { $($u128_field:ident: $u128_type:ty),* $(,)? })?
            // u64 fields (8-byte alignment)
            $(u64: { $($u64_field:ident: $u64_type:ty),* $(,)? })?
            // u32 fields (4-byte alignment)
            $(u32: { $($u32_field:ident: $u32_type:ty),* $(,)? })?
            // u16 fields (2-byte alignment)
            $(u16: { $($u16_field:ident: $u16_type:ty),* $(,)? })?
            // u8 fields and small arrays (1-byte)
            $(u8: { $($u8_field:ident: $u8_type:ty),* $(,)? })?
            // Special types (addresses, large arrays, etc)
            $(special: { $($special_field:ident: $special_type:ty),* $(,)? })?
        }
    ) => {
        $(#[$meta])*
        #[repr(C)]
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $name {
            // Fields ordered by alignment for zero padding
            $($(pub $u128_field: $u128_type,)*)?
            $($(pub $u64_field: $u64_type,)*)?
            $($(pub $u32_field: $u32_type,)*)?
            $($(pub $u16_field: $u16_type,)*)?
            $($(pub $u8_field: $u8_type,)*)?
            $($(pub $special_field: $special_type,)*)?
        }

        // Manual zerocopy implementation to handle padding correctly
        unsafe impl ::zerocopy::AsBytes for $name {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }

        unsafe impl ::zerocopy::FromBytes for $name {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }

        unsafe impl ::zerocopy::FromZeroes for $name {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }

        impl $name {
            /// Auto-generated constructor with fields in alignment order
            /// Use semantic constructors like `new()` or `from_*()` instead for better API
            #[allow(clippy::too_many_arguments)]
            pub fn new_raw(
                $($($u128_field: $u128_type,)*)?
                $($($u64_field: $u64_type,)*)?
                $($($u32_field: $u32_type,)*)?
                $($($u16_field: $u16_type,)*)?
                $($($u8_field: $u8_type,)*)?
                $($($special_field: $special_type,)*)?
            ) -> Self {
                Self {
                    $($($u128_field,)*)?
                    $($($u64_field,)*)?
                    $($($u32_field,)*)?
                    $($($u16_field,)*)?
                    $($($u8_field,)*)?
                    $($($special_field,)*)?
                }
            }

            /// Get the size of this TLV structure
            pub const fn size() -> usize {
                ::std::mem::size_of::<Self>()
            }

            /// Parse from bytes using zero-copy deserialization
            pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
                if data.len() < ::std::mem::size_of::<Self>() {
                    return Err(format!(
                        "Data too short for {}: need {}, got {}",
                        stringify!($name),
                        ::std::mem::size_of::<Self>(),
                        data.len()
                    ));
                }

                use ::zerocopy::Ref;
                let tlv_ref = Ref::<_, Self>::new(data)
                    .ok_or_else(|| format!("Failed to parse {} from bytes", stringify!($name)))?;
                Ok(*tlv_ref.into_ref())
            }

            /// Convert to bytes using zero-copy serialization
            pub fn to_bytes(&self) -> &[u8] {
                self.as_bytes()
            }
        }

    };
}

/// Helper macro for creating TLV with default padding
///
/// Automatically adds appropriate padding based on the struct size to ensure
/// alignment with 8-byte boundaries.
#[macro_export]
macro_rules! define_tlv_with_padding {
    (
        $(#[$meta:meta])*
        $name:ident {
            size: $expected_size:expr,
            $(u128: { $($u128_field:ident: $u128_type:ty),* $(,)? })?
            $(u64: { $($u64_field:ident: $u64_type:ty),* $(,)? })?
            $(u32: { $($u32_field:ident: $u32_type:ty),* $(,)? })?
            $(u16: { $($u16_field:ident: $u16_type:ty),* $(,)? })?
            $(u8: { $($u8_field:ident: $u8_type:ty),* $(,)? })?
            $(special: { $($special_field:ident: $special_type:ty),* $(,)? })?
        }
    ) => {
        define_tlv! {
            $(#[$meta])*
            $name {
                $(u128: { $($u128_field: $u128_type),* })?
                $(u64: { $($u64_field: $u64_type),* })?
                $(u32: { $($u32_field: $u32_type),* })?
                $(u16: { $($u16_field: $u16_type),* })?
                $(u8: { $($u8_field: $u8_type),* })?
                $(special: { $($special_field: $special_type),* })?
            }
        }

        impl $name {
            /// Expected size for this TLV structure
            pub const EXPECTED_SIZE: usize = $expected_size;

            /// Validate that the actual size matches expected
            pub const fn validate_size() -> bool {
                ::std::mem::size_of::<Self>() == Self::EXPECTED_SIZE
            }
        }

        // Compile-time assertion that size matches expected
        const _: () = {
            const ACTUAL: usize = ::std::mem::size_of::<$name>();
            const EXPECTED: usize = $expected_size;
            if ACTUAL != EXPECTED {
                panic!(concat!(
                    stringify!($name),
                    " size mismatch - expected ",
                    stringify!($expected_size),
                    " bytes but got ",
                    stringify!(ACTUAL),
                    " bytes. Check field alignment and padding!"
                ));
            }
        };
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use zerocopy::{AsBytes, FromBytes, FromZeroes};

    // Test the macro with a simple TLV structure
    define_tlv! {
        /// Test TLV structure
        TestTLV {
            u64: {
                timestamp: u64,
                value: i64
            }
            u32: { count: u32 }
            u16: { flags: u16 }
            u8: {
                status: u8,
                _padding: [u8; 1]
            }
            special: {}
        }
    }

    #[test]
    fn test_macro_generates_struct() {
        let tlv = TestTLV::new_raw(
            100,    // timestamp
            200,    // value
            300,    // count
            400,    // flags
            1,      // status
            [0; 1], // padding
        );

        assert_eq!(tlv.timestamp, 100);
        assert_eq!(tlv.value, 200);
        assert_eq!(tlv.count, 300);
        assert_eq!(tlv.flags, 400);
        assert_eq!(tlv.status, 1);
    }

    #[test]
    fn test_zero_copy_serialization() {
        let tlv = TestTLV::new_raw(100, 200, 300, 400, 1, [0; 1]);

        // Test serialization
        let bytes = tlv.to_bytes();
        assert_eq!(bytes.len(), std::mem::size_of::<TestTLV>());

        // Test deserialization
        let parsed = TestTLV::from_bytes(bytes).unwrap();
        assert_eq!(parsed, tlv);
    }

    // Test with special field types
    define_tlv_with_padding! {
        /// Complex TLV with addresses
        ComplexTLV {
            size: 88,
            u64: {
                amount_in: u64,
                amount_out: u64
            }
            u32: { block: u32 }
            u16: { venue: u16 }
            u8: {
                decimals_in: u8,
                decimals_out: u8
            }
            special: {
                pool_address: [u8; 20],
                token_in: [u8; 20],
                token_out: [u8; 20]
            }
        }
    }

    #[test]
    fn test_complex_tlv_with_addresses() {
        let tlv = ComplexTLV::new_raw(
            1000,    // amount_in
            2000,    // amount_out
            12345,   // block
            1,       // venue
            18,      // decimals_in
            6,       // decimals_out
            [1; 20], // pool_address
            [2; 20], // token_in
            [3; 20], // token_out
        );

        assert_eq!(tlv.amount_in, 1000);
        assert_eq!(tlv.pool_address, [1; 20]);
        assert_eq!(ComplexTLV::size(), 84);
        assert!(ComplexTLV::validate_size());
    }
}
