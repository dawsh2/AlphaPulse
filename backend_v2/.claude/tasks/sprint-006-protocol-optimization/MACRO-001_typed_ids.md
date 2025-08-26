---
task_id: MACRO-001
status: TODO
priority: CRITICAL
estimated_hours: 4
assigned_branch: feat/typed-id-wrappers
assignee: TBD
created: 2025-08-26
completed: null
---

# MACRO-001: Typed ID Wrappers

## Task Overview
**Sprint**: 006-protocol-optimization
**Priority**: CRITICAL  
**Estimate**: 4 hours
**Status**: TODO
**Goal**: Eliminate entire class of ID confusion bugs through compile-time type safety

## Problem
Currently using raw `u64` for all IDs (pool_id, signal_id, strategy_id, etc.), making it trivially easy to pass wrong ID type to functions. The compiler can't help because they're all just `u64`.

## Real Bug Examples Found
```rust
// BUG: Swapped parameters - compiles but wrong!
fn execute_arbitrage(pool_id: u64, signal_id: u64, strategy_id: u64) { }
execute_arbitrage(signal_id, pool_id, strategy_id); // WRONG ORDER!

// BUG: Using wrong ID type entirely
let pool_id = 12345;
let signal_id = 67890;
process_signal(pool_id); // Should be signal_id!
```

## Solution
Create zero-cost newtype wrappers that make these bugs impossible.

## Implementation

### Step 1: Create Macro
```rust
// libs/types/src/identifiers.rs

#[macro_export]
macro_rules! define_typed_id {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(
            Debug, 
            Clone, 
            Copy, 
            PartialEq, 
            Eq, 
            PartialOrd, 
            Ord,
            Hash
        )]
        #[repr(transparent)] // Same memory layout as u64
        pub struct $name(pub u64);
        
        impl $name {
            /// Create a new ID
            pub const fn new(id: u64) -> Self {
                Self(id)
            }
            
            /// Extract the inner u64 value
            pub const fn inner(&self) -> u64 {
                self.0
            }
            
            /// Generate next sequential ID
            pub fn next(&self) -> Self {
                Self(self.0 + 1)
            }
        }
        
        // Display for debugging
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
        
        // Conversions
        impl From<u64> for $name {
            fn from(id: u64) -> Self {
                Self(id)
            }
        }
        
        impl From<$name> for u64 {
            fn from(id: $name) -> u64 {
                id.0
            }
        }
        
        // Serialization support
        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                self.0.serialize(serializer)
            }
        }
        
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                u64::deserialize(deserializer).map(Self)
            }
        }
        
        // Database support (sqlx)
        #[cfg(feature = "sqlx")]
        impl<'r> sqlx::Decode<'r, sqlx::Postgres> for $name {
            fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
                let id = <i64 as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
                Ok(Self(id as u64))
            }
        }
        
        #[cfg(feature = "sqlx")]
        impl sqlx::Encode<'_, sqlx::Postgres> for $name {
            fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
                (self.0 as i64).encode_by_ref(buf)
            }
        }
    };
}
```

### Step 2: Generate All ID Types
```rust
// libs/types/src/identifiers.rs

// Pool-related IDs
define_typed_id!(
    /// Unique identifier for a liquidity pool
    PoolId
);

define_typed_id!(
    /// Unique identifier for a pool pair
    PoolPairId
);

// Signal-related IDs
define_typed_id!(
    /// Unique identifier for a trading signal
    SignalId
);

define_typed_id!(
    /// Unique identifier for an arbitrage opportunity
    OpportunityId
);

// Strategy-related IDs
define_typed_id!(
    /// Unique identifier for a trading strategy
    StrategyId
);

define_typed_id!(
    /// Unique identifier for a strategy instance
    StrategyInstanceId
);

// Execution-related IDs
define_typed_id!(
    /// Unique identifier for an order
    OrderId
);

define_typed_id!(
    /// Unique identifier for a trade execution
    TradeId
);

// System-related IDs
define_typed_id!(
    /// Unique identifier for an actor
    ActorId
);

define_typed_id!(
    /// Unique identifier for a session
    SessionId
);

define_typed_id!(
    /// Unique identifier for an instrument
    InstrumentId
);
```

### Step 3: Update Function Signatures
```rust
// BEFORE: Confusing, error-prone
fn process_arbitrage(
    pool_id: u64, 
    signal_id: u64, 
    strategy_id: u64
) -> Result<u64> { // What does this u64 return?
    // ...
}

// AFTER: Clear, type-safe
fn process_arbitrage(
    pool: PoolId,
    signal: SignalId,
    strategy: StrategyId  
) -> Result<OrderId> { // Clear return type!
    // ...
}
```

### Step 4: Fix Compilation Errors (Finding Bugs!)
```rust
// This will now fail to compile:
let pool_id = PoolId::new(123);
let signal_id = SignalId::new(456);

// ERROR: expected SignalId, found PoolId
process_signal(pool_id); // Caught at compile time!

// ERROR: wrong parameter order
execute_arbitrage(signal_id, pool_id, strategy_id); // Caught!
```

## Migration Strategy

### Phase 1: Add Types
1. Create all typed ID structs
2. Add conversion methods for gradual migration

### Phase 2: Update Core Structures
```rust
// Update TLV structures
pub struct PoolSwapTLV {
    pub pool_id: PoolId,  // Was: u64
    // ...
}

pub struct ArbitrageSignalTLV {
    pub signal_id: SignalId,  // Was: u64
    pub pool_a: PoolId,       // Was: u64
    pub pool_b: PoolId,       // Was: u64
    // ...
}
```

### Phase 3: Service Migration
Update each service to use typed IDs:
- MarketDataProcessor
- SignalGenerator
- ArbitrageDetector
- ExecutionEngine

## TDD Test Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_id_creation() {
        let pool_id = PoolId::new(123);
        assert_eq!(pool_id.inner(), 123);
        assert_eq!(pool_id.to_string(), "PoolId(123)");
    }

    #[test]
    fn test_typed_id_equality() {
        let id1 = SignalId::new(42);
        let id2 = SignalId::new(42);
        let id3 = SignalId::new(43);
        
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_typed_id_ordering() {
        let id1 = OrderId::new(1);
        let id2 = OrderId::new(2);
        
        assert!(id1 < id2);
    }

    #[test]
    fn test_typed_id_hashing() {
        use std::collections::HashMap;
        
        let mut map = HashMap::new();
        map.insert(PoolId::new(1), "pool_one");
        
        assert_eq!(map.get(&PoolId::new(1)), Some(&"pool_one"));
    }

    #[test]
    fn test_typed_id_serialization() {
        let id = StrategyId::new(999);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "999");
        
        let recovered: StrategyId = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, id);
    }

    #[test]
    fn test_zero_cost_abstraction() {
        // Verify no runtime overhead
        assert_eq!(
            std::mem::size_of::<PoolId>(),
            std::mem::size_of::<u64>()
        );
        
        // Verify transparent representation
        let id = SignalId::new(42);
        let raw_ptr = &id as *const SignalId as *const u64;
        unsafe {
            assert_eq!(*raw_ptr, 42);
        }
    }

    // This should NOT compile (verified manually)
    // #[test]
    // fn test_type_safety() {
    //     let pool_id = PoolId::new(1);
    //     let signal_id = SignalId::new(2);
    //     
    //     fn takes_pool_id(_: PoolId) {}
    //     takes_pool_id(signal_id); // COMPILE ERROR!
    // }
}
```

## Benefits
1. **Compile-Time Safety**: Impossible to mix up ID types
2. **Zero Runtime Cost**: `#[repr(transparent)]` ensures same performance
3. **Self-Documenting**: Function signatures clearly show what IDs expected
4. **Debugging**: Display implementation shows type in logs
5. **Serialization**: Works seamlessly with serde, databases

## Validation Checklist
- [ ] All ID types defined with macro
- [ ] Conversion traits implemented
- [ ] Services updated to use typed IDs  
- [ ] Compilation errors fixed (bugs found!)
- [ ] Tests demonstrate type safety
- [ ] Zero runtime overhead verified
- [ ] Documentation updated

## Definition of Done
- Macro implemented and tested
- All u64 IDs replaced with typed versions
- No possibility of ID confusion bugs
- Performance unchanged (zero-cost verified)
- Migration guide written
- Team trained on new pattern