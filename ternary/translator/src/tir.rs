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

fn encode_code_sections(sections: Vec<(u64, Vec<u8>)>) -> Result<Vec<u8>, String> {
    let mut output = format!("{TIR_HEADER}\n");
    let mut instructions = 0usize;
    for (base, code) in sections {
        let mut decoder = Decoder::with_ip(64, &code, base, DecoderOptions::NONE);
        while decoder.can_decode() {
            let instruction = decoder.decode();
            let ip = instruction.ip();
            if instruction.is_invalid() {
                return Err(format!(
                    "invalid x86-64 instruction encoding at 0x{ip:016x}"
                ));
            }
            let op = match instruction.mnemonic() {
                Mnemonic::Nop => "NOP".to_string(),
                Mnemonic::Ret => "RET".to_string(),
                Mnemonic::Mov
                    if instruction.op_count() == 2
                        && instruction.op_kind(0) == iced_x86::OpKind::Register
                        && instruction.op_kind(1) == iced_x86::OpKind::Register =>
                {
                    let dst = register_id(instruction.op_register(0))
                        .ok_or_else(|| format!("unsupported MOV destination at 0x{ip:016x}"))?;
                    let src = register_id(instruction.op_register(1))
                        .ok_or_else(|| format!("unsupported MOV source at 0x{ip:016x}"))?;
                    format!("MOV r{dst} r{src}")
                }
                Mnemonic::Add | Mnemonic::Sub | Mnemonic::Cmp
                    if instruction.op_count() == 2
                        && instruction.op_kind(0) == iced_x86::OpKind::Register
                        && instruction.op_kind(1) == iced_x86::OpKind::Register =>
                {
                    let dst = register_id(instruction.op_register(0)).ok_or_else(|| {
                        format!("unsupported arithmetic destination at 0x{ip:016x}")
                    })?;
                    let src = register_id(instruction.op_register(1))
                        .ok_or_else(|| format!("unsupported arithmetic source at 0x{ip:016x}"))?;
                    let mnemonic = match instruction.mnemonic() {
                        Mnemonic::Add => "ADD",
                        Mnemonic::Sub => "SUB",
                        Mnemonic::Cmp => "CMP",
                        _ => unreachable!(),
                    };
                    format!("{mnemonic} r{dst} r{src}")
                }
                Mnemonic::Jmp
                | Mnemonic::Je
                | Mnemonic::Jne
                | Mnemonic::Jl
                | Mnemonic::Jle
                | Mnemonic::Jg
                | Mnemonic::Jge
                | Mnemonic::Call => {
                    if instruction.op_kind(0) != iced_x86::OpKind::NearBranch64 {
                        return Err(format!(
                            "indirect control flow is unsupported at 0x{ip:016x}"
                        ));
                    }
                    let mnemonic = match instruction.mnemonic() {
                        Mnemonic::Jmp => "JMP",
                        Mnemonic::Je => "JZ",
                        Mnemonic::Jne => "JNZ",
                        Mnemonic::Jl => "JL",
                        Mnemonic::Jle => "JLE",
                        Mnemonic::Jg => "JG",
                        Mnemonic::Jge => "JGE",
                        Mnemonic::Call => "CALL",
                        _ => unreachable!(),
                    };
                    format!("{mnemonic} {:016x}", instruction.near_branch_target())
                }
                _ if instruction.code() == Code::Mov_r64_imm64 => {
                    let register = register_id(instruction.op0_register()).ok_or_else(|| {
                        format!("unsupported MOV destination register at 0x{ip:016x}")
                    })?;
                    let immediate = Ternary::from_u128(instruction.immediate64() as u128);
                    format!("MOV_IMM64 r{register} {immediate}")
                }
                _ => {
                    return Err(format!(
                        "unsupported x86-64 instruction {:?} at 0x{ip:016x}",
                        instruction.mnemonic()
                    ));
                }
            };
            writeln!(output, "{ip:016x} {op}").expect("writing to String cannot fail");
            instructions = instructions
                .checked_add(1)
                .ok_or_else(|| "instruction count overflow".to_string())?;
        }
    }
    if instructions == 0 {
        return Err("input contains no translatable instructions".to_string());
    }
    let bytes = output.into_bytes();
    validate_tir(&bytes)?;
    Ok(bytes)
}

/// Translate a 64-bit x86 Windows PE image. This imports executable sections
/// into the TRION TIR runtime; Win32/NT APIs and the Windows process ABI are not emulated.
pub fn translate_pe_to_tir(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let pe = match goblin::pe::PE::parse(bytes) {
        Ok(pe) => pe,
        Err(error) => return Err(format!("invalid PE executable: {error}")),
    };
    if !pe.is_64 || pe.header.coff_header.machine != 0x8664 {
        return Err("only x86-64 PE executables are supported".into());
    }
    let mut sections = Vec::new();
    for (index, section) in pe.sections.iter().enumerate() {
        if section.characteristics & 0x2000_0000 == 0 || section.size_of_raw_data == 0 {
            continue;
        }
        let start = usize::try_from(section.pointer_to_raw_data)
            .map_err(|_| format!("PE section {index} offset too large"))?;
        let size = usize::try_from(section.size_of_raw_data)
            .map_err(|_| format!("PE section {index} size too large"))?;
        let end = start
            .checked_add(size)
            .ok_or("PE section bounds overflow")?;
        let code = bytes
            .get(start..end)
            .ok_or_else(|| format!("PE section {index} lies outside file"))?;
        let image_base = u64::try_from(pe.image_base)
            .map_err(|_| "PE image base exceeds 64-bit address space")?;
        let base = image_base
            .checked_add(u64::from(section.virtual_address))
            .ok_or("PE section virtual address overflow")?;
        sections.push((base, code.to_vec()));
    }
    encode_code_sections(sections)
}

/// Translate a thin little-endian x86-64 Mach-O image. Universal/fat binaries,
/// ARM64, external relocations, and the macOS process/framework ABI are rejected.
pub fn translate_macho_to_tir(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < 32 || bytes.get(0..4) != Some(&[0xcf, 0xfa, 0xed, 0xfe]) {
        return Err("only thin little-endian 64-bit Mach-O images are supported".into());
    }
    let read_u32 = |offset: usize| -> Result<u32, String> {
        let end = offset.checked_add(4).ok_or("Mach-O offset overflow")?;
        let field = bytes.get(offset..end).ok_or("truncated Mach-O header")?;
        Ok(u32::from_le_bytes(
            field.try_into().map_err(|_| "invalid Mach-O field")?,
        ))
    };
    let read_u64 = |offset: usize| -> Result<u64, String> {
        let end = offset.checked_add(8).ok_or("Mach-O offset overflow")?;
        let field = bytes.get(offset..end).ok_or("truncated Mach-O header")?;
        Ok(u64::from_le_bytes(
            field.try_into().map_err(|_| "invalid Mach-O field")?,
        ))
    };
    if read_u32(4)? != 0x0100_0007 {
        return Err("only x86-64 Mach-O executables are supported".into());
    }
    let ncmds = usize::try_from(read_u32(16)?).map_err(|_| "Mach-O command count too large")?;
    let sizeofcmds = usize::try_from(read_u32(20)?).map_err(|_| "Mach-O command size too large")?;
    let commands_end = 32usize
        .checked_add(sizeofcmds)
        .ok_or("Mach-O command bounds overflow")?;
    if commands_end > bytes.len() {
        return Err("Mach-O load commands exceed file bounds".into());
    }
    let mut cursor = 32usize;
    let mut sections = Vec::new();
    for _ in 0..ncmds {
        let cmd = read_u32(cursor)?;
        let cmdsize =
            usize::try_from(read_u32(cursor + 4)?).map_err(|_| "Mach-O command too large")?;
        if cmdsize < 8
            || cursor
                .checked_add(cmdsize)
                .ok_or("Mach-O command overflow")?
                > commands_end
        {
            return Err("malformed Mach-O load command".into());
        }
        if cmd == 0x19 {
            if cmdsize < 72 {
                return Err("truncated LC_SEGMENT_64 command".into());
            }
            let initprot = read_u32(cursor + 60)?;
            let nsects = usize::try_from(read_u32(cursor + 64)?)
                .map_err(|_| "Mach-O section count too large")?;
            if initprot & 4 != 0 {
                let section_start = cursor + 72;
                let required = nsects
                    .checked_mul(80)
                    .and_then(|n| n.checked_add(section_start))
                    .ok_or("Mach-O section table overflow")?;
                if required > cursor + cmdsize {
                    return Err("Mach-O section table exceeds segment command".into());
                }
                for section_index in 0..nsects {
                    let section = section_start + section_index * 80;
                    let address = read_u64(section + 32)?;
                    let size = usize::try_from(read_u64(section + 40)?)
                        .map_err(|_| "Mach-O section too large")?;
                    let offset = usize::try_from(read_u32(section + 48)?)
                        .map_err(|_| "Mach-O section offset too large")?;
                    let relocation_count = read_u32(section + 60)?;
                    if relocation_count != 0 {
                        return Err(format!(
                            "Mach-O section {section_index} contains relocations that are not yet supported"
                        ));
                    }
                    if size == 0 {
                        continue;
                    }
                    let end = offset
                        .checked_add(size)
                        .ok_or("Mach-O section bounds overflow")?;
                    let code = bytes
                        .get(offset..end)
                        .ok_or("Mach-O executable section lies outside file")?;
                    sections.push((address, code.to_vec()));
                }
            }
        }
        cursor += cmdsize;
    }
    encode_code_sections(sections)
}

/// Translate ELF, PE, or thin x86-64 Mach-O code into the common TIR format.
pub fn translate_object_to_tir(bytes: &[u8]) -> Result<Vec<u8>, String> {
    match goblin::Object::parse(bytes)
        .map_err(|error| format!("unrecognised executable: {error}"))?
    {
        goblin::Object::Elf(_) => translate_elf_to_tir(bytes),
        goblin::Object::PE(_) => translate_pe_to_tir(bytes),
        goblin::Object::Mach(_) => translate_macho_to_tir(bytes),
        _ => Err("unsupported executable object format".into()),
    }
}

/// Translate the deliberately small, semantics-preserving x86-64 subset to TIR v1.
///
/// The returned stream is validated before it is exposed. Unsupported
/// instructions reject the whole input; no partial translation is returned.
fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let field = bytes
        .get(offset..offset.checked_add(2).ok_or("ELF offset overflow")?)
        .ok_or("truncated ELF u16 field")?;
    Ok(u16::from_le_bytes([field[0], field[1]]))
}
fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, String> {
    let field = bytes
        .get(offset..offset.checked_add(8).ok_or("ELF offset overflow")?)
        .ok_or("truncated ELF u64 field")?;
    Ok(u64::from_le_bytes(
        field.try_into().map_err(|_| "invalid ELF u64")?,
    ))
}

/// Apply statically resolvable x86-64 RELA relocations to one executable section.
/// ET_REL is rejected because it has no final section layout; unresolved/dynamic
/// absolute relocations are rejected instead of being guessed.
fn relocate_executable_section(
    bytes: &[u8],
    elf: &goblin::elf::Elf<'_>,
    target_index: usize,
) -> Result<Vec<u8>, String> {
    let target = elf
        .section_headers
        .get(target_index)
        .ok_or("invalid target section index")?;
    let start = usize::try_from(target.sh_offset).map_err(|_| "section offset too large")?;
    let size = usize::try_from(target.sh_size).map_err(|_| "section size too large")?;
    let end = start.checked_add(size).ok_or("section bounds overflow")?;
    let mut code = bytes
        .get(start..end)
        .ok_or("executable section lies outside file")?
        .to_vec();

    for (rel_index, rela) in elf.section_headers.iter().enumerate() {
        if rela.sh_info as usize != target_index
            || (rela.sh_type != section_header::SHT_RELA && rela.sh_type != section_header::SHT_REL)
        {
            continue;
        }
        if rela.sh_type == section_header::SHT_REL {
            return Err(format!(
                "ELF REL relocation section {rel_index} is unsupported; explicit addends are required"
            ));
        }
        if elf.header.e_type == header::ET_REL {
            return Err(format!(
                "ELF relocatable object has no final section layout (relocation section {rel_index})"
            ));
        }
        let symtab_index = rela.sh_link as usize;
        let symtab = elf.section_headers.get(symtab_index).ok_or_else(|| {
            format!("relocation section {rel_index} references an invalid symbol table")
        })?;
        if symtab.sh_type != section_header::SHT_SYMTAB
            && symtab.sh_type != section_header::SHT_DYNSYM
        {
            return Err(format!(
                "relocation section {rel_index} does not reference a symbol table"
            ));
        }
        let rela_start =
            usize::try_from(rela.sh_offset).map_err(|_| "relocation offset too large")?;
        let rela_size = usize::try_from(rela.sh_size).map_err(|_| "relocation size too large")?;
        let rela_end = rela_start
            .checked_add(rela_size)
            .ok_or("relocation bounds overflow")?;
        let entry_size =
            usize::try_from(rela.sh_entsize).map_err(|_| "relocation entry size too large")?;
        if entry_size < 24 || rela_size % entry_size != 0 {
            return Err(format!("malformed RELA section {rel_index}"));
        }
        let sym_start =
            usize::try_from(symtab.sh_offset).map_err(|_| "symbol table offset too large")?;
        let sym_size =
            usize::try_from(symtab.sh_size).map_err(|_| "symbol table size too large")?;
        let sym_end = sym_start
            .checked_add(sym_size)
            .ok_or("symbol table bounds overflow")?;
        let sym_entsize =
            usize::try_from(symtab.sh_entsize).map_err(|_| "symbol entry size too large")?;
        if sym_entsize < 24 || sym_size % sym_entsize != 0 {
            return Err(format!(
                "malformed symbol table for relocation section {rel_index}"
            ));
        }
        let rel_data = bytes
            .get(rela_start..rela_end)
            .ok_or("RELA section lies outside file")?;
        for entry in (0..rela_size).step_by(entry_size) {
            let r_offset = read_u64(rel_data, entry)?;
            let r_info = read_u64(rel_data, entry + 8)?;
            let addend = read_u64(rel_data, entry + 16)? as i64;
            let symbol_index =
                usize::try_from(r_info >> 32).map_err(|_| "symbol index too large")?;
            let relocation_type = r_info as u32;
            let symbol_offset = sym_start
                .checked_add(
                    symbol_index
                        .checked_mul(sym_entsize)
                        .ok_or("symbol index overflow")?,
                )
                .ok_or("symbol offset overflow")?;
            if symbol_offset
                .checked_add(24)
                .ok_or("symbol bounds overflow")?
                > sym_end
            {
                return Err(format!(
                    "relocation section {rel_index} has an out-of-range symbol index"
                ));
            }
            let symbol_section = read_u16(bytes, symbol_offset + 6)?;
            let symbol_value = read_u64(bytes, symbol_offset + 8)?;
            if relocation_type != 8 && symbol_section == 0 {
                return Err(format!(
                    "relocation section {rel_index} references an undefined symbol"
                ));
            }
            let place = r_offset.checked_sub(target.sh_addr).ok_or_else(|| {
                format!("relocation place precedes target section in section {rel_index}")
            })?;
            let place = usize::try_from(place).map_err(|_| "relocation place too large")?;
            match relocation_type {
                8 => {
                    // R_X86_64_RELATIVE: the TIR image uses a zero-based load
                    // bias, so the image-relative addend is the resolved value.
                    if symbol_index != 0 {
                        return Err(format!(
                            "R_X86_64_RELATIVE must use symbol zero in section {rel_index}"
                        ));
                    }
                    let value = u64::try_from(i128::from(addend)).map_err(|_| {
                        format!("relative relocation addend is out of range in section {rel_index}")
                    })?;
                    let patch = code
                        .get_mut(place..place.checked_add(8).ok_or("relocation patch overflow")?)
                        .ok_or("relative relocation patch lies outside executable section")?;
                    patch.copy_from_slice(&value.to_le_bytes());
                }
                2 | 4 => {
                    // R_X86_64_PC32 / R_X86_64_PLT32
                    let value =
                        i128::from(symbol_value) + i128::from(addend) - i128::from(r_offset);
                    let value = i32::try_from(value).map_err(|_| {
                        format!("PC-relative relocation overflows i32 in section {rel_index}")
                    })?;
                    let patch = code
                        .get_mut(place..place.checked_add(4).ok_or("relocation patch overflow")?)
                        .ok_or("PC-relative relocation patch lies outside executable section")?;
                    patch.copy_from_slice(&value.to_le_bytes());
                }
                1 => {
                    // R_X86_64_64
                    let value = u64::try_from(i128::from(symbol_value) + i128::from(addend))
                        .map_err(|_| {
                            format!("absolute relocation overflows u64 in section {rel_index}")
                        })?;
                    let patch = code
                        .get_mut(place..place.checked_add(8).ok_or("relocation patch overflow")?)
                        .ok_or("absolute relocation patch lies outside executable section")?;
                    patch.copy_from_slice(&value.to_le_bytes());
                }
                other => {
                    return Err(format!(
                        "unsupported x86-64 relocation type {other} in executable section {target_index}"
                    ));
                }
            }
        }
    }
    Ok(code)
}

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
        let end = start
            .checked_add(size)
            .ok_or_else(|| format!("section {section_index} bounds overflow"))?;
        section
            .sh_addr
            .checked_add(section.sh_size)
            .ok_or_else(|| format!("section {section_index} virtual address overflows"))?;
        let _raw_code = bytes
            .get(start..end)
            .ok_or_else(|| format!("section {section_index} lies outside the file"))?;
        let code = relocate_executable_section(bytes, &elf, section_index)?;
        executable_sections += 1;

        let mut decoder = Decoder::with_ip(64, &code, section.sh_addr, DecoderOptions::NONE);
        while decoder.can_decode() {
            let instruction = decoder.decode();
            let ip = instruction.ip();
            if instruction.is_invalid() {
                return Err(format!(
                    "invalid x86-64 instruction encoding at 0x{ip:016x}"
                ));
            }
            let op = match instruction.mnemonic() {
                Mnemonic::Nop => "NOP".to_string(),
                Mnemonic::Ret => "RET".to_string(),
                Mnemonic::Mov
                    if instruction.op_count() == 2
                        && instruction.op_kind(0) == iced_x86::OpKind::Register
                        && instruction.op_kind(1) == iced_x86::OpKind::Register =>
                {
                    let dst = register_id(instruction.op_register(0))
                        .ok_or_else(|| format!("unsupported MOV destination at 0x{ip:016x}"))?;
                    let src = register_id(instruction.op_register(1))
                        .ok_or_else(|| format!("unsupported MOV source at 0x{ip:016x}"))?;
                    format!("MOV r{dst} r{src}")
                }
                Mnemonic::Add | Mnemonic::Sub | Mnemonic::Cmp
                    if instruction.op_count() == 2
                        && instruction.op_kind(0) == iced_x86::OpKind::Register
                        && instruction.op_kind(1) == iced_x86::OpKind::Register =>
                {
                    let dst = register_id(instruction.op_register(0)).ok_or_else(|| {
                        format!("unsupported arithmetic destination at 0x{ip:016x}")
                    })?;
                    let src = register_id(instruction.op_register(1))
                        .ok_or_else(|| format!("unsupported arithmetic source at 0x{ip:016x}"))?;
                    format!("{:?} r{dst} r{src}", instruction.mnemonic()).to_ascii_uppercase()
                }
                Mnemonic::Jmp
                | Mnemonic::Je
                | Mnemonic::Jne
                | Mnemonic::Jl
                | Mnemonic::Jle
                | Mnemonic::Jg
                | Mnemonic::Jge
                | Mnemonic::Call => {
                    if instruction.op_kind(0) != iced_x86::OpKind::NearBranch64 {
                        return Err(format!(
                            "indirect control flow is unsupported at 0x{ip:016x}"
                        ));
                    }
                    let mnemonic = match instruction.mnemonic() {
                        Mnemonic::Jmp => "JMP",
                        Mnemonic::Je => "JZ",
                        Mnemonic::Jne => "JNZ",
                        Mnemonic::Jl => "JL",
                        Mnemonic::Jle => "JLE",
                        Mnemonic::Jg => "JG",
                        Mnemonic::Jge => "JGE",
                        Mnemonic::Call => "CALL",
                        _ => unreachable!(),
                    };
                    format!("{mnemonic} {:016x}", instruction.near_branch_target())
                }
                _ if instruction.code() == Code::Mov_r64_imm64 => {
                    let register = register_id(instruction.op0_register()).ok_or_else(|| {
                        format!("unsupported MOV destination register at 0x{ip:016x}")
                    })?;
                    let immediate = Ternary::from_u128(instruction.immediate64() as u128);
                    format!("MOV_IMM64 r{register} {immediate}")
                }
                _ => {
                    return Err(format!(
                        "unsupported x86-64 instruction {:?} at 0x{ip:016x}",
                        instruction.mnemonic()
                    ));
                }
            };
            writeln!(output, "{ip:016x} {op}").expect("writing to String cannot fail");
            instructions = instructions
                .checked_add(1)
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
    validate_tir(&output).map_err(|error| {
        format!(
            "{error}; generated TIR:\\n{}",
            String::from_utf8_lossy(&output)
        )
    })?;
    Ok(output)
}

/// Validate TIR with the exact parser and limits used by the execution runtime.
pub fn validate_tir(bytes: &[u8]) -> Result<usize, String> {
    trion_tir_runtime::validate_tir(bytes).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_elf(code: &[u8]) -> Vec<u8> {
        let code_offset = 256usize;
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
        bytes[section + 4..section + 8]
            .copy_from_slice(&section_header::SHT_PROGBITS.to_le_bytes());
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
    fn translates_register_arithmetic_and_direct_conditional_branches() {
        let code = [
            0x48, 0xb8, 42, 0, 0, 0, 0, 0, 0, 0, 0x48, 0xb9, 42, 0, 0, 0, 0, 0, 0, 0, 0x48, 0x39,
            0xc8, 0x74, 0x01, 0x90, 0xc3,
        ];
        let tir = translate_elf_to_tir(&minimal_elf(&code)).unwrap();
        let text = std::str::from_utf8(&tir).unwrap();
        assert!(text.contains("CMP r0 r1"));
        assert!(text.contains("JZ 000000000040001a"));
        assert!(text.contains("RET"));
        assert_eq!(validate_tir(&tir).unwrap(), 6);
    }

    #[test]
    fn imports_thin_x86_64_macho_executable_section() {
        let mut bytes = vec![0u8; 185];
        bytes[0..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        bytes[4..8].copy_from_slice(&0x0100_0007u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&2u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
        bytes[20..24].copy_from_slice(&152u32.to_le_bytes());
        let command = 32usize;
        bytes[command..command + 4].copy_from_slice(&0x19u32.to_le_bytes());
        bytes[command + 4..command + 8].copy_from_slice(&152u32.to_le_bytes());
        bytes[command + 24..command + 32].copy_from_slice(&0x1000u64.to_le_bytes());
        bytes[command + 32..command + 40].copy_from_slice(&1u64.to_le_bytes());
        bytes[command + 48..command + 56].copy_from_slice(&1u64.to_le_bytes());
        bytes[command + 60..command + 64].copy_from_slice(&5u32.to_le_bytes());
        bytes[command + 64..command + 68].copy_from_slice(&1u32.to_le_bytes());
        let section = command + 72;
        bytes[section + 32..section + 40].copy_from_slice(&0x1000u64.to_le_bytes());
        bytes[section + 40..section + 48].copy_from_slice(&1u64.to_le_bytes());
        bytes[section + 48..section + 52].copy_from_slice(&184u32.to_le_bytes());
        bytes[184] = 0xc3;
        let tir = translate_macho_to_tir(&bytes).unwrap();
        assert_eq!(
            std::str::from_utf8(&tir).unwrap(),
            "TRION-TIR 1 x86_64\n0000000000001000 RET\n"
        );
    }

    #[test]
    fn imports_minimal_x86_64_pe_executable_section() {
        let mut bytes = vec![0u8; 0x201];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        let coff = 0x84usize;
        bytes[coff..coff + 2].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[coff + 2..coff + 4].copy_from_slice(&1u16.to_le_bytes());
        bytes[coff + 16..coff + 18].copy_from_slice(&240u16.to_le_bytes());
        bytes[coff + 18..coff + 20].copy_from_slice(&0x0022u16.to_le_bytes());
        let optional = coff + 20;
        bytes[optional..optional + 2].copy_from_slice(&0x20bu16.to_le_bytes());
        bytes[optional + 24..optional + 32].copy_from_slice(&0x1400_0000_0u64.to_le_bytes());
        bytes[optional + 32..optional + 36].copy_from_slice(&0x1000u32.to_le_bytes());
        bytes[optional + 36..optional + 40].copy_from_slice(&0x200u32.to_le_bytes());
        bytes[optional + 56..optional + 60].copy_from_slice(&0x2000u32.to_le_bytes());
        bytes[optional + 60..optional + 64].copy_from_slice(&0x200u32.to_le_bytes());
        let section = optional + 240;
        bytes[section..section + 5].copy_from_slice(b".text");
        bytes[section + 8..section + 12].copy_from_slice(&1u32.to_le_bytes());
        bytes[section + 12..section + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        bytes[section + 16..section + 20].copy_from_slice(&1u32.to_le_bytes());
        bytes[section + 20..section + 24].copy_from_slice(&0x200u32.to_le_bytes());
        bytes[section + 36..section + 40].copy_from_slice(&0x6000_0020u32.to_le_bytes());
        bytes[0x200] = 0xc3;
        let tir = translate_pe_to_tir(&bytes).unwrap();
        assert!(std::str::from_utf8(&tir).unwrap().contains("RET"));
    }

    #[test]
    fn rejects_bad_architecture_malformed_elf_and_unsupported_instructions() {
        assert!(
            translate_elf_to_tir(b"not an ELF")
                .unwrap_err()
                .contains("not a recognized")
        );
        let mut wrong_arch = minimal_elf(&[0xc3]);
        wrong_arch[18..20].copy_from_slice(&header::EM_386.to_le_bytes());
        assert!(
            translate_elf_to_tir(&wrong_arch)
                .unwrap_err()
                .contains("expected x86-64")
        );
        assert!(
            translate_elf_to_tir(&minimal_elf(&[0xcc]))
                .unwrap_err()
                .contains("unsupported")
        );
        assert!(
            translate_elf_to_tir(&minimal_elf(&[0x48]))
                .unwrap_err()
                .contains("invalid")
        );
    }

    #[test]
    fn applies_internal_elf_pc32_relocation_before_decoding() {
        let mut elf = vec![0u8; 392];
        elf[..4].copy_from_slice(b"\x7fELF");
        elf[4] = 2;
        elf[5] = 1;
        elf[6] = 1;
        elf[16..18].copy_from_slice(&2u16.to_le_bytes());
        elf[18..20].copy_from_slice(&header::EM_X86_64.to_le_bytes());
        elf[20..24].copy_from_slice(&1u32.to_le_bytes());
        elf[24..32].copy_from_slice(&0x400000u64.to_le_bytes());
        elf[40..48].copy_from_slice(&64u64.to_le_bytes());
        elf[52..54].copy_from_slice(&64u16.to_le_bytes());
        elf[58..60].copy_from_slice(&64u16.to_le_bytes());
        elf[60..62].copy_from_slice(&4u16.to_le_bytes());

        let text = 128usize;
        elf[text + 4..text + 8].copy_from_slice(&section_header::SHT_PROGBITS.to_le_bytes());
        elf[text + 8..text + 16].copy_from_slice(
            &((section_header::SHF_ALLOC | section_header::SHF_EXECINSTR) as u64).to_le_bytes(),
        );
        elf[text + 16..text + 24].copy_from_slice(&0x400000u64.to_le_bytes());
        elf[text + 24..text + 32].copy_from_slice(&384u64.to_le_bytes());
        elf[text + 32..text + 40].copy_from_slice(&6u64.to_le_bytes());
        elf[text + 48..text + 56].copy_from_slice(&1u64.to_le_bytes());

        let rela = 192usize;
        elf[rela + 4..rela + 8].copy_from_slice(&section_header::SHT_RELA.to_le_bytes());
        elf[rela + 24..rela + 32].copy_from_slice(&320u64.to_le_bytes());
        elf[rela + 32..rela + 40].copy_from_slice(&24u64.to_le_bytes());
        elf[rela + 40..rela + 44].copy_from_slice(&3u32.to_le_bytes());
        elf[rela + 44..rela + 48].copy_from_slice(&1u32.to_le_bytes());
        elf[rela + 56..rela + 64].copy_from_slice(&24u64.to_le_bytes());

        let symtab = 256usize;
        elf[symtab + 4..symtab + 8].copy_from_slice(&2u32.to_le_bytes());
        elf[symtab + 24..symtab + 32].copy_from_slice(&344u64.to_le_bytes());
        elf[symtab + 32..symtab + 40].copy_from_slice(&48u64.to_le_bytes());
        elf[symtab + 56..symtab + 64].copy_from_slice(&24u64.to_le_bytes());

        elf[320..328].copy_from_slice(&0x400001u64.to_le_bytes());
        elf[328..336].copy_from_slice(&((1u64 << 32) | 2).to_le_bytes());
        elf[336..344].copy_from_slice(&(-4i64).to_le_bytes());
        elf[368 + 6..368 + 8].copy_from_slice(&1u16.to_le_bytes());
        elf[368 + 8..368 + 16].copy_from_slice(&0x400005u64.to_le_bytes());
        elf[384..390].copy_from_slice(&[0xe9, 0, 0, 0, 0, 0xc3]);

        let tir = translate_elf_to_tir(&elf).unwrap();
        let text = std::str::from_utf8(&tir).unwrap();
        assert!(text.contains("JMP 0000000000400005"));
        assert!(text.contains("0000000000400005 RET"));
    }

    #[test]
    fn rejects_elf_with_relocation_sections() {
        let mut elf = minimal_elf(&[0xc3]);
        elf[60..62].copy_from_slice(&3u16.to_le_bytes());
        let relocation_section = 192;
        elf[relocation_section + 4..relocation_section + 8]
            .copy_from_slice(&section_header::SHT_RELA.to_le_bytes());
        elf[relocation_section + 32..relocation_section + 40].copy_from_slice(&24u64.to_le_bytes());
        elf[relocation_section + 40..relocation_section + 44].copy_from_slice(&0u32.to_le_bytes());
        elf[relocation_section + 44..relocation_section + 48].copy_from_slice(&1u32.to_le_bytes());
        elf[relocation_section + 56..relocation_section + 64].copy_from_slice(&24u64.to_le_bytes());
        assert!(
            translate_elf_to_tir(&elf)
                .unwrap_err()
                .contains("symbol table")
        );
    }

    #[test]
    fn rejects_noncanonical_or_out_of_range_tir() {
        assert!(validate_tir(b"TRION-TIR 2 x86_64\n0000000000000000 NOP\n").is_err());
        assert!(validate_tir(b"TRION-TIR 1 x86_64\n0000000000000000 MOV_IMM64 r0 T\n").is_err());
        assert!(validate_tir(b"TRION-TIR 1 x86_64\n0000000000000000 MOV_IMM64 r16 1\n").is_err());
        assert!(validate_tir(b"TRION-TIR 1 x86_64\n0000000000000000 NOP").is_err());
    }
}
