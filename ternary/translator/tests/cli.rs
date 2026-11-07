use std::{fs, process::Command};

fn minimal_x86_64_elf(code: &[u8]) -> Vec<u8> {
    let code_offset = 192usize;
    let mut bytes = vec![0u8; code_offset + code.len()];
    bytes[..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[6] = 1;
    bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&62u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&0x400000u64.to_le_bytes());
    bytes[40..48].copy_from_slice(&64u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
    bytes[58..60].copy_from_slice(&64u16.to_le_bytes());
    bytes[60..62].copy_from_slice(&2u16.to_le_bytes());
    let section = 128usize;
    bytes[section + 4..section + 8].copy_from_slice(&1u32.to_le_bytes());
    bytes[section + 8..section + 16].copy_from_slice(&6u64.to_le_bytes());
    bytes[section + 16..section + 24].copy_from_slice(&0x400000u64.to_le_bytes());
    bytes[section + 24..section + 32].copy_from_slice(&(code_offset as u64).to_le_bytes());
    bytes[section + 32..section + 40].copy_from_slice(&(code.len() as u64).to_le_bytes());
    bytes[section + 48..section + 56].copy_from_slice(&1u64.to_le_bytes());
    bytes[code_offset..].copy_from_slice(code);
    bytes
}

#[test]
fn cli_prints_usage_without_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_trion-translate"))
        .output()
        .expect("run CLI");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("inspect"));
}

#[test]
fn rejects_non_elf_input() {
    let path = std::env::temp_dir().join(format!("trion-not-elf-{}.bin", std::process::id()));
    fs::write(&path, b"not an ELF executable").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_trion-translate"))
        .arg("inspect")
        .arg(&path)
        .output()
        .expect("run CLI");
    let _ = fs::remove_file(&path);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("error:")
    );
}

#[test]
fn translate_cli_writes_valid_tir_without_overwriting_existing_output() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!("trion-translate-{}-{nonce}", std::process::id()));
    let input = base.with_extension("elf");
    let output_path = base.with_extension("tir");
    fs::write(&input, minimal_x86_64_elf(&[0x90, 0xc3])).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_trion-translate"))
        .arg("translate")
        .arg(&input)
        .arg(&output_path)
        .output()
        .expect("run translator");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(&output_path).unwrap(),
        "TRION-TIR 1 x86_64\n0000000000400000 NOP\n0000000000400001 RET\n"
    );

    fs::write(&output_path, b"preserve").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_trion-translate"))
        .arg("translate")
        .arg(&input)
        .arg(&output_path)
        .output()
        .expect("run translator against existing output");
    assert!(!output.status.success());
    assert_eq!(fs::read(&output_path).unwrap(), b"preserve");
    fs::remove_file(input).unwrap();
    fs::remove_file(output_path).unwrap();
}

#[test]
fn package_cli_wraps_validated_tir_and_refuses_overwrite() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!("trion-package-{}-{nonce}", std::process::id()));
    let input = base.with_extension("tir");
    let output_path = base.with_extension("trx");
    let tir = b"TRION-TIR 1 x86_64\n0000000000400000 NOP\n";
    fs::write(&input, tir).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_trion-translate"))
        .arg("package")
        .arg(&input)
        .arg(&output_path)
        .output()
        .expect("run TRIONEX packager");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let container = fs::read(&output_path).unwrap();
    assert_eq!(&container[..8], b"TRIONEX\0");
    assert_eq!(u16::from_le_bytes([container[8], container[9]]), 1);
    assert_eq!(&container[36..], tir);

    fs::write(&output_path, b"preserve").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_trion-translate"))
        .arg("package")
        .arg(&input)
        .arg(&output_path)
        .output()
        .expect("run packager against existing output");
    assert!(!output.status.success());
    assert_eq!(fs::read(&output_path).unwrap(), b"preserve");

    fs::remove_file(input).unwrap();
    fs::remove_file(output_path).unwrap();
}

#[test]
fn run_cli_executes_validated_trionex_with_instruction_budget() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!("trion-run-{}-{nonce}", std::process::id()));
    let input = base.with_extension("tir");
    let executable = base.with_extension("trx");
    fs::write(
        &input,
        b"TRION-TIR 1 x86_64\n0000000000400000 MOV_IMM64 r0 1TTT0\n",
    )
    .unwrap();

    let packaged = Command::new(env!("CARGO_BIN_EXE_trion-translate"))
        .arg("package")
        .arg(&input)
        .arg(&executable)
        .output()
        .expect("package TRIONEX");
    assert!(
        packaged.status.success(),
        "{}",
        String::from_utf8_lossy(&packaged.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_trion-translate"))
        .arg("run")
        .arg(&executable)
        .output()
        .expect("run TRIONEX");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("r0=42"));

    let limited = Command::new(env!("CARGO_BIN_EXE_trion-translate"))
        .arg("run")
        .arg(&executable)
        .arg("1")
        .output()
        .expect("run TRIONEX with budget");
    assert!(
        limited.status.success(),
        "{}",
        String::from_utf8_lossy(&limited.stderr)
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(executable).unwrap();
}
