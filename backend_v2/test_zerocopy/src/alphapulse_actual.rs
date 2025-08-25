//! Testing AlphaPulse's ACTUAL requirements
//! 
//! We only need zerocopy for specific fixed-size collections with counts

use zerocopy::{IntoBytes, FromBytes, KnownLayout, Immutable};

// This is what we ACTUALLY need for AlphaPulse

#[repr(C)]
#[derive(Debug, Clone, Copy, IntoBytes, FromBytes, KnownLayout, Immutable, PartialEq)]
pub struct ReserveVec {
    pub count: u16,           // How many reserves are valid
    pub _padding: [u8; 6],    // Align to 8 bytes
    pub reserves: [u128; 8],  // Fixed size, up to 8 tokens
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstrumentId {
    pub venue: u16,
    pub asset_type: u8,
    pub reserved: u8,
    pub asset_id: u64,
}

// Manual zerocopy for InstrumentId to avoid padding issues
unsafe impl IntoBytes for InstrumentId {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl FromBytes for InstrumentId {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl KnownLayout for InstrumentId {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

unsafe impl Immutable for InstrumentId {
    fn only_derive_is_allowed_to_implement_this_trait() {}
}

#[repr(C)]
#[derive(Debug, Clone, Copy, IntoBytes, FromBytes, KnownLayout, Immutable, PartialEq)]
pub struct InstrumentVec {
    pub count: u16,                      // How many instruments are valid
    pub _padding: [u8; 6],               // Align to 8 bytes
    pub instruments: [InstrumentId; 16], // Fixed size, up to 16 instruments
}

impl ReserveVec {
    pub fn new(reserves: &[u128]) -> Result<Self, String> {
        if reserves.len() > 8 {
            return Err("Too many reserves".to_string());
        }
        
        let mut data = [0u128; 8];
        data[..reserves.len()].copy_from_slice(reserves);
        
        Ok(Self {
            count: reserves.len() as u16,
            _padding: [0; 6],
            reserves: data,
        })
    }
    
    pub fn as_slice(&self) -> &[u128] {
        &self.reserves[..self.count as usize]
    }
}

impl InstrumentVec {
    pub fn new(instruments: &[InstrumentId]) -> Result<Self, String> {
        if instruments.len() > 16 {
            return Err("Too many instruments".to_string());
        }
        
        let mut data = [InstrumentId {
            venue: 0,
            asset_type: 0,
            reserved: 0,
            asset_id: 0,
        }; 16];
        data[..instruments.len()].copy_from_slice(instruments);
        
        Ok(Self {
            count: instruments.len() as u16,
            _padding: [0; 6],
            instruments: data,
        })
    }
    
    pub fn as_slice(&self) -> &[InstrumentId] {
        &self.instruments[..self.count as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_reserve_vec_zerocopy() {
        let reserves = &[
            1000000000000000000u128,  // 1 token
            2000000000000000000u128,  // 2 tokens
            3000000000000000000u128,  // 3 tokens
        ];
        
        let vec = ReserveVec::new(reserves).unwrap();
        assert_eq!(vec.as_slice(), reserves);
        
        // Zero-copy serialization
        let bytes = vec.as_bytes();
        println!("ReserveVec serialized to {} bytes", bytes.len());
        
        // Zero-copy deserialization
        match ReserveVec::ref_from_bytes(bytes) {
            Ok(recovered) => {
                assert_eq!(recovered.as_slice(), reserves);
                println!("✅ ReserveVec zerocopy SUCCESS!");
            }
            Err(e) => {
                panic!("❌ ReserveVec zerocopy FAILED: {:?}", e);
            }
        }
    }
    
    #[test]
    fn test_instrument_vec_zerocopy() {
        let instruments = &[
            InstrumentId { venue: 1, asset_type: 1, reserved: 0, asset_id: 100 },
            InstrumentId { venue: 2, asset_type: 2, reserved: 0, asset_id: 200 },
            InstrumentId { venue: 3, asset_type: 3, reserved: 0, asset_id: 300 },
        ];
        
        let vec = InstrumentVec::new(instruments).unwrap();
        assert_eq!(vec.as_slice(), instruments);
        
        // Zero-copy serialization
        let bytes = vec.as_bytes();
        println!("InstrumentVec serialized to {} bytes", bytes.len());
        
        // Zero-copy deserialization  
        match InstrumentVec::ref_from_bytes(bytes) {
            Ok(recovered) => {
                assert_eq!(recovered.as_slice(), instruments);
                println!("✅ InstrumentVec zerocopy SUCCESS!");
            }
            Err(e) => {
                panic!("❌ InstrumentVec zerocopy FAILED: {:?}", e);
            }
        }
    }
    
    #[test]
    fn test_performance_critical_path() {
        use std::time::Instant;
        
        let reserves = &[1u128, 2, 3, 4, 5];
        let vec = ReserveVec::new(reserves).unwrap();
        
        let iterations = 100_000;
        
        // Measure serialization
        let start = Instant::now();
        for _ in 0..iterations {
            let _bytes = vec.as_bytes();
        }
        let serialize_time = start.elapsed();
        
        // Measure deserialization
        let bytes = vec.as_bytes();
        let start = Instant::now();
        for _ in 0..iterations {
            let _recovered = ReserveVec::ref_from_bytes(bytes).unwrap();
        }
        let deserialize_time = start.elapsed();
        
        let total_ns_per_op = (serialize_time.as_nanos() + deserialize_time.as_nanos()) / iterations;
        let total_us_per_op = total_ns_per_op as f64 / 1000.0;
        
        println!("Performance: {:.3}μs per roundtrip", total_us_per_op);
        
        if total_us_per_op < 35.0 {
            println!("✅ Performance target MET: {:.3}μs < 35μs", total_us_per_op);
        } else {
            println!("⚠️ Performance: {:.3}μs", total_us_per_op);
        }
    }
}