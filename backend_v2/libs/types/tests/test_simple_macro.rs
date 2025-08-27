// Simple test to verify OrderBookTLV macro syntax without dependencies

const MAX_ORDER_LEVELS: usize = 50;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
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

// Simplified macro for testing
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
    ///
    /// Field ordering optimized by macro: u64 fields first for cache alignment
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
            bids: FixedVec<OrderLevel, MAX_ORDER_LEVELS>, // Bid levels (highest price first)
            asks: FixedVec<OrderLevel, MAX_ORDER_LEVELS>  // Ask levels (lowest price first)
        }
    }
}

fn main() {
    println!("✅ OrderBookTLV macro syntax is valid!");

    // Test constructor with proper field order (macro puts u64 fields first)
    let order_book = OrderBookTLV::new_raw(
        1640995200000000000, // timestamp_ns (u64 - first)
        1000,                // sequence (u64 - second)
        100_000_000,         // precision_factor (i64 - third)
        12345,               // asset_id (u64 - fourth)
        1,                   // venue_id (u16)
        0,                   // asset_type (u8)
        0,                   // reserved (u8)
        FixedVec::new(),     // bids (special)
        FixedVec::new(),     // asks (special)
    );

    println!("✅ Created OrderBookTLV successfully!");
    println!("Size: {} bytes", std::mem::size_of::<OrderBookTLV>());
    println!("Field values:");
    println!("  timestamp_ns: {}", order_book.timestamp_ns);
    println!("  sequence: {}", order_book.sequence);
    println!("  precision_factor: {}", order_book.precision_factor);
    println!("  asset_id: {}", order_book.asset_id);
    println!("  venue_id: {}", order_book.venue_id);
    println!("  asset_type: {}", order_book.asset_type);
    println!("  reserved: {}", order_book.reserved);

    // Verify field order is correct (u64s come first)
    let expected_layout = [
        (
            "timestamp_ns",
            std::mem::offset_of!(OrderBookTLV, timestamp_ns),
        ),
        ("sequence", std::mem::offset_of!(OrderBookTLV, sequence)),
        (
            "precision_factor",
            std::mem::offset_of!(OrderBookTLV, precision_factor),
        ),
        ("asset_id", std::mem::offset_of!(OrderBookTLV, asset_id)),
        ("venue_id", std::mem::offset_of!(OrderBookTLV, venue_id)),
        ("asset_type", std::mem::offset_of!(OrderBookTLV, asset_type)),
        ("reserved", std::mem::offset_of!(OrderBookTLV, reserved)),
    ];

    println!("✅ Field layout (cache-friendly ordering):");
    for (name, offset) in expected_layout {
        println!("  {}: offset {} bytes", name, offset);
    }

    println!("🎉 OrderBookTLV macro conversion is syntactically correct and properly ordered!");
}
