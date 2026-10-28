use goblin::elf::{header, section_header};
use iced_x86::{Decoder, DecoderOptions, Formatter, IntelFormatter};
use std::{env, fs, process::ExitCode};

fn usage() {
    eprintln!("Trion binary translator — ELF inspection prototype");
    eprintln!("Usage: trion-translate inspect <x86-64-elf>");
}

fn inspect(path: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let elf =
        goblin::Object::parse(&bytes).map_err(|e| format!("not a recognized object file: {e}"))?;
    let elf = match elf {
        goblin::Object::Elf(elf) => elf,
        _ => return Err("input is not an ELF file; this prototype targets ELF".into()),
    };

    if elf.header.e_machine != header::EM_X86_64 {
        return Err(format!(
            "unsupported ELF machine {}; expected x86-64 ({})",
            elf.header.e_machine,
            header::EM_X86_64
        ));
    }
    if !elf.is_64 {
        return Err("expected a 64-bit ELF executable".into());
    }

    println!("Trion binary translator — inspection only");
    println!("File: {path}");
    println!("ELF type: {}", elf.header.e_type);
    println!("Entry point: 0x{:x}", elf.entry);
    println!("Sections: {}", elf.section_headers.len());
    println!("Note: this command does not translate or execute the binary.\n");

    let mut decoded = 0usize;
    let mut invalid = 0usize;
    let mut executable_sections = 0usize;
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    let mut unsupported_examples = Vec::<String>::new();
    let mut formatter = IntelFormatter::new();

    for (index, section) in elf.section_headers.iter().enumerate() {
        if section.sh_flags & (section_header::SHF_EXECINSTR as u64) == 0 || section.sh_size == 0 {
            continue;
        }
        let start = usize::try_from(section.sh_offset)
            .map_err(|_| "section offset too large".to_string())?;
        let size =
            usize::try_from(section.sh_size).map_err(|_| "section size too large".to_string())?;
        let end = start.checked_add(size).ok_or("section bounds overflow")?;
        let data = bytes
            .get(start..end)
            .ok_or_else(|| format!("section {index} lies outside file bounds"))?;
        executable_sections += 1;
        let ip = section.sh_addr;
        let mut decoder = Decoder::with_ip(64, data, ip, DecoderOptions::NONE);
        while decoder.can_decode() {
            let instruction = decoder.decode();
            if instruction.is_invalid() {
                invalid += 1;
                if unsupported_examples.len() < 12 {
                    unsupported_examples
                        .push(format!("0x{:x}: <invalid encoding>", instruction.ip()));
                }
                continue;
            }
            decoded += 1;
            let name = format!("{:?}", instruction.mnemonic());
            *counts.entry(name).or_insert(0) += 1;

            // Candidate first-pass integer/control-flow subset. This is only a
            // triage classification, not a claim that these ops are translated.
            let candidate = matches!(
                instruction.mnemonic(),
                iced_x86::Mnemonic::Nop
                    | iced_x86::Mnemonic::Mov
                    | iced_x86::Mnemonic::Movzx
                    | iced_x86::Mnemonic::Movsx
                    | iced_x86::Mnemonic::Lea
                    | iced_x86::Mnemonic::Add
                    | iced_x86::Mnemonic::Sub
                    | iced_x86::Mnemonic::Imul
                    | iced_x86::Mnemonic::Cmp
                    | iced_x86::Mnemonic::Test
                    | iced_x86::Mnemonic::Jmp
                    | iced_x86::Mnemonic::Je
                    | iced_x86::Mnemonic::Jne
                    | iced_x86::Mnemonic::Jl
                    | iced_x86::Mnemonic::Jle
                    | iced_x86::Mnemonic::Jg
                    | iced_x86::Mnemonic::Jge
                    | iced_x86::Mnemonic::Call
                    | iced_x86::Mnemonic::Ret
                    | iced_x86::Mnemonic::Push
                    | iced_x86::Mnemonic::Pop
                    | iced_x86::Mnemonic::Syscall
            );
            if !candidate && unsupported_examples.len() < 12 {
                let mut text = String::new();
                formatter.format(&instruction, &mut text);
                unsupported_examples.push(format!(
                    "0x{:x}: {text} [backend not implemented]",
                    instruction.ip()
                ));
            }
        }
    }

    println!("Executable sections decoded: {executable_sections}");
    println!("Instructions decoded: {decoded}");
    println!("Invalid instruction encodings: {invalid}");
    println!("\nInstruction frequencies:");
    for (mnemonic, count) in counts {
        println!("  {mnemonic:<16} {count}");
    }
    println!("\nExamples needing attention (first 12):");
    if unsupported_examples.is_empty() {
        println!("  None found by the current triage rules.");
    } else {
        for example in unsupported_examples {
            println!("  {example}");
        }
    }
    println!(
        "\nCandidate mnemonic classification is not translation support; no output binary was generated."
    );
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.as_slice() {
        [command, path] if command == "inspect" => inspect(path),
        _ => {
            usage();
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
