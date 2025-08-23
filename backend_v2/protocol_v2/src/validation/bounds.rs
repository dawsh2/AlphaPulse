//! Bounds Checking for Safe Memory Operations
//!
//! Prevents buffer overruns and validates data structure sizes

use crate::tlv::ParseError;

/// Check if a buffer has enough bytes for a read operation
pub fn check_buffer_bounds(buffer: &[u8], offset: usize, size: usize) -> Result<(), ParseError> {
    if offset.saturating_add(size) > buffer.len() {
        return Err(ParseError::MessageTooSmall {
            need: offset + size,
            got: buffer.len(),
        });
    }
    Ok(())
}

/// Safely extract a slice from a buffer with bounds checking
pub fn safe_slice(buffer: &[u8], offset: usize, size: usize) -> Result<&[u8], ParseError> {
    check_buffer_bounds(buffer, offset, size)?;
    Ok(&buffer[offset..offset + size])
}

/// Safely extract a mutable slice from a buffer with bounds checking
pub fn safe_slice_mut(
    buffer: &mut [u8],
    offset: usize,
    size: usize,
) -> Result<&mut [u8], ParseError> {
    if offset.saturating_add(size) > buffer.len() {
        return Err(ParseError::MessageTooSmall {
            need: offset + size,
            got: buffer.len(),
        });
    }
    Ok(&mut buffer[offset..offset + size])
}

/// Validate that a TLV payload size matches expectations
pub fn validate_tlv_payload_size(
    actual_size: usize,
    expected_size: Option<usize>,
    _tlv_type: u8,
) -> Result<(), ParseError> {
    if let Some(expected) = expected_size {
        if actual_size != expected {
            return Err(ParseError::PayloadTooLarge { size: actual_size });
        }
    }
    Ok(())
}

/// Check if a size value is within reasonable bounds
pub fn validate_size_bounds(size: usize, max_size: usize) -> Result<(), ParseError> {
    if size > max_size {
        return Err(ParseError::PayloadTooLarge { size });
    }
    Ok(())
}

/// Validate alignment for zero-copy operations
pub fn check_alignment<T>(ptr: *const u8) -> bool {
    let alignment = std::mem::align_of::<T>();
    (ptr as usize) % alignment == 0
}

/// Bounds-checked integer conversion
pub trait SafeConvert<T> {
    fn safe_convert(self) -> Result<T, ParseError>;
}

impl SafeConvert<usize> for u32 {
    fn safe_convert(self) -> Result<usize, ParseError> {
        Ok(self as usize)
    }
}

impl SafeConvert<u32> for usize {
    fn safe_convert(self) -> Result<u32, ParseError> {
        self.try_into()
            .map_err(|_| ParseError::PayloadTooLarge { size: self })
    }
}

impl SafeConvert<u16> for usize {
    fn safe_convert(self) -> Result<u16, ParseError> {
        self.try_into()
            .map_err(|_| ParseError::PayloadTooLarge { size: self })
    }
}

impl SafeConvert<u8> for usize {
    fn safe_convert(self) -> Result<u8, ParseError> {
        self.try_into()
            .map_err(|_| ParseError::PayloadTooLarge { size: self })
    }
}

/// Safe memory copy with bounds checking
pub fn safe_copy(
    src: &[u8],
    dst: &mut [u8],
    src_offset: usize,
    dst_offset: usize,
    len: usize,
) -> Result<(), ParseError> {
    check_buffer_bounds(src, src_offset, len)?;
    check_buffer_bounds(dst, dst_offset, len)?;

    dst[dst_offset..dst_offset + len].copy_from_slice(&src[src_offset..src_offset + len]);
    Ok(())
}

/// Buffer overflow protection for dynamic allocations
pub const MAX_ALLOCATION_SIZE: usize = 100 * 1024 * 1024; // 100MB
pub const MAX_TLV_PAYLOAD_SIZE: usize = 255; // 255 bytes (u8 max)
pub const MAX_EXTENDED_TLV_PAYLOAD_SIZE: usize = 65535; // 65KB extended
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB total message

/// Validate that an allocation size is reasonable
pub fn validate_allocation_size(size: usize) -> Result<(), ParseError> {
    validate_size_bounds(size, MAX_ALLOCATION_SIZE)
}

/// Validate TLV payload size based on type
pub fn validate_tlv_size(size: usize, is_extended: bool) -> Result<(), ParseError> {
    let max_size = if is_extended {
        MAX_EXTENDED_TLV_PAYLOAD_SIZE
    } else {
        MAX_TLV_PAYLOAD_SIZE
    };
    validate_size_bounds(size, max_size)
}

/// Validate total message size
pub fn validate_message_size(size: usize) -> Result<(), ParseError> {
    validate_size_bounds(size, MAX_MESSAGE_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_bounds_checking() {
        let buffer = vec![0u8; 10];

        // Valid access
        assert!(check_buffer_bounds(&buffer, 0, 10).is_ok());
        assert!(check_buffer_bounds(&buffer, 5, 5).is_ok());
        assert!(check_buffer_bounds(&buffer, 9, 1).is_ok());

        // Invalid access
        assert!(check_buffer_bounds(&buffer, 0, 11).is_err());
        assert!(check_buffer_bounds(&buffer, 10, 1).is_err());
        assert!(check_buffer_bounds(&buffer, 5, 6).is_err());
    }

    #[test]
    fn test_safe_slice() {
        let buffer = vec![1, 2, 3, 4, 5];

        // Valid slice
        let slice = safe_slice(&buffer, 1, 3).unwrap();
        assert_eq!(slice, &[2, 3, 4]);

        // Invalid slice
        assert!(safe_slice(&buffer, 3, 3).is_err());
    }

    #[test]
    fn test_safe_slice_mut() {
        let mut buffer = vec![1, 2, 3, 4, 5];

        // Valid mutable slice
        {
            let slice = safe_slice_mut(&mut buffer, 1, 3).unwrap();
            slice[0] = 10;
        }
        assert_eq!(buffer[1], 10);

        // Invalid slice
        assert!(safe_slice_mut(&mut buffer, 3, 3).is_err());
    }

    #[test]
    fn test_size_validation() {
        assert!(validate_size_bounds(100, 1000).is_ok());
        assert!(validate_size_bounds(1000, 1000).is_ok());
        assert!(validate_size_bounds(1001, 1000).is_err());

        assert!(validate_allocation_size(1000).is_ok());
        assert!(validate_allocation_size(MAX_ALLOCATION_SIZE).is_ok());
        assert!(validate_allocation_size(MAX_ALLOCATION_SIZE + 1).is_err());
    }

    #[test]
    fn test_tlv_size_validation() {
        // Standard TLV
        assert!(validate_tlv_size(255, false).is_ok());
        assert!(validate_tlv_size(256, false).is_err());

        // Extended TLV
        assert!(validate_tlv_size(65535, true).is_ok());
        assert!(validate_tlv_size(65536, true).is_err());
    }

    #[test]
    fn test_safe_convert() {
        let large_usize = 1000usize;
        let small_usize = 200usize;

        // Safe conversions
        let _as_u32: Result<u32, _> = large_usize.safe_convert();
        assert!(_as_u32.is_ok());
        let _as_u8: Result<u8, _> = small_usize.safe_convert();
        assert!(_as_u8.is_ok());

        // Unsafe conversion
        let too_large = usize::MAX;
        let _too_large_as_u32: Result<u32, _> = too_large.safe_convert();
        assert!(_too_large_as_u32.is_err());
    }

    #[test]
    fn test_safe_copy() {
        let src = vec![1, 2, 3, 4, 5];
        let mut dst = vec![0; 10];

        // Valid copy
        assert!(safe_copy(&src, &mut dst, 1, 2, 3).is_ok());
        assert_eq!(dst[2..5], [2, 3, 4]);

        // Invalid copy (source bounds)
        assert!(safe_copy(&src, &mut dst, 3, 0, 3).is_err());

        // Invalid copy (destination bounds)
        assert!(safe_copy(&src, &mut dst, 0, 8, 3).is_err());
    }

    #[test]
    fn test_alignment_checking() {
        let buffer = vec![0u8; 16];
        let ptr = buffer.as_ptr();

        // Test alignment for different types
        assert!(check_alignment::<u8>(ptr)); // u8 has alignment 1
                                             // Note: We can't guarantee alignment for u32/u64 from Vec<u8> without special allocation

        // Test with a properly aligned allocation
        let aligned_buffer: Vec<u64> = vec![0; 2];
        let aligned_ptr = aligned_buffer.as_ptr() as *const u8;
        assert!(check_alignment::<u64>(aligned_ptr));
    }
}
