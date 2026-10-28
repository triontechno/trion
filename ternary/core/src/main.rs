use std::env;
use std::process::ExitCode;
use trion_ternary::Ternary;

fn usage() {
    eprintln!("Trion balanced-ternary tool");
    eprintln!("Usage:");
    eprintln!("  trion-ternary from-decimal <integer>");
    eprintln!("  trion-ternary to-decimal <balanced-ternary>");
    eprintln!("  trion-ternary add <a> <b>");
    eprintln!("  trion-ternary sub <a> <b>");
    eprintln!("  trion-ternary mul <a> <b>");
}

fn run(args: &[String]) -> Result<String, String> {
    match args {
        [cmd, value] if cmd == "from-decimal" => {
            let n = value.parse::<i128>().map_err(|e| format!("invalid i128 integer: {e}"))?;
            Ok(Ternary::from_i128(n).to_string())
        }
        [cmd, value] if cmd == "to-decimal" => {
            Ternary::parse(value).map_err(|e| e.to_string())?
                .to_i128().map(|v| v.to_string()).map_err(|e| e.to_string())
        }
        [cmd, a, b] if cmd == "add" || cmd == "sub" || cmd == "mul" => {
            let a = Ternary::parse(a).map_err(|e| e.to_string())?;
            let b = Ternary::parse(b).map_err(|e| e.to_string())?;
            let result = match cmd.as_str() {
                "add" => a.add(&b),
                "sub" => a.sub(&b),
                _ => a.mul(&b),
            };
            Ok(format!("{result} (balanced ternary)"))
        }
        _ => Err("invalid command or arguments".into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(output) => { println!("{output}"); ExitCode::SUCCESS }
        Err(error) => { eprintln!("Error: {error}"); usage(); ExitCode::FAILURE }
    }
}
