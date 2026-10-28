use std::{fs, process::Command};

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
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a recognized object file"));
}
