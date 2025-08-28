//! Typed ID Bridge for TLV Structures
//!
//! This module provides a macro to bridge between raw `u64` IDs in TLV structs
//! and the strongly-typed IDs used in application logic.
//!
//! ## The Problem This Solves
//!
//! Low-level TLV structs must use primitive types like `u64` to be compatible with
//! zerocopy for high-performance serialization. However, application logic needs
//! strongly-typed IDs like `SignalId` and `PoolId` to prevent bugs. This module
//! provides the bridge between these two worlds.
//!
//! ## Usage
//!
//! ```rust
//! use torq_types::{with_typed_ids, SignalId, PoolId, StrategyId};
//!
//! // TLV struct remains unchanged with raw u64 fields for zerocopy
//! #[repr(C)]
//! #[derive(AsBytes, FromBytes, FromZeroes)]
//! pub struct ArbitrageSignalTLV {
//!     pub signal_id: u64,
//!     pub pool_a_id: u64,
//!     pub pool_b_id: u64,
//!     pub strategy_id: u64,
//!     // ... other fields
//! }
//!
//! // Add typed bridge methods with a single declarative block
//! with_typed_ids!(ArbitrageSignalTLV,
//!     signal_id   -> SignalId,
//!     pool_a_id   -> PoolId,
//!     pool_b_id   -> PoolId,
//!     strategy_id -> StrategyId
//! );
//!
//! // Now you can use type-safe methods:
//! let signal_tlv = ArbitrageSignalTLV { signal_id: 12345, /* ... */ };
//! let typed_signal: SignalId = signal_tlv.signal_id_typed();
//! ```
//!
//! ## Benefits
//!
//! - **DRY**: Declare field-to-type relationships in one clear line each
//! - **Zero-Cost**: `#[inline(always)]` ensures no runtime overhead
//! - **Maintainable**: Add new typed fields by adding one line to the macro
//! - **Readable**: Serves as clear documentation of intended field types

#[macro_export]
macro_rules! with_typed_ids {
    (
        $struct_name:ty,
        $($field:ident -> $type:ty),*
        $(,)?
    ) => {
        impl $struct_name {
            $(
                paste::paste! {
                    #[inline(always)]
                    pub fn [<$field _typed>](&self) -> $type {
                        <$type>::from(self.$field)
                    }

                    #[inline(always)]
                    pub fn [<set_ $field _typed>](&mut self, id: $type) {
                        self.$field = id.into();
                    }

                    pub fn [<validate_ $field>](&self) -> Result<(), crate::common::errors::ValidationError> {
                        <$type>::new_validated(self.$field)?;
                        Ok(())
                    }
                }
            )*

            pub fn validate_all_typed_ids(&self) -> Result<(), crate::common::errors::ValidationError> {
                $(paste::paste! { self.[<validate_ $field>]()?; })*
                Ok(())
            }

            pub fn extract_all_typed_ids(&self) -> ($($type,)*) {
                (
                    $(
                        paste::paste! {
                            self.[<$field _typed>]()
                        },
                    )*
                )
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::identifiers::{PoolId, SignalId, StrategyId};
    use crate::define_tlv;
    use zerocopy::AsBytes;

    #[test]
    fn test_with_typed_ids_macro() {
        // Create a test TLV structure
        define_tlv! {
            /// Test TLV for demonstrating typed bridge
            TestSignalTLV {
                u64: {
                    signal_id: u64,
                    pool_a_id: u64,
                    pool_b_id: u64,
                    strategy_id: u64
                }
                u32: {}
                u16: {}
                u8: {}
                special: {}
            }
        }

        // Apply the elegant typed bridge macro
        with_typed_ids!(TestSignalTLV,
            signal_id   -> SignalId,
            pool_a_id   -> PoolId,
            pool_b_id   -> PoolId,
            strategy_id -> StrategyId
        );

        // Test the generated methods
        let mut signal_tlv = TestSignalTLV::new_raw(12345, 67890, 11111, 22222);

        // Test typed getters
        assert_eq!(signal_tlv.signal_id_typed(), SignalId::new(12345));
        assert_eq!(signal_tlv.pool_a_id_typed(), PoolId::new(67890));
        assert_eq!(signal_tlv.pool_b_id_typed(), PoolId::new(11111));
        assert_eq!(signal_tlv.strategy_id_typed(), StrategyId::new(22222));

        // Test typed setters
        signal_tlv.set_signal_id_typed(SignalId::new(99999));
        signal_tlv.set_pool_a_id_typed(PoolId::new(88888));
        signal_tlv.set_pool_b_id_typed(PoolId::new(77777));
        signal_tlv.set_strategy_id_typed(StrategyId::new(66666));

        // Verify raw fields were updated
        assert_eq!(signal_tlv.signal_id, 99999);
        assert_eq!(signal_tlv.pool_a_id, 88888);
        assert_eq!(signal_tlv.pool_b_id, 77777);
        assert_eq!(signal_tlv.strategy_id, 66666);

        // Test individual validation
        assert!(signal_tlv.validate_signal_id().is_ok());
        assert!(signal_tlv.validate_pool_a_id().is_ok());
        assert!(signal_tlv.validate_pool_b_id().is_ok());
        assert!(signal_tlv.validate_strategy_id().is_ok());

        // Test bulk validation
        assert!(signal_tlv.validate_all_typed_ids().is_ok());

        // Test bulk extraction
        let (signal, pool_a, pool_b, strategy) = signal_tlv.extract_all_typed_ids();
        assert_eq!(signal, SignalId::new(99999));
        assert_eq!(pool_a, PoolId::new(88888));
        assert_eq!(pool_b, PoolId::new(77777));
        assert_eq!(strategy, StrategyId::new(66666));
    }

    #[test]
    fn test_validation_errors() {
        define_tlv! {
            ValidationTestTLV {
                u64: { test_id: u64 }
                u32: {}
                u16: {}
                u8: {}
                special: {}
            }
        }

        with_typed_ids!(ValidationTestTLV,
            test_id -> SignalId
        );

        // Valid TLV
        let valid_tlv = ValidationTestTLV::new_raw(12345);
        assert!(valid_tlv.validate_test_id().is_ok());
        assert!(valid_tlv.validate_all_typed_ids().is_ok());

        // Invalid TLV with null ID
        let invalid_tlv = ValidationTestTLV::new_raw(0);
        assert!(invalid_tlv.validate_test_id().is_err());
        assert!(invalid_tlv.validate_all_typed_ids().is_err());
    }

    #[test]
    fn test_zero_cost_abstraction() {
        define_tlv! {
            PerfTestTLV {
                u64: { signal_id: u64 }
                u32: {}
                u16: {}
                u8: {}
                special: {}
            }
        }

        with_typed_ids!(PerfTestTLV,
            signal_id -> SignalId
        );

        let tlv = PerfTestTLV::new_raw(42);

        // Direct field access
        let raw_value = tlv.signal_id;

        // Typed access - should compile to identical assembly
        let typed_value = tlv.signal_id_typed();

        assert_eq!(raw_value, typed_value.inner());

        // Verify no size overhead
        assert_eq!(std::mem::size_of::<PerfTestTLV>(), 8); // Just 1 u64
    }

    #[test]
    fn test_real_world_usage_pattern() {
        // Simulate a real TLV structure from the codebase
        define_tlv! {
            /// Arbitrage signal TLV with multiple ID fields
            RealArbitrageSignalTLV {
                u64: {
                    signal_id: u64,
                    pool_a_id: u64,
                    pool_b_id: u64,
                    strategy_id: u64,
                    profit_potential: i64,
                    timestamp_ns: u64
                }
                u32: {}
                u16: { venue_id: u16 }
                u8: {
                    status: u8,
                    _padding: [u8; 1]
                }
                special: {}
            }
        }

        // Add typed bridges for just the ID fields
        with_typed_ids!(RealArbitrageSignalTLV,
            signal_id   -> SignalId,
            pool_a_id   -> PoolId,
            pool_b_id   -> PoolId,
            strategy_id -> StrategyId
        );

        // Usage in service code
        let signal_tlv = RealArbitrageSignalTLV::new_raw(
            12345,               // signal_id
            67890,               // pool_a_id
            11111,               // pool_b_id
            22222,               // strategy_id
            1000,                // profit_potential (not typed)
            1640995200000000000, // timestamp_ns (not typed)
            1,                   // venue_id (not typed)
            0,                   // status (not typed)
            [0; 1],              // padding (not typed)
        );

        // Service layer gets type safety for ID fields
        let signal = signal_tlv.signal_id_typed();
        let pool_a = signal_tlv.pool_a_id_typed();
        let pool_b = signal_tlv.pool_b_id_typed();
        let strategy = signal_tlv.strategy_id_typed();

        // Raw fields still accessible for non-ID data
        let profit = signal_tlv.profit_potential;
        let timestamp = signal_tlv.timestamp_ns;
        let venue = signal_tlv.venue_id;

        // Type safety prevents confusion
        fn process_arbitrage(
            signal: SignalId,
            pool_a: PoolId,
            pool_b: PoolId,
            strategy: StrategyId,
        ) -> bool {
            // Compiler prevents: process_arbitrage(pool_a, signal, pool_b, strategy)
            true
        }

        assert!(process_arbitrage(signal, pool_a, pool_b, strategy));

        // Validate all typed fields at once
        assert!(signal_tlv.validate_all_typed_ids().is_ok());
    }
}
