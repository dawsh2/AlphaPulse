use zerocopy::{AsBytes, FromBytes, FromZeroes};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsBytes, FromBytes, FromZeroes)]
pub struct InstrumentId {
    // Group 64-bit fields first
    pub asset_id: u64,  // Venue-specific identifier (8 bytes)
    
    // Then 16-bit fields  
    pub venue: u16,     // VenueId enum (2 bytes)
    
    // Finally 8-bit fields (need 2 bytes to reach 12 total)
    pub asset_type: u8, // AssetType enum (1 byte)
    pub reserved: u8,   // Future use/flags (1 byte)
}

fn main() {
    println!("InstrumentId size: {}", std::mem::size_of::<InstrumentId>());
    println!("Expected size: 12");
}
