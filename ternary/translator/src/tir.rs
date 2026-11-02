use goblin::elf::{header, section_header};
use iced_x86::{Code, Decoder, DecoderOptions, Mnemonic, Register};
use std::fmt::Write as _;
use trion_ternary::Ternary;

const TIR_HEADER: &str = "TRION-TIR 1 x86_64";

fn x86_elf(bytes: &[u8]) -> Result<goblin::elf::Elf<'_>, String> {
    let object = goblin::Object::parse(bytes)
        .map_err(|error| format!("not a recognized object file: {error}"))?;
    let elf = match object {
        goblin::Object::Elf(elf) => elf,
        _ => return Err("input is not an ELF file".to_string()),
    };
    if !elf.is_64 || elf.header.e_machine != header::EM_X86_64 {
        return Err(format!(
            "unsupported ELF architecture (class 64-bit: {}, machine: {}); expected x86-64",
            elf.is_64, elf.header.e_machine
        ));
    }
    if let Some((index, _)) = elf.section_headers.iter().enumerate().find(|(_, section)| {
        section.sh_type == section_header::SHT_REL || section.sh_type == section_header::SHT_RELA
    }) {
        return Err(format!(
            "ELF relocation section {index} is unsupported; relocated code cannot be translated"
        ));
    }
    Ok(elf)
}

fn register_id(register: Register) -> Option<u8> {
    Some(match register {
        Register::RAX => 0,
        Register::RCX => 1,
        Register::RDX => 2,
        Register::RBX => 3,
        Register::RSP => 4,
        Register::RBP => 5,
        Register::RSI => 6,
        Register::RDI => 7,
        Register::R8 => 8,
        Register::R9 => 9,
        Register::R10 => 10,
        Register::R11 => 11,
        Register::R12 => 12,
        Register::R13 => 13,
        Register::R14 => 14,
        Register::R15 => 15,
        _ => return None,
    })
}

/// Translate the deliberately small, semantics-preserving x86-64 subset to TIR v1.
///
/// The returned stream is validated before it is exposed. Unsupported
/// instructions reject the whole input; no partial translation is returned.
pub fn translate_elf_to_tir(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let elf = x86_elf(bytes)?;
    let mut output = format!("{TIR_HEADER}\n");
    let mut executable_sections = 0usize;
    let mut instructions = 0usize;

    for (section_index, section) in elf.section_headers.iter().enumerate() {
        if section.sh_flags & (section_header::SHF_EXECINSTR as u64) == 0 || section.sh_size == 0 {
            continue;
        }
        let start = usize::try_from(section.sh_offset)
            .map_err(|_| format!("section {section_index} offset is too large"))?;
        let size = usize::try_from(section.sh_size)
            .map_err(|_| format!("section {section_index} size is too large"))?;
        let end = start.checked_add(size)
            .ok_or_else(|| format!("section {section_index} bounds overflow"))?;
        section.sh_addr.checked_add(section.sh_size)
            .ok_or_else(|| format!("section {section_index} virtual address overflows"))?;
        let code = bytes.get(start..end)
            .ok_or_else(|| format!("section {section_index} lies outside the file"))?;
        executable_sections += 1;

        let mut decoder = Decoder::with_ip(64, code, section.sh_addr, DecoderOptions::NONE);
        while decoder.can_decode() {
            let instruction = decoder.decode();
            let ip = instruction.ip();
            if instruction.is_invalid() {
                return Err(format!("invalid x86-64 instruction encoding at 0x{ip:016x}"));
            }

            if instruction.mnemonic() == Mnemonic::Nop {
                writeln!(output, "{ip:016x} NOP").expect("writing to String cannot fail");
            } else if instruction.code() == Code::Mov_r64_imm64 {
                let register = register_id(instruction.op0_register())
                    .ok_or_else(|| format!("unsupported MOV destination register at 0x{ip:016x}"))?;
                let immediate = Ternary::from_u128(instruction.immediate64() as u128);
                writeln!(output, "{ip:016x} MOV_IMM64 r{register} {immediate}")
                    .expect("writing to String cannot fail");
            } else if instruction.code() == Code::Retnq {
                writeln!(output, "{ip:016x} RET").expect("writing to String cannot fail");
            } else {
                return Err(format!(
                    "unsupported x86-64 instruction {:?} at 0x{ip:016x}",
                    instruction.mnemonic()
                ));
            }
            instructions = instructions.checked_add(1)
                .ok_or_else(|| "instruction count overflow".to_string())?;
        }
    }

    if executable_sections == 0 {
        return Err("ELF contains no nonempty executable sections".to_string());
    }
    if instructions == 0 {
        return Err("ELF contains no translatable instructions".to_string());
    }
    let output = output.into_bytes();
    validate_tir(&output)?;
    Ok(output)
}

/// Validate a canonical textual TIR v1 stream and return its instruction count.
pub fn validate_tir(bytes: &[u8]) -> Result<usize, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| format!("TIR is not UTF-8: {error}"))?;
    if !text.ends_with('\n') {
        return Err("TIR must end with a newline".to_string());
    }
    let mut lines = text.split_terminator('\n');
    if lines.next() != Some(TIR_HEADER) {
        return Err("unsupported TIR header or version".to_string());
    }

    let mut count = 0usize;
    for (index, line) in lines.enumerate() {
        let fields: Vec<&str> = line.split(' ').collect();
        if fields.iter().any(|field| field.is_empty()) || fields.len() < 2 {
            return Err(format!("invalid TIR record on line {}", index + 2));
        }
        if fields[0].len() != 16
            || !fields[0].bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(format!("invalid instruction address on line {}", index + 2));
        }
        u64::from_str_radix(fields[0], 16)
            .map_err(|_| format!("invalid instruction address on line {}", index + 2))?;
        match fields.as_slice() {
            [_, "NOP"] | [_, "RET"] => {}
            [_, "MOV_IMM64", register, immediate] => {
                let register_id = register.strip_prefix('r')
                    .and_then(|value| value.parse::<u8>().ok())
                    .filter(|value| *value < 16)
                    .ok_or_else(|| format!("invalid register on line {}", index + 2))?;
                if register.as_bytes() != format!("r{register_id}").as_bytes() {
                    return Err(format!("noncanonical register on line {}", index + 2));
                }
                let value = Ternary::parse(immediate)
                    .map_err(|error| format!("invalid immediate on line {}: {error}", index + 2))?;
                let unsigned = value.to_u128()
                    .map_err(|_| format!("immediate is negative on line {}", index + 2))?;
                if value.to_string().as_str() != *immediate || unsigned > u64::MAX as u128 {
                    return Err(format!("noncanonical or out-of-range immediate on line {}", index + 2));
                }
            }
            _ => return Err(format!("unsupported TIR operation on line {}", index + 2)),
        }
        count = count.checked_add(1).ok_or_else(|| "TIR instruction count overflow".to_string())?;
    }
    if count == 0 {
        return Err("TIR contains no instructions".to_string());
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_elf(code: &[u8]) -> Vec<u8> {
        let code_offset = 192usize;
        let mut bytes = vec![0u8; code_offset + code.len()];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2; // ELFCLASS64
        bytes[5] = 1; // ELFDATA2LSB
        bytes[6] = 1; // EV_CURRENT
        bytes[16..18].copy_from_slice(&2u16.to_le_bytes()); // ET_EXEC
        bytes[18..20].copy_from_slice(&header::EM_X86_64.to_le_bytes());
        bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
        bytes[24..32].copy_from_slice(&0x400000u64.to_le_bytes());
        bytes[40..48].copy_from_slice(&64u64.to_le_bytes());
        bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
        bytes[58..60].copy_from_slice(&64u16.to_le_bytes());
        bytes[60..62].copy_from_slice(&2u16.to_le_bytes());
        let section = 128usize;
        bytes[section + 4..section + 8].copy_from_slice(&section_header::SHT_PROGBITS.to_le_bytes());
        bytes[section + 8..section + 16].copy_from_slice(
            &((section_header::SHF_ALLOC | section_header::SHF_EXECINSTR) as u64).to_le_bytes(),
        );
        bytes[section + 16..section + 24].copy_from_slice(&0x400000u64.to_le_bytes());
        bytes[section + 24..section + 32].copy_from_slice(&(code_offset as u64).to_le_bytes());
        bytes[section + 32..section + 40].copy_from_slice(&(code.len() as u64).to_le_bytes());
        bytes[section + 48..section + 56].copy_from_slice(&1u64.to_le_bytes());
        bytes[code_offset..].copy_from_slice(code);
        bytes
    }

    #[test]
    fn translates_and_validates_supported_subset() {
        let elf = minimal_elf(&[0x90, 0x48, 0xb8, 1, 0, 0, 0, 0, 0, 0, 0, 0xc3]);
        let tir = translate_elf_to_tir(&elf).unwrap();
        assert_eq!(
            std::str::from_utf8(&tir).unwrap(),
            "TRION-TIR 1 x86_64\n\
             0000000000400000 NOP\n\
             0000000000400001 MOV_IMM64 r0 1\n\
             000000000040000b RET\n"
        );
        assert_eq!(validate_tir(&tir).unwrap(), 3);
    }

    #[test]
    fn rejects_bad_architecture_malformed_elf_and_unsupported_instructions() {
        assert!(translate_elf_to_tir(b"not an ELF").unwrap_err().contains("not a recognized"));
        let mut wrong_arch = minimal_elf(&[0xc3]);
        wrong_arch[18..20].copy_from_slice(&header::EM_386.to_le_bytes());
        assert!(translate_elf_to_tir(&wrong_arch).unwrap_err().contains("expected x86-64"));
        assert!(translate_elf_to_tir(&minimal_elf(&[0xcc])).unwrap_err().contains("unsupported"));
        assert!(translate_elf_to_tir(&minimal_elf(&[0x48])).unwrap_err().contains("invalid"));
    }

    #[test]
    fn rejects_elf_with_relocation_sections() {
        let mut elf = minimal_elf(&[0xc3]);
        let relocation_section = 128 + 64;
        elf[relocation_section + 4..relocation_section + 8]
            .copy_from_slice(&section_header::SHT_RELA.to_le_bytes());
        elf[relocation_section + 32..relocation_section + 40]
            .copy_from_slice(&0u64.to_le_bytes());
        assert!(translate_elf_to_tir(&elf).unwrap_err().contains("relocation section"));
    }

    #[test]
    fn rejects_noncanonical_or_out_of_range_tir() {
        assert!(validate_tir(b"TRION-TIR 2 x86_64\n0000000000000000 NOP\n").is_err());
        assert!(validate_tir(b"TRION-TIR 1 x86_64\n0000000000000000 MOV_IMM64 r0 T\n").is_err());
        assert!(validate_tir(b"TRION-TIR 1 x86_64\n0000000000000000 MOV_IMM64 r16 1\n").is_err());
        assert!(validate_tir(b"TRION-TIR 1 x86_64\n0000000000000000 NOP").is_err());
    }
}
