//! Versioned TRION interpreted executable container (TRIONEX v1).
#![forbid(unsafe_code)]

use core::fmt;
use trion_tir_runtime::validate_tir;

const MAGIC: &[u8; 8] = b"TRIONEX\0";
const VERSION: u16 = 1;
const HEADER_LEN: usize = 36;
const ARCH_X86_64_TIR: u16 = 1;
const PAYLOAD_TIR_V1: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutableError {
    TruncatedHeader, InvalidMagic, UnsupportedVersion, InvalidHeaderLength,
    UnsupportedArchitecture, UnsupportedPayloadKind, UnsupportedFlags,
    NonzeroReservedField, LengthOverflow, LengthMismatch, ChecksumMismatch,
    InvalidTirPayload, AllocationFailure,
}

impl fmt::Display for ExecutableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TruncatedHeader => "TRIONEX header is truncated",
            Self::InvalidMagic => "invalid TRIONEX magic",
            Self::UnsupportedVersion => "unsupported TRIONEX version",
            Self::InvalidHeaderLength => "invalid TRIONEX header length",
            Self::UnsupportedArchitecture => "unsupported TRIONEX architecture",
            Self::UnsupportedPayloadKind => "unsupported TRIONEX payload kind",
            Self::UnsupportedFlags => "unsupported TRIONEX flags",
            Self::NonzeroReservedField => "TRIONEX reserved field must be zero",
            Self::LengthOverflow => "TRIONEX length overflow",
            Self::LengthMismatch => "TRIONEX payload length does not match file",
            Self::ChecksumMismatch => "TRIONEX payload checksum mismatch",
            Self::InvalidTirPayload => "TRIONEX payload is not valid TIR v1",
            Self::AllocationFailure => "unable to allocate TRIONEX buffer",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ExecutableError {}

/// A validated TRIONEX v1 interpreted executable containing canonical TIR v1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrionExecutable {
    payload: Vec<u8>,
}

impl TrionExecutable {
    pub fn from_tir(payload: &[u8]) -> Result<Self, ExecutableError> {
        validate_tir(payload).map_err(|_| ExecutableError::InvalidTirPayload)?;
        let mut owned = Vec::new();
        owned.try_reserve_exact(payload.len())
            .map_err(|_| ExecutableError::AllocationFailure)?;
        owned.extend_from_slice(payload);
        Ok(Self { payload: owned })
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, ExecutableError> {
        if bytes.len() < HEADER_LEN {
            return Err(ExecutableError::TruncatedHeader);
        }
        if bytes.get(0..8) != Some(MAGIC.as_slice()) {
            return Err(ExecutableError::InvalidMagic);
        }
        if read_u16(bytes, 8)? != VERSION {
            return Err(ExecutableError::UnsupportedVersion);
        }
        if usize::from(read_u16(bytes, 10)?) != HEADER_LEN {
            return Err(ExecutableError::InvalidHeaderLength);
        }
        if read_u16(bytes, 12)? != ARCH_X86_64_TIR {
            return Err(ExecutableError::UnsupportedArchitecture);
        }
        if read_u16(bytes, 14)? != PAYLOAD_TIR_V1 {
            return Err(ExecutableError::UnsupportedPayloadKind);
        }
        if read_u32(bytes, 16)? != 0 {
            return Err(ExecutableError::UnsupportedFlags);
        }
        if read_u32(bytes, 20)? != 0 {
            return Err(ExecutableError::NonzeroReservedField);
        }

        let payload_len = usize::try_from(read_u64(bytes, 24)?)
            .map_err(|_| ExecutableError::LengthOverflow)?;
        let expected_len = HEADER_LEN.checked_add(payload_len)
            .ok_or(ExecutableError::LengthOverflow)?;
        if bytes.len() != expected_len {
            return Err(ExecutableError::LengthMismatch);
        }
        let payload = bytes.get(HEADER_LEN..expected_len)
            .ok_or(ExecutableError::LengthMismatch)?;
        if crc32(payload) != read_u32(bytes, 32)? {
            return Err(ExecutableError::ChecksumMismatch);
        }
        Self::from_tir(payload)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ExecutableError> {
        let payload_len = u64::try_from(self.payload.len())
            .map_err(|_| ExecutableError::LengthOverflow)?;
        let total_len = HEADER_LEN.checked_add(self.payload.len())
            .ok_or(ExecutableError::LengthOverflow)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(total_len)
            .map_err(|_| ExecutableError::AllocationFailure)?;
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&36u16.to_le_bytes());
        bytes.extend_from_slice(&ARCH_X86_64_TIR.to_le_bytes());
        bytes.extend_from_slice(&PAYLOAD_TIR_V1.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&payload_len.to_le_bytes());
        bytes.extend_from_slice(&crc32(&self.payload).to_le_bytes());
        bytes.extend_from_slice(&self.payload);
        Ok(bytes)
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ExecutableError> {
    let end = offset.checked_add(2).ok_or(ExecutableError::LengthOverflow)?;
    let field = bytes.get(offset..end).ok_or(ExecutableError::TruncatedHeader)?;
    Ok(u16::from_le_bytes([field[0], field[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ExecutableError> {
    let end = offset.checked_add(4).ok_or(ExecutableError::LengthOverflow)?;
    let field = bytes.get(offset..end).ok_or(ExecutableError::TruncatedHeader)?;
    Ok(u32::from_le_bytes([field[0], field[1], field[2], field[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ExecutableError> {
    let end = offset.checked_add(8).ok_or(ExecutableError::LengthOverflow)?;
    let field = bytes.get(offset..end).ok_or(ExecutableError::TruncatedHeader)?;
    Ok(u64::from_le_bytes([
        field[0], field[1], field[2], field[3], field[4], field[5], field[6], field[7],
    ]))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    const TIR: &[u8] = b"TRION-TIR 1 x86_64\n0000000000400000 NOP\n";

    #[test]
    fn round_trips_valid_tir() {
        let executable = TrionExecutable::from_tir(TIR).unwrap();
        let bytes = executable.to_bytes().unwrap();
        let parsed = TrionExecutable::parse(&bytes).unwrap();
        assert_eq!(parsed, executable);
        assert_eq!(parsed.payload(), TIR);
    }

    #[test]
    fn rejects_truncated_header_and_bad_magic() {
        assert_eq!(TrionExecutable::parse(&[0; 10]), Err(ExecutableError::TruncatedHeader));
        let mut bytes = TrionExecutable::from_tir(TIR).unwrap().to_bytes().unwrap();
        bytes[0] = b'X';
        assert_eq!(TrionExecutable::parse(&bytes), Err(ExecutableError::InvalidMagic));
    }

    #[test]
    fn rejects_bad_version_architecture_flags_and_reserved() {
        let original = TrionExecutable::from_tir(TIR).unwrap().to_bytes().unwrap();
        let mut bytes = original.clone();
        bytes[8] = 2;
        assert_eq!(TrionExecutable::parse(&bytes), Err(ExecutableError::UnsupportedVersion));
        let mut bytes = original.clone();
        bytes[12] = 2;
        assert_eq!(TrionExecutable::parse(&bytes), Err(ExecutableError::UnsupportedArchitecture));
        let mut bytes = original.clone();
        bytes[16] = 1;
        assert_eq!(TrionExecutable::parse(&bytes), Err(ExecutableError::UnsupportedFlags));
        let mut bytes = original;
        bytes[20] = 1;
        assert_eq!(TrionExecutable::parse(&bytes), Err(ExecutableError::NonzeroReservedField));
    }

    #[test]
    fn rejects_length_and_checksum_mismatch() {
        let original = TrionExecutable::from_tir(TIR).unwrap().to_bytes().unwrap();
        let mut bytes = original.clone();
        bytes[24] = bytes[24].wrapping_add(1);
        assert_eq!(TrionExecutable::parse(&bytes), Err(ExecutableError::LengthMismatch));
        let mut bytes = original;
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        assert_eq!(TrionExecutable::parse(&bytes), Err(ExecutableError::ChecksumMismatch));
    }

    #[test]
    fn rejects_invalid_tir_payload() {
        let invalid = b"TRION-TIR 1 x86_64\n0000000000400000 ADD r0 r1\n";
        assert_eq!(TrionExecutable::from_tir(invalid), Err(ExecutableError::InvalidTirPayload));
    }

    #[test]
    fn crc32_matches_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }
}
