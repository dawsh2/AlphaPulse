use packed_struct::prelude::*;
use zerocopy::{AsBytes, FromBytes, FromZeroes};

/// Manual padding approach (current)
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
#[repr(C)]
pub struct TradeTLVManual {
    pub asset_id: u64,     // 8 bytes
    pub price: i64,        // 8 bytes  
    pub volume: i64,       // 8 bytes
    pub timestamp_ns: u64, // 8 bytes
    pub venue_id: u16,     // 2 bytes
    pub asset_type: u8,    // 1 byte
    pub reserved: u8,      // 1 byte
    pub side: u8,          // 1 byte
    pub _padding: [u8; 3], // Manual calculation: 40 - 37 = 3 bytes
}

/// Test struct using packed_struct - let's see what it generates
#[derive(PackedStruct, Debug, Clone, Copy, PartialEq)]
#[packed_struct(endian = "little", size_bytes = "40")]
pub struct TradeTLVPacked {
    #[packed_field(bytes = "0..8")]
    pub asset_id: u64,
    #[packed_field(bytes = "8..16")]
    pub price: i64,
    #[packed_field(bytes = "16..24")]
    pub volume: i64,
    #[packed_field(bytes = "24..32")]
    pub timestamp_ns: u64,
    #[packed_field(bytes = "32..34")]
    pub venue_id: u16,
    #[packed_field(bytes = "34")]
    pub asset_type: u8,
    #[packed_field(bytes = "35")]
    pub reserved: u8,
    #[packed_field(bytes = "36")]
    pub side: u8,
    // Bytes 37-39: Automatic padding (we hope!)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_size_equivalence() {
        println!("Manual struct size: {}", mem::size_of::<TradeTLVManual>());
        println!("Packed struct size: {}", mem::size_of::<TradeTLVPacked>());
        
        assert_eq!(
            mem::size_of::<TradeTLVManual>(),
            mem::size_of::<TradeTLVPacked>()
        );
    }

    #[test]
    fn test_alignment() {
        println!("Manual alignment: {}", mem::align_of::<TradeTLVManual>());
        println!("Packed alignment: {}", mem::align_of::<TradeTLVPacked>());
    }

    #[test]
    fn test_field_access_performance() {
        let manual = TradeTLVManual {
            asset_id: 12345,
            price: 100_000_000,
            volume: 50_000_000,
            timestamp_ns: 1234567890,
            venue_id: 1,
            asset_type: 1,
            reserved: 0,
            side: 0,
            _padding: [0; 3],
        };

        let packed = TradeTLVPacked {
            asset_id: 12345,
            price: 100_000_000,
            volume: 50_000_000,
            timestamp_ns: 1234567890,
            venue_id: 1,
            asset_type: 1,
            reserved: 0,
            side: 0,
        };

        // TEST: Can we directly access fields without copy?
        let manual_price = manual.price; // Direct field access
        let packed_price = packed.price; // Does this work the same way?
        
        assert_eq!(manual_price, packed_price);
    }

    #[test]
    fn test_packed_struct_methods() {
        let packed = TradeTLVPacked {
            asset_id: 12345,
            price: 100_000_000,
            volume: 50_000_000,
            timestamp_ns: 1234567890,
            venue_id: 1,
            asset_type: 1,
            reserved: 0,
            side: 0,
        };

        // TEST: What methods does packed_struct actually generate?
        let packed_bytes = packed.pack().expect("pack should work");
        println!("Packed bytes length: {}", packed_bytes.len());
        
        let unpacked = TradeTLVPacked::unpack(&packed_bytes).expect("unpack should work");
        assert_eq!(packed.asset_id, unpacked.asset_id);
        assert_eq!(packed.price, unpacked.price);
        
        // CRITICAL TEST: Does pack() require copying data?
        // If it returns Vec<u8> or [u8; N], that's a copy operation!
    }
}
