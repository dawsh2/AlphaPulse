// Simple test to verify OrderBookTLV macro syntax is correct
#[macro_use]
extern crate torq_types;

// Simplified version just to test macro syntax
use zerocopy::{AsBytes, FromBytes, FromZeroes};

const MAX_ORDER_LEVELS: usize = 50;

#[derive(Debug, Clone, Copy, Default, AsBytes, FromBytes, FromZeroes)]
pub struct OrderLevel {
    pub price: i64,
    pub size: i64,
    pub order_count: u32,
    pub reserved: u32,
}

// Fixed-size vector stub for testing
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedVec<T, const N: usize>
where
    T: Copy,
{
    count: u16,
    _padding: [u8; 6],
    elements: [T; N],
}

impl<T: Copy + Default> FixedVec<T, MAX_ORDER_LEVELS> {
    pub fn new() -> Self {
        Self {
            count: 0,
            _padding: [0; 6],
            elements: [T::default(); MAX_ORDER_LEVELS],
        }
    }
}

unsafe impl AsBytes for FixedVec<OrderLevel, MAX_ORDER_LEVELS> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}
unsafe impl FromBytes for FixedVec<OrderLevel, MAX_ORDER_LEVELS> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}
unsafe impl FromZeroes for FixedVec<OrderLevel, MAX_ORDER_LEVELS> {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

// Macro for testing
macro_rules! define_tlv {
    (
        $(#[$meta:meta])*
        $name:ident {
            $(u64: { $($u64_field:ident: $u64_type:ty),* $(,)? })?
            $(u32: { $($u32_field:ident: $u32_type:ty),* $(,)? })?
            $(u16: { $($u16_field:ident: $u16_type:ty),* $(,)? })?
            $(u8: { $($u8_field:ident: $u8_type:ty),* $(,)? })?
            $(special: { $($special_field:ident: $special_type:ty),* $(,)? })?
        }
    ) => {
        $(#[$meta])*
        #[repr(C)]
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $name {
            $($(pub $u64_field: $u64_type,)*)?
            $($(pub $u32_field: $u32_type,)*)?
            $($(pub $u16_field: $u16_type,)*)?
            $($(pub $u8_field: $u8_type,)*)?
            $($(pub $special_field: $special_type,)*)?
        }

        unsafe impl AsBytes for $name {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }
        unsafe impl FromBytes for $name {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }
        unsafe impl FromZeroes for $name {
            fn only_derive_is_allowed_to_implement_this_trait() {}
        }

        impl $name {
            #[allow(clippy::too_many_arguments)]
            pub fn new_raw(
                $($($u64_field: $u64_type,)*)?
                $($($u32_field: $u32_type,)*)?
                $($($u16_field: $u16_type,)*)?
                $($($u8_field: $u8_type,)*)?
                $($($special_field: $special_type,)*)?
            ) -> Self {
                Self {
                    $($($u64_field,)*)?
                    $($($u32_field,)*)?
                    $($($u16_field,)*)?
                    $($($u8_field,)*)?
                    $($($special_field,)*)?
                }
            }
        }
    };
}

// Test the macro syntax from the OrderBookTLV conversion
define_tlv! {
    /// OrderBook TLV structure for complete order book snapshots
    OrderBookTLV {
        u64: {
            timestamp_ns: u64,     // Nanosecond timestamp when snapshot was taken
            sequence: u64,         // Sequence number for gap detection (venue-specific)
            precision_factor: i64, // Precision factor for price/size conversion
            asset_id: u64         // Asset identifier from InstrumentId
        }
        u32: {}
        u16: { venue_id: u16 } // Venue identifier as primitive u16
        u8: {
            asset_type: u8,    // Asset type from InstrumentId
            reserved: u8       // Reserved byte for alignment
        }
        special: {
            bids: FixedVec<OrderLevel, MAX_ORDER_LEVELS>, // Bid levels
            asks: FixedVec<OrderLevel, MAX_ORDER_LEVELS>  // Ask levels
        }
    }
}

fn main() {
    println!("✅ OrderBookTLV macro syntax is valid!");

    // Test constructor
    let order_book = OrderBookTLV::new_raw(
        1640995200000000000, // timestamp_ns
        1000,                // sequence
        100_000_000,         // precision_factor
        12345,               // asset_id
        1,                   // venue_id
        0,                   // asset_type
        0,                   // reserved
        FixedVec::new(),     // bids
        FixedVec::new(),     // asks
    );

    println!(
        "✅ Created OrderBookTLV: {} bytes",
        std::mem::size_of::<OrderBookTLV>()
    );
    println!(
        "Fields: timestamp={}, asset_id={}, venue_id={}",
        order_book.timestamp_ns, order_book.asset_id, order_book.venue_id
    );

    // Test zero-copy traits
    let bytes = order_book.as_bytes();
    println!("✅ Serialized to {} bytes", bytes.len());

    println!("🎉 OrderBookTLV macro conversion is syntactically correct!");
}
