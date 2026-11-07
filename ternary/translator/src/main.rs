use goblin::elf::{header, section_header};
use iced_x86::{Decoder, DecoderOptions, Formatter, IntelFormatter};
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
};
use trion_tir_runtime::{ExecutionOutcome, MachineState, execute_tir};

mod tir;

fn usage() {
    eprintln!("Trion executable inspection, TIR translator and TRIONEX runtime");
    eprintln!("Usage:");
    eprintln!("  trion-translate inspect <ELF|PE .exe|Mach-O|macOS .app>");
    eprintln!("  trion-translate translate <ELF|PE .exe|Mach-O|macOS .app> <new-output.tir>");
    eprintln!("  trion-translate import <ELF|PE .exe|Mach-O|macOS .app> <new-output.trx>");
    eprintln!("  trion-translate package <validated-input.tir> <new-output.trx>");
    eprintln!("  trion-translate run <input.trx> [max-instructions]");
}

fn app_executable(path: &Path) -> Result<PathBuf, String> {
    let contents = path.join("Contents");
    let macos = contents.join("MacOS");
    if !contents.is_dir() || !macos.is_dir() {
        return Err("invalid .app bundle: expected Contents/MacOS".into());
    }
    let mut candidates = fs::read_dir(&macos)
        .map_err(|e| format!("cannot read app bundle executable directory: {e}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| candidate.is_file())
        .collect::<Vec<_>>();
    candidates.sort();
    match candidates.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err("app bundle contains no regular executable in Contents/MacOS".into()),
        _ => Err("app bundle has multiple executables; select the intended Contents/MacOS binary explicitly".into()),
    }
}

fn input_bytes(path: &str) -> Result<(PathBuf, Vec<u8>, bool), String> {
    let supplied = PathBuf::from(path);
    let is_app = supplied.is_dir()
        || supplied
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("app"));
    let actual = if is_app {
        app_executable(&supplied)?
    } else {
        supplied
    };
    let bytes = fs::read(&actual).map_err(|e| format!("cannot read {}: {e}", actual.display()))?;
    Ok((actual, bytes, is_app))
}

fn inspect(path: &str) -> Result<(), String> {
    let (actual, bytes, is_app) = input_bytes(path)?;
    let object = goblin::Object::parse(&bytes)
        .map_err(|e| format!("not a recognized executable/object file: {e}"))?;
    println!("TRION executable format inspection");
    println!("Input: {path}");
    if is_app {
        println!("Bundle executable: {}", actual.display());
        println!("Bundle type: macOS .app");
    }
    match object {
        goblin::Object::Elf(elf) => {
            if elf.header.e_machine != header::EM_X86_64 || !elf.is_64 {
                return Err(format!(
                    "unsupported ELF architecture: expected x86-64, machine={}, 64-bit={}",
                    elf.header.e_machine, elf.is_64
                ));
            }
            println!("Format: ELF64");
            println!("Architecture: x86-64");
            println!("Entry point: 0x{:x}", elf.entry);
            println!("Sections: {}", elf.section_headers.len());
            inspect_elf_code(&bytes, &elf)?;
        }
        goblin::Object::PE(pe) => {
            println!("Format: PE/COFF (Windows executable)");
            println!(
                "Architecture machine: 0x{:04x}",
                pe.header.coff_header.machine
            );
            println!("64-bit optional header: {}", pe.is_64);
            println!("Sections: {}", pe.sections.len());
            println!(
                "Note: PE parsing/inspection is supported; Windows API emulation and execution are not implemented."
            );
        }
        goblin::Object::Mach(_) => {
            println!("Format: Mach-O (Apple executable)");
            println!(
                "Note: Mach-O parsing/recognition is supported; macOS framework, loader and system-call compatibility are not implemented."
            );
        }
        goblin::Object::Archive(_) => {
            return Err("static archives are not executable inputs".into());
        }
        goblin::Object::Unknown(magic) => {
            return Err(format!("unsupported executable magic: {magic:?}"));
        }
        _ => return Err("unsupported executable object format".into()),
    }
    Ok(())
}

fn inspect_elf_code(bytes: &[u8], elf: &goblin::elf::Elf<'_>) -> Result<(), String> {
    let mut decoded = 0usize;
    let mut candidate_mnemonics = 0usize;
    let mut noncandidate_mnemonics = 0usize;
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
        let mut decoder = Decoder::with_ip(64, data, section.sh_addr, DecoderOptions::NONE);
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
            if candidate {
                candidate_mnemonics += 1;
            } else {
                noncandidate_mnemonics += 1;
            }
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
    println!("Candidate mnemonics (triage only): {candidate_mnemonics}");
    println!("Non-candidate mnemonics (backend absent): {noncandidate_mnemonics}");
    println!("Invalid instruction encodings: {invalid}");
    println!("Instruction frequencies:");
    for (mnemonic, count) in counts {
        println!("  {mnemonic:<16} {count}");
    }
    println!("Examples needing attention (first 12):");
    for example in unsupported_examples {
        println!("  {example}");
    }
    println!(
        "Candidate mnemonic classification is not translation support; no output binary was generated."
    );
    Ok(())
}

fn translate(path: &str, output_path: &str) -> Result<(), String> {
    let (actual, bytes, is_app) = input_bytes(path)?;
    let output = tir::translate_object_to_tir(&bytes)?;
    if is_app {
        println!("Imported macOS application bundle executable: {}", actual.display());
    }
    let count = tir::validate_tir(&output)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)
        .map_err(|error| format!("cannot create {output_path} without overwriting: {error}"))?;
    if let Err(error) = file.write_all(&output) {
        drop(file);
        return match fs::remove_file(output_path) {
            Ok(()) => Err(format!(
                "cannot write translated output {output_path}: {error}"
            )),
            Err(cleanup_error) => Err(format!(
                "cannot write translated output {output_path}: {error}; also cannot remove partial output: {cleanup_error}"
            )),
        };
    }
    println!("Translated {count} instructions to validated TIR v1: {output_path}");
    println!("Output is an intermediate representation, not an executable binary.");
    Ok(())
}

fn import(path: &str, output_path: &str) -> Result<(), String> {
    let (actual, bytes, is_app) = input_bytes(path)?;
    let tir_bytes = tir::translate_object_to_tir(&bytes)?;
    let executable = trion_executable::TrionExecutable::from_tir(&tir_bytes)
        .map_err(|error| format!("translated program failed TRIONEX validation: {error}"))?;
    let container = executable.to_bytes()
        .map_err(|error| format!("cannot encode TRIONEX container: {error}"))?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)
        .map_err(|error| format!("cannot create {output_path} without overwriting: {error}"))?;
    if let Err(error) = file.write_all(&container) {
        drop(file);
        return match fs::remove_file(output_path) {
            Ok(()) => Err(format!("cannot write {output_path}: {error}")),
            Err(cleanup_error) => Err(format!("cannot write {output_path}: {error}; also cannot remove partial output: {cleanup_error}")),
        };
    }
    println!("Imported {} executable instructions into TRIONEX v1: {output_path}", tir::validate_tir(&tir_bytes)?);
    println!("Source: {}{}", actual.display(), if is_app { " (macOS .app bundle)" } else { "" });
    println!("Only the translated x86-64 instruction subset is supported; OS APIs and platform ABIs are not emulated.");
    Ok(())
}

fn package(input_path: &str, output_path: &str) -> Result<(), String> {
    let payload =
        fs::read(input_path).map_err(|error| format!("cannot read {input_path}: {error}"))?;
    let executable = trion_executable::TrionExecutable::from_tir(&payload)
        .map_err(|error| format!("input is not valid canonical TIR v1: {error}"))?;
    let bytes = executable
        .to_bytes()
        .map_err(|error| format!("cannot encode TRIONEX container: {error}"))?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)
        .map_err(|error| format!("cannot create {output_path} without overwriting: {error}"))?;
    if let Err(error) = file.write_all(&bytes) {
        drop(file);
        return match fs::remove_file(output_path) {
            Ok(()) => Err(format!("cannot write {output_path}: {error}")),
            Err(cleanup_error) => Err(format!(
                "cannot write {output_path}: {error}; also cannot remove partial output: {cleanup_error}"
            )),
        };
    }
    println!(
        "Packaged {} validated TIR bytes into TRIONEX v1: {output_path}",
        payload.len()
    );
    println!("TRIONEX is an interpreted TIR container, not native machine code.");
    Ok(())
}

fn run(path: &str, max_instructions: usize) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let executable = trion_executable::TrionExecutable::parse(&bytes)
        .map_err(|e| format!("invalid or unsupported TRIONEX executable: {e}"))?;
    let memory = [0u8; 8];
    let mut registers = [0u64; 16];
    registers[4] = 0x1000;
    let mut state = MachineState {
        registers,
        memory_base: 0x1000,
        memory: &memory,
    };
    let outcome = execute_tir(executable.payload(), &mut state, max_instructions)
        .map_err(|e| format!("TRIONEX execution failed: {e}"))?;
    match outcome {
        ExecutionOutcome::Completed {
            instructions_executed,
        } => println!(
            "TRIONEX completed: {instructions_executed} instructions; r0={}",
            state.registers[0]
        ),
        ExecutionOutcome::Returned {
            target,
            instructions_executed,
        } => println!(
            "TRIONEX returned to 0x{target:016x} after {instructions_executed} instructions"
        ),
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.as_slice() {
        [command, path] if command == "inspect" => inspect(path),
        [command, input, output] if command == "translate" => translate(input, output),
        [command, input, output] if command == "package" => package(input, output),
        [command, input, output] if command == "import" => import(input, output),
        [command, input] if command == "run" => run(input, 65_536),
        [command, input, limit] if command == "run" => match limit.parse::<usize>() {
            Ok(limit) if limit > 0 => run(input, limit),
            _ => Err("max-instructions must be a positive integer".into()),
        },
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
