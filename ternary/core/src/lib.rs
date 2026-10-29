//! Balanced-ternary arithmetic for Trion.
//!
//! Trits are -1, 0, +1. Text uses T for -1, 0 for zero, and 1 for +1.
//! This is a software model that runs on conventional binary hardware.

use core::fmt;

/// A canonical balanced-ternary integer, stored least-significant trit first.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ternary { trits: Vec<i8> }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TernaryError { EmptyInput, InvalidDigit(char), DecimalOverflow }

impl fmt::Display for TernaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "ternary input cannot be empty"),
            Self::InvalidDigit(c) => write!(f, "invalid balanced-ternary digit '{c}' (use T, 0, 1)"),
            Self::DecimalOverflow => write!(f, "value is outside the i128 decimal range"),
        }
    }
}
impl std::error::Error for TernaryError {}

impl Ternary {
    pub fn zero() -> Self { Self { trits: vec![0] } }

    /// Convert decimal i128 without abs(), so i128::MIN is supported.
    pub fn from_i128(mut value: i128) -> Self {
        if value == 0 { return Self::zero(); }
        let mut trits = Vec::new();
        while value != 0 {
            let rem = value.rem_euclid(3);
            value = value.div_euclid(3);
            let digit = if rem == 2 { value += 1; -1 } else { rem as i8 };
            trits.push(digit);
        }
        Self { trits }
    }

    /// Parse most-significant-first text. T/t represents -1.
    pub fn parse(input: &str) -> Result<Self, TernaryError> {
        if input.is_empty() { return Err(TernaryError::EmptyInput); }
        let mut trits = Vec::with_capacity(input.len());
        for ch in input.chars().rev() {
            trits.push(match ch {
                'T' | 't' => -1, '0' => 0, '1' => 1,
                other => return Err(TernaryError::InvalidDigit(other)),
            });
        }
        let mut value = Self { trits };
        value.normalize();
        Ok(value)
    }

    pub fn trits_lsf(&self) -> &[i8] { &self.trits }

    pub fn to_i128(&self) -> Result<i128, TernaryError> {
        // Convert the magnitude in u128 so i128::MIN (whose magnitude is
        // one larger than i128::MAX) remains representable during conversion.
        let negative = self.trits.last().copied().unwrap_or(0) < 0;
        let mut magnitude = 0u128;
        for &trit in self.trits.iter().rev() {
            let digit = if negative { -trit } else { trit };
            magnitude = magnitude.checked_mul(3).ok_or(TernaryError::DecimalOverflow)?;
            let addend = digit as i128;
            if addend < 0 {
                magnitude = magnitude.checked_sub((-addend) as u128).ok_or(TernaryError::DecimalOverflow)?;
            } else {
                magnitude = magnitude.checked_add(addend as u128).ok_or(TernaryError::DecimalOverflow)?;
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

    pub fn add(&self, rhs: &Self) -> Self {
        let n = self.trits.len().max(rhs.trits.len());
        let mut out = Vec::with_capacity(n + 1);
        let mut carry = 0i8;
        for i in 0..n {
            let total = self.trits.get(i).copied().unwrap_or(0)
                + rhs.trits.get(i).copied().unwrap_or(0) + carry;
            let (digit, next) = normalize_small(total);
            out.push(digit);
            carry = next;
        }
        if carry != 0 { out.push(carry); }
        let mut result = Self { trits: out };
        result.normalize();
        result
    }

    pub fn sub(&self, rhs: &Self) -> Self { self.add(&rhs.negated()) }

    pub fn negated(&self) -> Self {
        let mut result = Self { trits: self.trits.iter().map(|t| -*t).collect() };
        result.normalize();
        result
    }

    /// Multiply via trit convolution and balanced carry normalization.
    pub fn mul(&self, rhs: &Self) -> Self {
        if self.is_zero() || rhs.is_zero() { return Self::zero(); }
        let mut accum = vec![0i64; self.trits.len() + rhs.trits.len() + 1];
        for (i, &a) in self.trits.iter().enumerate() {
            for (j, &b) in rhs.trits.iter().enumerate() { accum[i + j] += (a as i64) * (b as i64); }
        }
        let mut out = Vec::with_capacity(accum.len());
        let mut carry = 0i64;
        for coefficient in accum {
            let total = coefficient + carry;
            let rem = total.rem_euclid(3);
            let digit = match rem { 0 => 0, 1 => 1, _ => -1 };
            carry = (total - digit) / 3;
            out.push(digit as i8);
        }
        while carry != 0 {
            let rem = carry.rem_euclid(3);
            let digit = match rem { 0 => 0, 1 => 1, _ => -1 };
            carry = (carry - digit) / 3;
            out.push(digit as i8);
        }
        let mut result = Self { trits: out };
        result.normalize();
        result
    }

    pub fn is_zero(&self) -> bool { self.trits.iter().all(|&t| t == 0) }

    fn normalize(&mut self) {
        while self.trits.len() > 1 && self.trits.last() == Some(&0) { self.trits.pop(); }
        if self.trits.is_empty() { self.trits.push(0); }
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
            f.write_str(match trit { -1 => "T", 0 => "0", 1 => "1", _ => unreachable!() })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_values_round_trip() {
        let cases = [(0, "0"), (1, "1"), (2, "1T"), (3, "10"), (4, "11"),
            (5, "1TT"), (8, "10T"), (-1, "T"), (-2, "T1"), (-3, "T0"), (-4, "TT")];
        for (decimal, ternary) in cases {
            assert_eq!(Ternary::from_i128(decimal).to_string(), ternary);
            assert_eq!(Ternary::parse(ternary).unwrap().to_i128().unwrap(), decimal);
        }
    }

    #[test]
    fn i128_edges_round_trip() {
        for n in [i128::MIN, i128::MIN + 1, -1000, -2, -1, 0, 1, 2, 1000, i128::MAX] {
            assert_eq!(Ternary::from_i128(n).to_i128().unwrap(), n);
        }
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
        assert!(matches!(Ternary::parse("12"), Err(TernaryError::InvalidDigit('2'))));
    }

    #[test]
    fn arithmetic_can_exceed_i128() {
        let huge = Ternary::parse("1111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111").unwrap();
        let result = huge.add(&huge);
        assert!(result.to_i128().is_err());
        assert_eq!(result, huge.mul(&Ternary::from_i128(2)));
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
/// Maximum number of unbalanced trits that fit in a binary integer.
pub const U3_U64_CAPACITY: usize = 40;

/// Errors returned by checked unbalanced-ternary packing operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackedTritError {
    InvalidTrit { index: usize, value: u8 },
    TooManyTrits { count: usize, capacity: usize },
    InvalidCount { count: usize, capacity: usize },
    InvalidPackedValue { value: u64, count: usize },
}

impl fmt::Display for PackedTritError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTrit { index, value } =>
                write!(f, "unbalanced trit at index {index} is {value}; expected 0, 1, or 2"),
            Self::TooManyTrits { count, capacity } =>
                write!(f, "{count} trits exceed target capacity {capacity}"),
            Self::InvalidCount { count, capacity } =>
                write!(f, "trit count {count} exceeds capacity {capacity}"),
            Self::InvalidPackedValue { value, count } =>
                write!(f, "packed value {value} is invalid for {count} trits"),
        }
    }
}
impl std::error::Error for PackedTritError {}

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
        return Err(PackedTritError::TooManyTrits { count: trits.len(), capacity: U3_U32_CAPACITY });
    }
    let mut value = 0u64;
    let mut place = 1u64;
    for (index, &trit) in trits.iter().enumerate() {
        if trit > 2 {
            return Err(PackedTritError::InvalidTrit { index, value: trit });
        }
        value += (trit as u64) * place;
        place *= 3;
    }
    Ok(value as u32)
}

/// Decode a u32 into exactly `count` unbalanced trits, least-significant first.
/// Rejects non-canonical values rather than silently discarding high trits.
pub fn unpack_u3_u32(value: u32, count: usize) -> Result<Vec<u8>, PackedTritError> {
    let limit = u3_limit(count, U3_U32_CAPACITY)?;
    if (value as u128) >= limit {
        return Err(PackedTritError::InvalidPackedValue { value: value as u64, count });
    }
    let mut remaining = value;
    let mut trits = Vec::with_capacity(count);
    for _ in 0..count {
        trits.push((remaining % 3) as u8);
        remaining /= 3;
    }
    Ok(trits)
}

/// Pack unbalanced trits (each 0, 1, or 2) into a u64, least-significant first.
pub fn pack_u3_u64(trits: &[u8]) -> Result<u64, PackedTritError> {
    if trits.len() > U3_U64_CAPACITY {
        return Err(PackedTritError::TooManyTrits { count: trits.len(), capacity: U3_U64_CAPACITY });
    }
    let mut value = 0u128;
    let mut place = 1u128;
    for (index, &trit) in trits.iter().enumerate() {
        if trit > 2 {
            return Err(PackedTritError::InvalidTrit { index, value: trit });
        }
        value += (trit as u128) * place;
        place *= 3;
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
    let mut trits = Vec::with_capacity(count);
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
        value => Err(PackedTritError::InvalidTrit {
            index: 0,
            value: if value < 0 { 255 } else { value as u8 },
        }),
    }
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
                for _ in 0..count { trits.push((n % 3) as u8); n /= 3; }
                assert_eq!(unpack_u3_u32(pack_u3_u32(&trits).unwrap(), count).unwrap(), trits);
                assert_eq!(unpack_u3_u64(pack_u3_u64(&trits).unwrap(), count).unwrap(), trits);
            }
        }
        assert_eq!(pack_u3_u32(&vec![2; 20]).unwrap(), 3u64.pow(20) as u32 - 1);
        assert_eq!(pack_u3_u64(&vec![2; 40]).unwrap(), 3u128.pow(40) as u64 - 1);
    }

    #[test]
    fn rejects_invalid_digits_counts_and_noncanonical_values() {
        assert!(matches!(pack_u3_u32(&[0, 3]), Err(PackedTritError::InvalidTrit { index: 1, value: 3 })));
        assert!(matches!(pack_u3_u32(&vec![0; 21]), Err(PackedTritError::TooManyTrits { .. })));
        assert!(matches!(unpack_u3_u32(3, 1), Err(PackedTritError::InvalidPackedValue { .. })));
        assert!(matches!(unpack_u3_u64(1, 41), Err(PackedTritError::InvalidCount { .. })));
        assert_eq!(unpack_u3_u32(0, 0).unwrap(), Vec::<u8>::new());
        assert!(matches!(unpack_u3_u32(1, 0), Err(PackedTritError::InvalidPackedValue { .. })));
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
}
