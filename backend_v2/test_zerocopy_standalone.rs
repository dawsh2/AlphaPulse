use zerocopy::{AsBytes, FromBytes, FromZeroes};

#[repr(C)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes, PartialEq)]
pub struct TestFixedVec<T: Copy + Default + AsBytes + FromBytes + FromZeroes, const N: usize> {
    len: u16,          
    _padding: [u8; 6], 
    data: [T; N],      
}

impl<T: Copy + Default + AsBytes + FromBytes + FromZeroes, const N: usize> TestFixedVec<T, N> {
    pub fn new(slice: &[T]) -> Result<Self, &'static str> {
        if slice.len() > N {
            return Err("slice too long");
        }
        let mut data = [T::default(); N];
        data[..slice.len()].copy_from_slice(slice);
        Ok(Self { 
            len: slice.len() as u16, 
            _padding: [0; 6],
            data,
        })
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data[..self.len as usize]
    }
}

fn main() {
    println!("Testing zerocopy const generic derives...");
    
    let original = &[1u64, 2, 3, 4, 5];
    let fixed: TestFixedVec<u64, 8> = TestFixedVec::new(original).unwrap();

    // Zero-copy cast to bytes
    let bytes: &[u8] = fixed.as_bytes();
    println!("Serialized to {} bytes", bytes.len());
    
    // Zero-copy recovery using ref_from_bytes  
    match TestFixedVec::<u64, 8>::ref_from_bytes(bytes) {
        Some(recovered) => {
            println!("✅ SUCCESS: Zerocopy const generic derives work!");
            println!("Original: {:?}", original);
            println!("Recovered: {:?}", recovered.as_slice());
            assert_eq!(original, recovered.as_slice());
        }
        None => {
            println!("❌ FAILED: Could not recover from bytes");
        }
    }
}