//! Packed balanced-ternary logical memory for TRION.
//!
//! Five balanced trits are stored in one conventional byte. This is a
//! software encoding in binary-backed memory, not physical ternary RAM.

#![forbid(unsafe_code)]

use core::fmt;

const TRITS_PER_BYTE: usize = 5;
const POW3: [u16; TRITS_PER_BYTE] = [1, 3, 9, 27, 81];
const MAX_PACKED_BYTE: u16 = 242;
const ZERO_PACKED_BYTE: u8 = 121;

/// Errors returned by checked packed-memory operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackedTritError {
    InvalidTrit { index: usize, value: i8 },
    IndexOutOfBounds { index: usize, len: usize },
    LengthOverflow,
    AllocationFailure,
    WrongByteLength { expected: usize, actual: usize },
    InvalidPackedByte { index: usize, value: u8 },
    NonCanonicalPadding { index: usize },
}

impl fmt::Display for PackedTritError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTrit { index, value } => {
                write!(f, "invalid balanced trit {value} at index {index}")
            }
            Self::IndexOutOfBounds { index, len } => {
                write!(f, "trit index {index} is outside buffer length {len}")
            }
            Self::LengthOverflow => f.write_str("packed trit buffer length overflow"),
            Self::AllocationFailure => f.write_str("unable to allocate packed trit buffer"),
            Self::WrongByteLength { expected, actual } => {
                write!(f, "packed byte length {actual}; expected {expected}")
            }
            Self::InvalidPackedByte { index, value } => {
                write!(
                    f,
                    "invalid packed byte {value} at byte index {index}; maximum is 242"
                )
            }
            Self::NonCanonicalPadding { index } => {
                write!(
                    f,
                    "noncanonical padding in final packed byte at index {index}"
                )
            }
        }
    }
}

impl std::error::Error for PackedTritError {}

/// A logical sequence of balanced trits packed into ordinary bytes.
///
/// Trits are indexed from least-significant to most-significant within each
/// byte. Five trits occupy one byte; the logical length is stored separately.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackedTritBuffer {
    len: usize,
    bytes: Vec<u8>,
}

impl PackedTritBuffer {
    /// Allocate a buffer containing `len` logical zero trits.
    pub fn new_zeroed(len: usize) -> Result<Self, PackedTritError> {
        let byte_len = packed_byte_len(len)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(byte_len)
            .map_err(|_| PackedTritError::AllocationFailure)?;
        bytes.resize(byte_len, ZERO_PACKED_BYTE);
        Ok(Self { len, bytes })
    }

    /// Pack a slice of balanced trits. Each input must be -1, 0, or +1.
    pub fn from_balanced_trits(values: &[i8]) -> Result<Self, PackedTritError> {
        let mut result = Self::new_zeroed(values.len())?;
        for (index, &value) in values.iter().enumerate() {
            result.set(index, value)?;
        }
        Ok(result)
    }

    /// Validate and copy packed bytes with an explicit logical trit count.
    ///
    /// Bytes must be in 0..=242. Unused trits in the final byte must encode
    /// logical zero, ensuring one canonical byte representation per value.
    pub fn from_packed_bytes(len: usize, bytes: &[u8]) -> Result<Self, PackedTritError> {
        let expected = packed_byte_len(len)?;
        if bytes.len() != expected {
            return Err(PackedTritError::WrongByteLength {
                expected,
                actual: bytes.len(),
            });
        }

        for (index, &byte) in bytes.iter().enumerate() {
            if u16::from(byte) > MAX_PACKED_BYTE {
                return Err(PackedTritError::InvalidPackedByte { index, value: byte });
            }
        }

        let remainder = len % TRITS_PER_BYTE;
        if remainder != 0 {
            let last_index = bytes.len() - 1;
            let last = u16::from(bytes[last_index]);
            for power in POW3.iter().skip(remainder) {
                let digit = (last / *power) % 3;
                if digit != 1 {
                    return Err(PackedTritError::NonCanonicalPadding { index: last_index });
                }
            }
        }

        let mut owned = Vec::new();
        owned
            .try_reserve_exact(expected)
            .map_err(|_| PackedTritError::AllocationFailure)?;
        owned.extend_from_slice(bytes);
        Ok(Self { len, bytes: owned })
    }

    /// Number of logical balanced trits.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the logical buffer is empty.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Number of backing bytes. This is not a physical ternary-memory size.
    pub const fn packed_byte_len(&self) -> usize {
        self.bytes.len()
    }

    /// Borrow the packed binary backing bytes.
    pub fn as_packed_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Read one balanced trit. Returns None when the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<i8> {
        if index >= self.len {
            return None;
        }
        let byte_index = index / TRITS_PER_BYTE;
        let position = index % TRITS_PER_BYTE;
        let packed = u16::from(*self.bytes.get(byte_index)?);
        let unbalanced = (packed / POW3[position]) % 3;
        Some(unbalanced as i8 - 1)
    }

    /// Write one balanced trit, rejecting invalid values and out-of-range indices.
    pub fn set(&mut self, index: usize, value: i8) -> Result<(), PackedTritError> {
        if !(-1..=1).contains(&value) {
            return Err(PackedTritError::InvalidTrit { index, value });
        }
        if index >= self.len {
            return Err(PackedTritError::IndexOutOfBounds {
                index,
                len: self.len,
            });
        }

        let byte_index = index / TRITS_PER_BYTE;
        let position = index % TRITS_PER_BYTE;
        let power = POW3[position];
        let old_byte = u16::from(self.bytes[byte_index]);
        let old_digit = (old_byte / power) % 3;
        let new_digit = (value + 1) as u16;
        let updated = old_byte as i32 + (new_digit as i32 - old_digit as i32) * power as i32;

        if !(0..=i32::from(MAX_PACKED_BYTE)).contains(&updated) {
            return Err(PackedTritError::InvalidPackedByte {
                index: byte_index,
                value: self.bytes[byte_index],
            });
        }
        self.bytes[byte_index] = updated as u8;
        Ok(())
    }

    /// Expand the logical sequence into a newly allocated vector.
    pub fn to_balanced_trits(&self) -> Result<Vec<i8>, PackedTritError> {
        let mut values = Vec::new();
        values
            .try_reserve_exact(self.len)
            .map_err(|_| PackedTritError::AllocationFailure)?;
        for index in 0..self.len {
            // The range is exactly len, so get must return Some.
            let value = self.get(index).ok_or(PackedTritError::IndexOutOfBounds {
                index,
                len: self.len,
            })?;
            values.push(value);
        }
        Ok(values)
    }
}

fn packed_byte_len(trit_len: usize) -> Result<usize, PackedTritError> {
    trit_len
        .checked_add(TRITS_PER_BYTE - 1)
        .map(|rounded| rounded / TRITS_PER_BYTE)
        .ok_or(PackedTritError::LengthOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeroed_buffer_has_canonical_zero_encoding() {
        let buffer = PackedTritBuffer::new_zeroed(7).unwrap();
        assert_eq!(buffer.to_balanced_trits().unwrap(), vec![0; 7]);
        assert_eq!(buffer.as_packed_bytes(), &[121, 121]);
    }

    #[test]
    fn round_trips_all_balanced_trit_values_and_partial_bytes() {
        let values = [-1, 0, 1, 1, -1, 0, 1];
        let buffer = PackedTritBuffer::from_balanced_trits(&values).unwrap();
        assert_eq!(buffer.to_balanced_trits().unwrap(), values);
        let decoded =
            PackedTritBuffer::from_packed_bytes(values.len(), buffer.as_packed_bytes()).unwrap();
        assert_eq!(decoded, buffer);
    }

    #[test]
    fn every_valid_packed_byte_decodes_and_reencodes() {
        for packed in 0u8..=242 {
            let buffer = PackedTritBuffer::from_packed_bytes(5, &[packed]).unwrap();
            let expanded = buffer.to_balanced_trits().unwrap();
            let encoded = PackedTritBuffer::from_balanced_trits(&expanded).unwrap();
            assert_eq!(encoded.as_packed_bytes(), &[packed]);
        }
    }

    #[test]
    fn rejects_reserved_byte_values() {
        for packed in 243u8..=255 {
            assert!(matches!(
                PackedTritBuffer::from_packed_bytes(5, &[packed]),
                Err(PackedTritError::InvalidPackedByte { .. })
            ));
        }
    }

    #[test]
    fn rejects_noncanonical_padding() {
        assert!(matches!(
            PackedTritBuffer::from_packed_bytes(1, &[0]),
            Err(PackedTritError::NonCanonicalPadding { .. })
        ));
        assert!(PackedTritBuffer::from_packed_bytes(1, &[121]).is_ok());
    }

    #[test]
    fn rejects_wrong_byte_count() {
        assert!(matches!(
            PackedTritBuffer::from_packed_bytes(6, &[121]),
            Err(PackedTritError::WrongByteLength { .. })
        ));
    }

    #[test]
    fn set_and_get_cover_each_trit_position() {
        let mut buffer = PackedTritBuffer::new_zeroed(5).unwrap();
        for (index, value) in [-1, 0, 1, -1, 1].into_iter().enumerate() {
            buffer.set(index, value).unwrap();
            assert_eq!(buffer.get(index), Some(value));
        }
    }

    #[test]
    fn rejects_invalid_trits_and_out_of_bounds_access() {
        let mut buffer = PackedTritBuffer::new_zeroed(2).unwrap();
        assert!(matches!(
            buffer.set(0, 2),
            Err(PackedTritError::InvalidTrit { .. })
        ));
        assert!(matches!(
            buffer.set(2, 0),
            Err(PackedTritError::IndexOutOfBounds { .. })
        ));
        assert_eq!(buffer.get(2), None);
    }

    #[test]
    fn empty_buffer_is_valid() {
        let buffer = PackedTritBuffer::new_zeroed(0).unwrap();
        assert!(buffer.is_empty());
        assert!(buffer.as_packed_bytes().is_empty());
        assert_eq!(PackedTritBuffer::from_packed_bytes(0, &[]).unwrap(), buffer);
    }
}
