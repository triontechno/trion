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
        let mut value = 0i128;
        for &trit in self.trits.iter().rev() {
            value = value.checked_mul(3).ok_or(TernaryError::DecimalOverflow)?;
            value = value.checked_add(trit as i128).ok_or(TernaryError::DecimalOverflow)?;
        }
        Ok(value)
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
        let huge = Ternary::parse("111111111111111111111111111111111111111111111111111111").unwrap();
        let result = huge.add(&huge);
        assert!(result.to_i128().is_err());
        assert_eq!(result, huge.mul(&Ternary::from_i128(2)));
    }
}
