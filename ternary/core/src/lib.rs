//! Balanced-ternary arithmetic for Trion.
//!
//! Trits are -1, 0, +1. Text uses T for -1, 0 for zero, and 1 for +1.
//! This is a software model that runs on conventional binary hardware.

use core::fmt;

/// A canonical balanced-ternary integer, stored least-significant trit first.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ternary {
    trits: Vec<i8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TernaryError {
    EmptyInput,
    InvalidDigit(char),
    InvalidTrit { index: usize, value: i8 },
    DecimalOverflow,
    NegativeUnsigned,
    SizeOverflow,
    AllocationFailure,
    InvalidSerialization(&'static str),
}

impl fmt::Display for TernaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "ternary input cannot be empty"),
            Self::InvalidDigit(c) => {
                write!(f, "invalid balanced-ternary digit '{c}' (use T, 0, 1)")
            }
            Self::DecimalOverflow => write!(f, "value is outside the target integer range"),
            Self::InvalidTrit { index, value } => write!(
                f,
                "balanced trit at index {index} is {value}; expected -1, 0, or 1"
            ),
            Self::NegativeUnsigned => {
                write!(f, "negative balanced-ternary value cannot convert to u128")
            }
            Self::SizeOverflow => write!(f, "ternary value size overflow"),
            Self::AllocationFailure => write!(f, "unable to allocate ternary value storage"),
            Self::InvalidSerialization(reason) => write!(f, "invalid TRN3 serialization: {reason}"),
        }
    }
}
impl std::error::Error for TernaryError {}

impl Ternary {
    /// Return zero, represented canonically by one zero trit.
    pub fn zero() -> Self {
        Self { trits: vec![0] }
    }

    /// Convert a signed binary integer to its unique balanced-ternary form.
    ///
    /// The digit vector is least-significant trit first. This conversion is
    /// exact for every `i128`, including `i128::MIN`.
    pub fn from_i128(mut value: i128) -> Self {
        if value == 0 {
            return Self::zero();
        }
        let mut trits = Vec::new();
        while value != 0 {
            let rem = value.rem_euclid(3);
            value = value.div_euclid(3);
            let digit = if rem == 2 {
                value += 1;
                -1
            } else {
                rem as i8
            };
            trits.push(digit);
        }
        Self { trits }
    }

    /// Convert an unsigned binary integer to its unique balanced-ternary form.
    pub fn from_u128(mut value: u128) -> Self {
        if value == 0 {
            return Self::zero();
        }
        let mut trits = Vec::new();
        while value != 0 {
            let rem = value % 3;
            value /= 3;
            let digit = if rem == 2 {
                value += 1;
                -1
            } else {
                rem as i8
            };
            trits.push(digit);
        }
        Self { trits }
    }

    /// Construct a value from least-significant-first balanced trits.
    ///
    /// Leading zero trits are removed. An empty slice denotes zero.
    pub fn from_balanced_trits_lsf(trits: &[i8]) -> Result<Self, TernaryError> {
        if let Some((index, value)) = trits
            .iter()
            .enumerate()
            .find_map(|(index, &value)| (!(-1..=1).contains(&value)).then_some((index, value)))
        {
            return Err(TernaryError::InvalidTrit { index, value });
        }
        let mut digits = Vec::new();
        digits
            .try_reserve_exact(trits.len().max(1))
            .map_err(|_| TernaryError::AllocationFailure)?;
        digits.extend_from_slice(trits);
        let mut value = Self { trits: digits };
        value.normalize();
        Ok(value)
    }

    /// Parse most-significant-first text. T/t represents -1.
    pub fn parse(input: &str) -> Result<Self, TernaryError> {
        if input.is_empty() {
            return Err(TernaryError::EmptyInput);
        }
        let mut trits = Vec::new();
        trits
            .try_reserve_exact(input.len())
            .map_err(|_| TernaryError::AllocationFailure)?;
        for ch in input.chars().rev() {
            trits.push(match ch {
                'T' | 't' => -1,
                '0' => 0,
                '1' => 1,
                other => return Err(TernaryError::InvalidDigit(other)),
            });
        }
        let mut value = Self { trits };
        value.normalize();
        Ok(value)
    }

    pub fn trits_lsf(&self) -> &[i8] {
        &self.trits
    }

    pub fn to_i128(&self) -> Result<i128, TernaryError> {
        // Convert the magnitude in u128 so i128::MIN (whose magnitude is
        // one larger than i128::MAX) remains representable during conversion.
        let negative = self.trits.last().copied().unwrap_or(0) < 0;
        let mut magnitude = 0u128;
        for &trit in self.trits.iter().rev() {
            let digit = if negative { -trit } else { trit };
            magnitude = magnitude
                .checked_mul(3)
                .ok_or(TernaryError::DecimalOverflow)?;
            let addend = digit as i128;
            if addend < 0 {
                magnitude = magnitude
                    .checked_sub((-addend) as u128)
                    .ok_or(TernaryError::DecimalOverflow)?;
            } else {
                magnitude = magnitude
                    .checked_add(addend as u128)
                    .ok_or(TernaryError::DecimalOverflow)?;
            }
        }
        if negative {
            let min_magnitude = (i128::MAX as u128) + 1;
            if magnitude == min_magnitude {
                Ok(i128::MIN)
            } else if magnitude <= i128::MAX as u128 {
                Ok(-(magnitude as i128))
            } else {
                Err(TernaryError::DecimalOverflow)
            }
        } else if magnitude <= i128::MAX as u128 {
            Ok(magnitude as i128)
        } else {
            Err(TernaryError::DecimalOverflow)
        }
    }

    /// Convert to an unsigned binary integer, rejecting negative values and overflow.
    pub fn to_u128(&self) -> Result<u128, TernaryError> {
        let negative = self.trits.last().copied().unwrap_or(0) < 0;
        if negative {
            return Err(TernaryError::NegativeUnsigned);
        }
        let mut value = 0u128;
        for &trit in self.trits.iter().rev() {
            value = value.checked_mul(3).ok_or(TernaryError::DecimalOverflow)?;
            if trit < 0 {
                value = value
                    .checked_sub((-trit) as u128)
                    .ok_or(TernaryError::DecimalOverflow)?;
            } else {
                value = value
                    .checked_add(trit as u128)
                    .ok_or(TernaryError::DecimalOverflow)?;
            }
        }
        Ok(value)
    }

    /// Add two arbitrary-precision balanced integers.
    ///
    /// The operation has no numeric overflow; storage-size and allocation
    /// failures are returned explicitly.
    pub fn try_add(&self, rhs: &Self) -> Result<Self, TernaryError> {
        let n = self.trits.len().max(rhs.trits.len());
        let capacity = n.checked_add(1).ok_or(TernaryError::SizeOverflow)?;
        let mut out = Vec::new();
        out.try_reserve_exact(capacity)
            .map_err(|_| TernaryError::AllocationFailure)?;
        let mut carry = 0i8;
        for i in 0..n {
            let total = self.trits.get(i).copied().unwrap_or(0)
                + rhs.trits.get(i).copied().unwrap_or(0)
                + carry;
            let (digit, next) = normalize_small(total);
            out.push(digit);
            carry = next;
        }
        if carry != 0 {
            out.push(carry);
        }
        let mut result = Self { trits: out };
        result.normalize();
        Ok(result)
    }

    pub fn add(&self, rhs: &Self) -> Self {
        self.try_add(rhs)
            .expect("ternary addition allocation failed")
    }

    /// Subtract two arbitrary-precision balanced integers.
    pub fn try_sub(&self, rhs: &Self) -> Result<Self, TernaryError> {
        self.try_add(&rhs.try_negated()?)
    }

    pub fn sub(&self, rhs: &Self) -> Self {
        self.try_sub(rhs)
            .expect("ternary subtraction allocation failed")
    }

    /// Negate an arbitrary-precision balanced integer.
    pub fn try_negated(&self) -> Result<Self, TernaryError> {
        let mut trits = Vec::new();
        trits
            .try_reserve_exact(self.trits.len())
            .map_err(|_| TernaryError::AllocationFailure)?;
        trits.extend(self.trits.iter().map(|t| -*t));
        let mut result = Self { trits };
        result.normalize();
        Ok(result)
    }

    pub fn negated(&self) -> Self {
        self.try_negated()
            .expect("ternary negation allocation failed")
    }

    /// Multiply via trit convolution and balanced carry normalization.
    pub fn mul(&self, rhs: &Self) -> Self {
        self.try_mul(rhs).expect("ternary multiplication failed")
    }

    /// Multiply two arbitrary-precision balanced integers with checked storage arithmetic.
    pub fn try_mul(&self, rhs: &Self) -> Result<Self, TernaryError> {
        if self.is_zero() || rhs.is_zero() {
            return Ok(Self::zero());
        }
        let capacity = self
            .trits
            .len()
            .checked_add(rhs.trits.len())
            .and_then(|length| length.checked_add(1))
            .ok_or(TernaryError::SizeOverflow)?;
        let mut accum = Vec::new();
        accum
            .try_reserve_exact(capacity)
            .map_err(|_| TernaryError::AllocationFailure)?;
        accum.resize(capacity, 0i128);
        for (i, &a) in self.trits.iter().enumerate() {
            for (j, &b) in rhs.trits.iter().enumerate() {
                let index = i.checked_add(j).ok_or(TernaryError::SizeOverflow)?;
                let cell = accum.get_mut(index).ok_or(TernaryError::SizeOverflow)?;
                *cell = cell
                    .checked_add((a as i128) * (b as i128))
                    .ok_or(TernaryError::SizeOverflow)?;
            }
        }
        let mut out = Vec::new();
        out.try_reserve_exact(capacity)
            .map_err(|_| TernaryError::AllocationFailure)?;
        let mut carry = 0i128;
        for coefficient in accum {
            let total = coefficient
                .checked_add(carry)
                .ok_or(TernaryError::SizeOverflow)?;
            let rem = total.rem_euclid(3);
            let digit = match rem {
                0 => 0,
                1 => 1,
                _ => -1,
            };
            carry = total.checked_sub(digit).ok_or(TernaryError::SizeOverflow)? / 3;
            out.push(digit as i8);
        }
        while carry != 0 {
            out.try_reserve(1)
                .map_err(|_| TernaryError::AllocationFailure)?;
            let rem = carry.rem_euclid(3);
            let digit = match rem {
                0 => 0,
                1 => 1,
                _ => -1,
            };
            carry = carry.checked_sub(digit).ok_or(TernaryError::SizeOverflow)? / 3;
            out.push(digit as i8);
        }
        let mut result = Self { trits: out };
        result.normalize();
        Ok(result)
    }

    pub fn is_zero(&self) -> bool {
        self.trits.iter().all(|&t| t == 0)
    }

    fn normalize(&mut self) {
        while self.trits.len() > 1 && self.trits.last() == Some(&0) {
            self.trits.pop();
        }
        if self.trits.is_empty() {
            self.trits.push(0);
        }
    }
}

fn normalize_small(total: i8) -> (i8, i8) {
    match total {
        -3..=-2 => (total + 3, -1),
        -1..=1 => (total, 0),
        2..=3 => (total - 3, 1),
        _ => unreachable!("sum of two trits and carry is between -3 and 3"),
    }
}

impl fmt::Display for Ternary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &trit in self.trits.iter().rev() {
            f.write_str(match trit {
                -1 => "T",
                0 => "0",
                1 => "1",
                _ => unreachable!(),
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_values_round_trip() {
        let cases = [
            (0, "0"),
            (1, "1"),
            (2, "1T"),
            (3, "10"),
            (4, "11"),
            (5, "1TT"),
            (8, "10T"),
            (-1, "T"),
            (-2, "T1"),
            (-3, "T0"),
            (-4, "TT"),
        ];
        for (decimal, ternary) in cases {
            assert_eq!(Ternary::from_i128(decimal).to_string(), ternary);
            assert_eq!(Ternary::parse(ternary).unwrap().to_i128().unwrap(), decimal);
        }
    }

    #[test]
    fn i128_edges_round_trip() {
        for n in [
            i128::MIN,
            i128::MIN + 1,
            -1000,
            -2,
            -1,
            0,
            1,
            2,
            1000,
            i128::MAX,
        ] {
            assert_eq!(Ternary::from_i128(n).to_i128().unwrap(), n);
        }
    }

    #[test]
    fn u128_bridge_round_trips_and_rejects_negative_values() {
        for n in [0, 1, 2, 3, 42, u64::MAX as u128, u128::MAX] {
            assert_eq!(Ternary::from_u128(n).to_u128().unwrap(), n);
        }
        assert_eq!(
            Ternary::from_i128(-1).to_u128(),
            Err(TernaryError::NegativeUnsigned)
        );
        assert_eq!(
            Ternary::from_balanced_trits_lsf(&[0, 2]),
            Err(TernaryError::InvalidTrit { index: 1, value: 2 })
        );
        assert_eq!(
            Ternary::from_balanced_trits_lsf(&[1, 0, 0])
                .unwrap()
                .to_string(),
            "1"
        );
        assert_eq!(
            Ternary::from_balanced_trits_lsf(&[]).unwrap(),
            Ternary::zero()
        );
        assert_eq!(
            Ternary::from_i128(i128::MAX)
                .add(&Ternary::from_i128(1))
                .to_i128(),
            Err(TernaryError::DecimalOverflow)
        );
        assert_eq!(
            Ternary::from_u128(u128::MAX)
                .add(&Ternary::from_i128(1))
                .to_u128(),
            Err(TernaryError::DecimalOverflow)
        );
    }

    #[test]
    fn arithmetic_matches_decimal_for_small_range() {
        for a in -100i128..=100 {
            for b in -100i128..=100 {
                let ta = Ternary::from_i128(a);
                let tb = Ternary::from_i128(b);
                assert_eq!(ta.add(&tb).to_i128().unwrap(), a + b);
                assert_eq!(ta.sub(&tb).to_i128().unwrap(), a - b);
                assert_eq!(ta.mul(&tb).to_i128().unwrap(), a * b);
            }
        }
    }

    #[test]
    fn canonicalization_and_errors() {
        assert_eq!(Ternary::parse("0001").unwrap().to_string(), "1");
        assert_eq!(Ternary::parse("t01").unwrap().to_string(), "T01");
        assert!(matches!(Ternary::parse(""), Err(TernaryError::EmptyInput)));
        assert!(matches!(
            Ternary::parse("12"),
            Err(TernaryError::InvalidDigit('2'))
        ));
    }

    #[test]
    fn arithmetic_can_exceed_i128() {
        let huge = Ternary::parse("1111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111").unwrap();
        let result = huge.add(&huge);
        assert!(result.to_i128().is_err());
        assert_eq!(result, huge.mul(&Ternary::from_i128(2)));
    }

    #[test]
    fn checked_arithmetic_matches_unbounded_operations() {
        for a in -50i128..=50 {
            for b in -50i128..=50 {
                let ta = Ternary::from_i128(a);
                let tb = Ternary::from_i128(b);
                assert_eq!(ta.try_add(&tb).unwrap(), ta.add(&tb));
                assert_eq!(ta.try_sub(&tb).unwrap(), ta.sub(&tb));
                assert_eq!(ta.try_mul(&tb).unwrap(), ta.mul(&tb));
                assert_eq!(ta.try_negated().unwrap(), ta.negated());
            }
        }
    }

    #[test]
    fn trn3_serialization_is_stable_and_rejects_malformed_values() {
        let value = Ternary::from_i128(-42);
        let encoded = serialize_balanced(&value).unwrap();
        assert_eq!(&encoded[..10], b"TRN3\x01\x01\x05\x00\x00\x00");
        assert_eq!(deserialize_balanced(&encoded).unwrap(), value);
        assert_eq!(
            deserialize_balanced(&serialize_balanced(&Ternary::zero()).unwrap()).unwrap(),
            Ternary::zero()
        );

        assert!(matches!(
            deserialize_balanced(b"TRN"),
            Err(TernaryError::InvalidSerialization(_))
        ));
        assert!(matches!(
            deserialize_balanced(b"NOPE\x01\x01\x01\x00\x00\x00\x01"),
            Err(TernaryError::InvalidSerialization("bad magic"))
        ));
        let mut bad = encoded.clone();
        bad[4] = 2;
        assert!(matches!(
            deserialize_balanced(&bad),
            Err(TernaryError::InvalidSerialization("unsupported version"))
        ));
        let mut bad = encoded.clone();
        bad[5] = 2;
        assert!(matches!(
            deserialize_balanced(&bad),
            Err(TernaryError::InvalidSerialization(
                "unsupported trit domain"
            ))
        ));
        let mut bad = encoded.clone();
        bad[6..10].copy_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            deserialize_balanced(&bad),
            Err(TernaryError::InvalidSerialization(
                "digit count must not be zero"
            ))
        ));
        let mut bad = encoded.clone();
        bad.push(1);
        assert!(matches!(
            deserialize_balanced(&bad),
            Err(TernaryError::InvalidSerialization(
                "payload length does not match digit count"
            ))
        ));
        let mut bad = encoded.clone();
        *bad.last_mut().unwrap() = 3;
        assert!(matches!(
            deserialize_balanced(&bad),
            Err(TernaryError::InvalidSerialization("digit is outside 0..=2"))
        ));
        let mut bad = encoded.clone();
        *bad.last_mut().unwrap() = 1;
        assert!(matches!(
            deserialize_balanced(&bad),
            Err(TernaryError::InvalidSerialization(
                "leading zero trit is noncanonical"
            ))
        ));
    }
}

// ---------------------------------------------------------------------------
// Checked packed unbalanced-ternary representation (Phase 1).
//
// This is a software representation stored in ordinary binary integers. Trit
// zero is least-significant. It is intentionally separate from `Ternary`,
// whose digits are balanced (-1, 0, +1).
// ---------------------------------------------------------------------------

/// Maximum number of unbalanced trits that fit in a binary integer.
pub const U3_U32_CAPACITY: usize = 20;
/// Maximum number of unbalanced trits that fit in a 64-bit integer.
pub const U3_U64_CAPACITY: usize = 40;
/// Maximum number of unbalanced trits that fit in a byte.
pub const U3_U8_CAPACITY: usize = 5;
/// Maximum number of unbalanced trits that fit in a 16-bit integer.
pub const U3_U16_CAPACITY: usize = 10;

const SERIAL_MAGIC: &[u8; 4] = b"TRN3";
const SERIAL_VERSION: u8 = 1;
const SERIAL_BALANCED_DOMAIN: u8 = 1;
const SERIAL_HEADER_LEN: usize = 10;

/// Errors returned by checked unbalanced-ternary packing operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackedTritError {
    InvalidTrit { index: usize, value: u8 },
    InvalidBalancedTrit { value: i8 },
    TooManyTrits { count: usize, capacity: usize },
    InvalidCount { count: usize, capacity: usize },
    InvalidPackedValue { value: u64, count: usize },
    Overflow { bits: u8 },
    AllocationFailure,
}

impl fmt::Display for PackedTritError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTrit { index, value } => write!(
                f,
                "unbalanced trit at index {index} is {value}; expected 0, 1, or 2"
            ),
            Self::InvalidBalancedTrit { value } => {
                write!(f, "balanced trit is {value}; expected -1, 0, or 1")
            }
            Self::TooManyTrits { count, capacity } => {
                write!(f, "{count} trits exceed target capacity {capacity}")
            }
            Self::InvalidCount { count, capacity } => {
                write!(f, "trit count {count} exceeds capacity {capacity}")
            }
            Self::InvalidPackedValue { value, count } => {
                write!(f, "packed value {value} is invalid for {count} trits")
            }
            Self::Overflow { bits } => write!(f, "packed trit value overflows u{bits}"),
            Self::AllocationFailure => write!(f, "unable to allocate decoded trit storage"),
        }
    }
}
impl std::error::Error for PackedTritError {}

/// Pack unbalanced trits into the smallest supported binary integer widths.
pub fn pack_u3_u8(trits: &[u8]) -> Result<u8, PackedTritError> {
    if trits.len() > U3_U8_CAPACITY {
        return Err(PackedTritError::TooManyTrits {
            count: trits.len(),
            capacity: U3_U8_CAPACITY,
        });
    }
    let value = pack_u3_u32(trits)?;
    u8::try_from(value).map_err(|_| PackedTritError::Overflow { bits: 8 })
}

/// Decode exactly `count` least-significant-first unbalanced trits from a byte.
pub fn unpack_u3_u8(value: u8, count: usize) -> Result<Vec<u8>, PackedTritError> {
    if count > U3_U8_CAPACITY {
        return Err(PackedTritError::InvalidCount {
            count,
            capacity: U3_U8_CAPACITY,
        });
    }
    unpack_u3_u32(value as u32, count)
}

/// Pack unbalanced trits into a 16-bit integer.
pub fn pack_u3_u16(trits: &[u8]) -> Result<u16, PackedTritError> {
    if trits.len() > U3_U16_CAPACITY {
        return Err(PackedTritError::TooManyTrits {
            count: trits.len(),
            capacity: U3_U16_CAPACITY,
        });
    }
    let value = pack_u3_u32(trits)?;
    u16::try_from(value).map_err(|_| PackedTritError::Overflow { bits: 16 })
}

/// Decode exactly `count` least-significant-first unbalanced trits from a 16-bit integer.
pub fn unpack_u3_u16(value: u16, count: usize) -> Result<Vec<u8>, PackedTritError> {
    if count > U3_U16_CAPACITY {
        return Err(PackedTritError::InvalidCount {
            count,
            capacity: U3_U16_CAPACITY,
        });
    }
    unpack_u3_u32(value as u32, count)
}

/// Compare packed U3 integers after validating each value for the declared count.
pub fn compare_u3_u32(
    left: u32,
    right: u32,
    count: usize,
) -> Result<core::cmp::Ordering, PackedTritError> {
    let limit = u3_limit(count, U3_U32_CAPACITY)?;
    if (left as u128) >= limit {
        return Err(PackedTritError::InvalidPackedValue {
            value: left as u64,
            count,
        });
    }
    if (right as u128) >= limit {
        return Err(PackedTritError::InvalidPackedValue {
            value: right as u64,
            count,
        });
    }
    Ok(left.cmp(&right))
}

/// Compare packed U3 integers after validating each value for the declared count.
pub fn compare_u3_u64(
    left: u64,
    right: u64,
    count: usize,
) -> Result<core::cmp::Ordering, PackedTritError> {
    let limit = u3_limit(count, U3_U64_CAPACITY)?;
    if (left as u128) >= limit {
        return Err(PackedTritError::InvalidPackedValue { value: left, count });
    }
    if (right as u128) >= limit {
        return Err(PackedTritError::InvalidPackedValue {
            value: right,
            count,
        });
    }
    Ok(left.cmp(&right))
}

fn u3_limit(count: usize, capacity: usize) -> Result<u128, PackedTritError> {
    if count > capacity {
        return Err(PackedTritError::InvalidCount { count, capacity });
    }
    let mut limit = 1u128;
    for _ in 0..count {
        limit *= 3;
    }
    Ok(limit)
}

/// Pack unbalanced trits (each 0, 1, or 2) into a u32, least-significant first.
pub fn pack_u3_u32(trits: &[u8]) -> Result<u32, PackedTritError> {
    if trits.len() > U3_U32_CAPACITY {
        return Err(PackedTritError::TooManyTrits {
            count: trits.len(),
            capacity: U3_U32_CAPACITY,
        });
    }
    let mut value = 0u64;
    let mut place = 1u64;
    for (index, &trit) in trits.iter().enumerate() {
        if trit > 2 {
            return Err(PackedTritError::InvalidTrit { index, value: trit });
        }
        value = value
            .checked_add(
                (trit as u64)
                    .checked_mul(place)
                    .ok_or(PackedTritError::Overflow { bits: 32 })?,
            )
            .ok_or(PackedTritError::Overflow { bits: 32 })?;
        place = place
            .checked_mul(3)
            .ok_or(PackedTritError::Overflow { bits: 32 })?;
    }
    Ok(value as u32)
}

/// Decode a u32 into exactly `count` unbalanced trits, least-significant first.
/// Rejects non-canonical values rather than silently discarding high trits.
pub fn unpack_u3_u32(value: u32, count: usize) -> Result<Vec<u8>, PackedTritError> {
    let limit = u3_limit(count, U3_U32_CAPACITY)?;
    if (value as u128) >= limit {
        return Err(PackedTritError::InvalidPackedValue {
            value: value as u64,
            count,
        });
    }
    let mut remaining = value;
    let mut trits = Vec::new();
    trits
        .try_reserve_exact(count)
        .map_err(|_| PackedTritError::AllocationFailure)?;
    for _ in 0..count {
        trits.push((remaining % 3) as u8);
        remaining /= 3;
    }
    Ok(trits)
}

/// Pack unbalanced trits (each 0, 1, or 2) into a u64, least-significant first.
pub fn pack_u3_u64(trits: &[u8]) -> Result<u64, PackedTritError> {
    if trits.len() > U3_U64_CAPACITY {
        return Err(PackedTritError::TooManyTrits {
            count: trits.len(),
            capacity: U3_U64_CAPACITY,
        });
    }
    let mut value = 0u128;
    let mut place = 1u128;
    for (index, &trit) in trits.iter().enumerate() {
        if trit > 2 {
            return Err(PackedTritError::InvalidTrit { index, value: trit });
        }
        value = value
            .checked_add(
                (trit as u128)
                    .checked_mul(place)
                    .ok_or(PackedTritError::Overflow { bits: 64 })?,
            )
            .ok_or(PackedTritError::Overflow { bits: 64 })?;
        place = place
            .checked_mul(3)
            .ok_or(PackedTritError::Overflow { bits: 64 })?;
    }
    Ok(value as u64)
}

/// Decode a u64 into exactly `count` unbalanced trits, least-significant first.
/// Rejects non-canonical values rather than silently discarding high trits.
pub fn unpack_u3_u64(value: u64, count: usize) -> Result<Vec<u8>, PackedTritError> {
    let limit = u3_limit(count, U3_U64_CAPACITY)?;
    if (value as u128) >= limit {
        return Err(PackedTritError::InvalidPackedValue { value, count });
    }
    let mut remaining = value;
    let mut trits = Vec::new();
    trits
        .try_reserve_exact(count)
        .map_err(|_| PackedTritError::AllocationFailure)?;
    for _ in 0..count {
        trits.push((remaining % 3) as u8);
        remaining /= 3;
    }
    Ok(trits)
}

/// Convert one unbalanced digit to its balanced logical equivalent.
pub fn u3_to_balanced(trit: u8) -> Result<i8, PackedTritError> {
    match trit {
        0 => Ok(-1),
        1 => Ok(0),
        2 => Ok(1),
        value => Err(PackedTritError::InvalidTrit { index: 0, value }),
    }
}

/// Convert one balanced digit to its unbalanced packing equivalent.
pub fn balanced_to_u3(trit: i8) -> Result<u8, PackedTritError> {
    match trit {
        -1 => Ok(0),
        0 => Ok(1),
        1 => Ok(2),
        value => Err(PackedTritError::InvalidBalancedTrit { value }),
    }
}

/// Serialize a balanced integer using the canonical TRN3 version 1 format.
///
/// The byte layout is `TRN3`, version byte, domain byte (`1` = balanced),
/// a little-endian `u32` digit count, then one byte per trit in
/// least-significant-first order (`0` = -1, `1` = 0, `2` = +1). The canonical
/// zero is a one-digit payload containing `1`. Unknown versions/domains are
/// rejected rather than interpreted using future semantics.
pub fn serialize_balanced(value: &Ternary) -> Result<Vec<u8>, TernaryError> {
    let count = u32::try_from(value.trits.len()).map_err(|_| TernaryError::SizeOverflow)?;
    let total_len = SERIAL_HEADER_LEN
        .checked_add(value.trits.len())
        .ok_or(TernaryError::SizeOverflow)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(total_len)
        .map_err(|_| TernaryError::AllocationFailure)?;
    bytes.extend_from_slice(SERIAL_MAGIC);
    bytes.push(SERIAL_VERSION);
    bytes.push(SERIAL_BALANCED_DOMAIN);
    bytes.extend_from_slice(&count.to_le_bytes());
    for (index, &trit) in value.trits.iter().enumerate() {
        let encoded = match trit {
            -1 => 0,
            0 => 1,
            1 => 2,
            other => {
                return Err(TernaryError::InvalidTrit {
                    index,
                    value: other,
                });
            }
        };
        bytes.push(encoded);
    }
    Ok(bytes)
}

/// Deserialize and validate one canonical TRN3 version 1 balanced integer.
pub fn deserialize_balanced(bytes: &[u8]) -> Result<Ternary, TernaryError> {
    if bytes.len() < SERIAL_HEADER_LEN {
        return Err(TernaryError::InvalidSerialization("truncated header"));
    }
    if bytes.get(..4) != Some(SERIAL_MAGIC.as_slice()) {
        return Err(TernaryError::InvalidSerialization("bad magic"));
    }
    if bytes.get(4) != Some(&SERIAL_VERSION) {
        return Err(TernaryError::InvalidSerialization("unsupported version"));
    }
    if bytes.get(5) != Some(&SERIAL_BALANCED_DOMAIN) {
        return Err(TernaryError::InvalidSerialization(
            "unsupported trit domain",
        ));
    }
    let count_bytes: [u8; 4] = bytes
        .get(6..10)
        .ok_or(TernaryError::InvalidSerialization("truncated digit count"))?
        .try_into()
        .map_err(|_| TernaryError::InvalidSerialization("invalid digit count"))?;
    let count = usize::try_from(u32::from_le_bytes(count_bytes))
        .map_err(|_| TernaryError::InvalidSerialization("digit count is too large"))?;
    if count == 0 {
        return Err(TernaryError::InvalidSerialization(
            "digit count must not be zero",
        ));
    }
    let expected_len =
        SERIAL_HEADER_LEN
            .checked_add(count)
            .ok_or(TernaryError::InvalidSerialization(
                "digit count overflows length",
            ))?;
    if bytes.len() != expected_len {
        return Err(TernaryError::InvalidSerialization(
            "payload length does not match digit count",
        ));
    }
    let payload = bytes
        .get(SERIAL_HEADER_LEN..)
        .ok_or(TernaryError::InvalidSerialization("missing digit payload"))?;
    if payload.last().copied() == Some(1) && count != 1 {
        return Err(TernaryError::InvalidSerialization(
            "leading zero trit is noncanonical",
        ));
    }
    let mut trits = Vec::new();
    trits
        .try_reserve_exact(count)
        .map_err(|_| TernaryError::AllocationFailure)?;
    for &digit in payload {
        trits.push(match digit {
            0 => -1,
            1 => 0,
            2 => 1,
            _ => return Err(TernaryError::InvalidSerialization("digit is outside 0..=2")),
        });
    }
    Ok(Ternary { trits })
}

#[cfg(test)]
mod packed_u3_tests {
    use super::*;

    #[test]
    fn packed_round_trips_and_boundaries() {
        for count in 0..=8 {
            let total = 3usize.pow(count as u32);
            for mut n in 0..total {
                let mut trits = Vec::with_capacity(count);
                for _ in 0..count {
                    trits.push((n % 3) as u8);
                    n /= 3;
                }
                assert_eq!(
                    unpack_u3_u32(pack_u3_u32(&trits).unwrap(), count).unwrap(),
                    trits
                );
                assert_eq!(
                    unpack_u3_u64(pack_u3_u64(&trits).unwrap(), count).unwrap(),
                    trits
                );
            }
        }
        assert_eq!(pack_u3_u32(&[2; 20]).unwrap(), 3u64.pow(20) as u32 - 1);
        assert_eq!(pack_u3_u64(&[2; 40]).unwrap(), 3u128.pow(40) as u64 - 1);
    }

    #[test]
    fn rejects_invalid_digits_counts_and_noncanonical_values() {
        assert!(matches!(
            pack_u3_u32(&[0, 3]),
            Err(PackedTritError::InvalidTrit { index: 1, value: 3 })
        ));
        assert!(matches!(
            pack_u3_u32(&[0; 21]),
            Err(PackedTritError::TooManyTrits { .. })
        ));
        assert!(matches!(
            unpack_u3_u32(3, 1),
            Err(PackedTritError::InvalidPackedValue { .. })
        ));
        assert!(matches!(
            unpack_u3_u64(1, 41),
            Err(PackedTritError::InvalidCount { .. })
        ));
        assert_eq!(unpack_u3_u32(0, 0).unwrap(), Vec::<u8>::new());
        assert!(matches!(
            unpack_u3_u32(1, 0),
            Err(PackedTritError::InvalidPackedValue { .. })
        ));
    }

    #[test]
    fn single_digit_domain_conversion_is_explicit() {
        for (u, b) in [(0, -1), (1, 0), (2, 1)] {
            assert_eq!(u3_to_balanced(u).unwrap(), b);
            assert_eq!(balanced_to_u3(b).unwrap(), u);
        }
        assert!(u3_to_balanced(3).is_err());
        assert!(balanced_to_u3(2).is_err());
    }

    #[test]
    fn packed_values_use_little_significance_first() {
        assert_eq!(pack_u3_u32(&[1, 2]).unwrap(), 7);
        assert_eq!(unpack_u3_u32(7, 2).unwrap(), vec![1, 2]);
    }

    #[test]
    fn packed_small_widths_and_comparisons_validate_boundaries() {
        for count in 0..=5 {
            let total = 3usize.pow(count as u32);
            for mut n in 0..total {
                let mut trits = Vec::with_capacity(count);
                for _ in 0..count {
                    trits.push((n % 3) as u8);
                    n /= 3;
                }
                assert_eq!(
                    unpack_u3_u8(pack_u3_u8(&trits).unwrap(), count).unwrap(),
                    trits
                );
            }
        }
        assert_eq!(pack_u3_u8(&[2; 5]).unwrap(), 242);
        assert!(matches!(
            pack_u3_u8(&[0; 6]),
            Err(PackedTritError::TooManyTrits { .. })
        ));
        assert!(matches!(
            unpack_u3_u8(1, 6),
            Err(PackedTritError::InvalidCount { .. })
        ));
        assert_eq!(pack_u3_u16(&[2; 10]).unwrap(), 59048);
        assert_eq!(unpack_u3_u16(59048, 10).unwrap(), vec![2; 10]);
        assert_eq!(compare_u3_u32(5, 6, 2).unwrap(), core::cmp::Ordering::Less);
        assert!(matches!(
            compare_u3_u32(3, 0, 1),
            Err(PackedTritError::InvalidPackedValue { .. })
        ));
        assert!(matches!(
            compare_u3_u64(0, 0, 41),
            Err(PackedTritError::InvalidCount { .. })
        ));
    }
}
