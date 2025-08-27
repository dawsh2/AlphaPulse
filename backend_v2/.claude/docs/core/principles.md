# High-Leverage Engineering Patterns

**Philosophy**: Focus on patterns that automatically enforce good practices. When you use these correctly, principles like DRY, SRP, and KISS follow naturally.

## 🌱 Code Organization & Maintainability

### 1. Use clear module boundaries
Organize code into modules that mirror the domain, not arbitrary files.
Avoid "God modules" (e.g. utils.rs). Instead:
```rust
mod address;
mod pool;
mod sync;
```
Use `pub use` to re-export only what's part of the public API.

### 2. Prefer composition over macros
Derives (`#[derive(...)]`) are idiomatic, but avoid custom procedural macros unless they're saving you tons of boilerplate.
Traits + generics usually make intent clearer than "magic macros."

### 3. Document at the module and type level
Add `//!` at the top of a module to explain its role.
For public APIs, use `///` comments, examples, and invariants.

### 4. Extension Traits for Adding Behavior
Add methods to existing types without wrapping - forces centralization and prevents duplication.

```rust
pub trait AddressConversion {
    fn to_padded(&self) -> [u8; 32];
    fn from_padded(padded: &[u8; 32]) -> Self;
}

impl AddressConversion for [u8; 20] {
    fn to_padded(&self) -> [u8; 32] {
        let mut padded = [0u8; 32];
        padded[..20].copy_from_slice(self);
        padded
    }
    
    fn from_padded(padded: &[u8; 32]) -> Self {
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&padded[..20]);
        addr
    }
}

// Now ANY [u8; 20] gets these methods - no duplication possible
```

## 🧩 API & Interface Design

### 5. Narrow public APIs
Use `pub(crate)` and `pub(super)` liberally.
Only expose what consumers need, not internal helpers.
This keeps refactoring safe and your API surface small.

### 6. Traits for behavior, types for data
Traits give you clean extension points:
```rust
trait AddressConversion {
    fn to_padded(&self) -> [u8; 32];
}
```
Avoid traits as "just interfaces" if a simple function works. Use traits when you need polymorphism.

### 7. Use newtypes for domain clarity
Instead of `String` or `[u8; 32]` everywhere:
```rust
struct EthAddress([u8; 20]);
struct PoolId(u64);
```
This avoids accidental mixing and makes signatures self-documenting.

### 8. From/Into Traits for Canonical Conversions
Implement `From` for infallible conversions, `TryFrom` for fallible ones.

```rust
impl From<[u8; 20]> for Address {
    fn from(bytes: [u8; 20]) -> Self {
        Address(bytes)
    }
}

impl TryFrom<&str> for Address {
    type Error = ParseError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        // Parse logic here - only ONE place
    }
}

// Usage is natural and consistent
let addr1 = Address::from(bytes);
let addr2: Address = bytes.into();
let addr3 = Address::try_from("0x123...")?;
```

## ⚖️ Correctness & Safety

### 9. Leverage the type system
Use enums with `#[non_exhaustive]` for open sets.
Prefer `Option`/`Result` over sentinel values.
Encode invariants in types (e.g., `NonZeroU64`, `Duration`).

### 10. Make invalid states impossible
Use the type system to prevent bugs at compile time.

```rust
// ❌ WRONG: Runtime validation needed
struct Order {
    status: String,  // "pending", "filled" - error prone
}

// ✅ CORRECT: Invalid states can't exist
enum OrderStatus {
    Pending,
    Filled { price: Price, timestamp: i64 },
    Cancelled { reason: CancelReason },
}

struct Order {
    status: OrderStatus,  // Can only be valid states
}
```

### 11. Immutability by default
Keep fields private, expose getters where necessary.
Don't make a field `pub` unless it's truly part of the contract.

### 12. Zero-cost abstraction first
If a "safe wrapper" can be expressed as a newtype or trait, do that instead of raw pointers or unsafe.

```rust
// Type wrapper with zero runtime cost
#[repr(transparent)]
#[derive(AsBytes, FromBytes)]
pub struct PaddedAddress([u8; 32]);

impl PaddedAddress {
    #[inline(always)]  // Compiles to nothing
    pub fn from_eth(addr: [u8; 20]) -> Self {
        let mut padded = [0u8; 32];
        padded[..20].copy_from_slice(&addr);
        Self(padded)
    }
}
```

Only drop into `unsafe` for very specific, measured reasons.

## 🚦 Testing & Reliability

### 13. Unit + property tests
Use `#[cfg(test)]` + `mod tests` colocated with the code.
Quick unit tests for invariants, but also property-based tests with proptest.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn roundtrip_conversion(addr in any::<[u8; 20]>()) {
        let padded = addr_to_32(addr);
        let recovered = addr_from_32(&padded);
        assert_eq!(addr, recovered);  // Must always be true
    }
}
```

### 14. Compile-time checks
Use `static_assertions::assert_eq_size!` for zero-copy layouts.
Use `const fn` and `const generics` to enforce compile-time invariants.

```rust
pub struct FixedBuffer<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> FixedBuffer<N> {
    pub fn write(&mut self, offset: usize, byte: u8) {
        self.data[offset] = byte;  // Bounds check optimized away
    }
}
```

### 15. Fuzzing & integration tests
Rust integrates great with fuzzers (`cargo fuzz`).
For protocols/serialization, fuzzers catch edge cases you won't think of.

### 16. Table-driven tests for comprehensive coverage
```rust
#[test]
fn test_price_conversion() {
    let cases = [
        (100.0, 10_000_000_000),
        (0.01, 1_000_000),
        (45_000.0, 4_500_000_000_000),
    ];
    
    for (input, expected) in cases {
        assert_eq!(to_fixed_point(input), expected);
    }
}
```

## 📦 Ecosystem & Tooling

### 17. Follow Rust's ecosystem conventions
Use `serde` for serialization unless there's a very good reason not to.
Use `thiserror` for error handling.
Use `tracing` for logging in async/microservice contexts.

### 18. Clippy + Rustfmt always on
Add `#![deny(clippy::all)]` or at least `#![warn(clippy::pedantic)]` in CI.
`cargo fmt` in CI to keep diffs clean.

### 19. Crates for shared patterns
If logic is shared between services, extract it into a crate in your workspace.
Don't repeat "address conversion" in 10 places.

### 20. Iterator chains over loops
Composable operations naturally prevent code duplication.

```rust
// Operations compose naturally - no duplication needed
let results: Vec<_> = items
    .into_iter()
    .filter(Item::is_valid)      // Reusable predicate
    .map(process_item)           // Reusable transformation  
    .filter(|p| p.value > threshold)  // Reusable filter
    .collect();
```

### 21. Option/Result combinators
Chain operations without nested matches.

```rust
// Chain operations naturally - prevents error handling duplication
fn get_config_value(key: &str) -> Option<i32> {
    read_config()?
        .get(key)?
        .parse()
        .ok()
}
```

## ✨ Human Factors

### 22. Error messages for humans
Define custom error types per module (`enum Error { … }`).
Implement `Display` to make them readable in logs.

### 23. Don't over-generalize too early
Start concrete. Generalize only when you see repeated patterns.
Rust's type system is very powerful, but it can hurt readability if you make everything generic too soon.

### 24. Code review hygiene
Encourage reviewers to look at:
- API surface: is it minimal?
- Types: do they express intent?
- Safety: is there any `unsafe` and is it justified?

### 25. Clone-on-Write for efficiency
Avoid unnecessary clones automatically.

```rust
use std::borrow::Cow;

fn process_text<'a>(input: &'a str, transform: bool) -> Cow<'a, str> {
    if transform {
        Cow::Owned(input.to_uppercase())  // Clone only when needed
    } else {
        Cow::Borrowed(input)  // No clone
    }
}
```

## Anti-Patterns to Avoid

### 1. Stringly-Typed Programming
```rust
// ❌ WRONG: Strings for everything
fn process(action: &str, data: &str) -> String

// ✅ CORRECT: Proper types
fn process(action: Action, data: &Data) -> Result<Output, Error>
```

### 2. Boolean Blindness
```rust
// ❌ WRONG: What do these mean?
configure(true, false, true, false);

// ✅ CORRECT: Named struct
configure(Config {
    enable_cache: true,
    use_compression: false,
    verify_ssl: true,
    debug_mode: false,
});
```

### 3. Primitive Obsession
```rust
// ❌ WRONG: Primitives for domain concepts
fn calculate_profit(buy: f64, sell: f64, quantity: i32) -> f64

// ✅ CORRECT: Domain types
fn calculate_profit(buy: Price, sell: Price, quantity: Quantity) -> Profit
```

## Summary

These patterns automatically enforce:
- **DRY**: Extension traits prevent method duplication
- **Type Safety**: Newtypes prevent argument mixing  
- **Performance**: Zero-cost abstractions give safety without overhead
- **Maintainability**: Single canonical implementations
- **Correctness**: Invalid states become impossible

**Remember**: Good patterns make bad code hard to write.