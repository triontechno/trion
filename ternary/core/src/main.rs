use std::env;
use std::process::ExitCode;
use trion_ternary::{
    Ternary, balanced_to_u3, pack_u3_u32, pack_u3_u64, u3_to_balanced, unpack_u3_u32,
    unpack_u3_u64,
};

fn usage() {
    eprintln!("Trion balanced-ternary tool");
    eprintln!("Usage:");
    eprintln!("  trion-ternary from-decimal <integer>");
    eprintln!("  trion-ternary to-decimal <balanced-ternary>");
    eprintln!("  trion-ternary add <a> <b>");
    eprintln!("  trion-ternary sub <a> <b>");
    eprintln!("  trion-ternary mul <a> <b>");
    eprintln!("  trion-ternary pack-u32 <trit-list>");
    eprintln!("  trion-ternary unpack-u32 <packed-value> <count>");
    eprintln!("  trion-ternary pack-u64 <trit-list>");
    eprintln!("  trion-ternary unpack-u64 <packed-value> <count>");
    eprintln!("  trion-ternary u3-to-balanced <digit>");
    eprintln!("  trion-ternary balanced-to-u3 <digit>");
}

fn parse_u3_trits(input: &str) -> Result<Vec<u8>, String> {
    let mut trits = Vec::new();
    let normalized = input.trim();
    if normalized.is_empty() {
        return Err("empty trit list".to_string());
    }

    for raw in normalized.split(',') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }

        if token.len() == 1 && token.chars().all(|ch| ch.is_ascii_digit()) {
            let value = token.parse::<u8>().map_err(|e| format!("invalid trit {token:?}: {e}"))?;
            if value > 2 {
                return Err(format!("trit value {value} is outside the unbalanced domain {{0,1,2}}"));
            }
            trits.push(value);
            continue;
        }

        for ch in token.chars() {
            let value = match ch {
                '0' => 0,
                '1' => 1,
                '2' => 2,
                other => return Err(format!("invalid trit char '{other}' in '{input}'")),
            };
            trits.push(value);
        }
    }

    Ok(trits)
}

fn format_u3_trits(trits: &[u8]) -> String {
    trits
        .iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn format_balanced_trit(value: i8) -> String {
    match value {
        -1 => "T".to_string(),
        0 => "0".to_string(),
        1 => "1".to_string(),
        other => format!("invalid balanced trit: {other}"),
    }
}

fn run(args: &[String]) -> Result<String, String> {
    match args {
        [cmd, value] if cmd == "from-decimal" => {
            let n = value.parse::<i128>().map_err(|e| format!("invalid i128 integer: {e}"))?;
            Ok(Ternary::from_i128(n).to_string())
        }
        [cmd, value] if cmd == "to-decimal" => {
            Ternary::parse(value)
                .map_err(|e| e.to_string())?
                .to_i128()
                .map(|v| v.to_string())
                .map_err(|e| e.to_string())
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
        [cmd, trits] if cmd == "pack-u32" => {
            let values = parse_u3_trits(trits)?;
            let packed = pack_u3_u32(&values).map_err(|e| e.to_string())?;
            Ok(packed.to_string())
        }
        [cmd, value, count] if cmd == "unpack-u32" => {
            let packed = value.parse::<u32>().map_err(|e| format!("invalid u32 value: {e}"))?;
            let count = count.parse::<usize>().map_err(|e| format!("invalid count: {e}"))?;
            let decoded = unpack_u3_u32(packed, count).map_err(|e| e.to_string())?;
            Ok(format_u3_trits(&decoded))
        }
        [cmd, trits] if cmd == "pack-u64" => {
            let values = parse_u3_trits(trits)?;
            let packed = pack_u3_u64(&values).map_err(|e| e.to_string())?;
            Ok(packed.to_string())
        }
        [cmd, value, count] if cmd == "unpack-u64" => {
            let packed = value.parse::<u64>().map_err(|e| format!("invalid u64 value: {e}"))?;
            let count = count.parse::<usize>().map_err(|e| format!("invalid count: {e}"))?;
            let decoded = unpack_u3_u64(packed, count).map_err(|e| e.to_string())?;
            Ok(format_u3_trits(&decoded))
        }
        [cmd, digit] if cmd == "u3-to-balanced" => {
            let trit = digit.parse::<u8>().map_err(|e| format!("invalid unbalanced trit: {e}"))?;
            let balanced = u3_to_balanced(trit).map_err(|e| e.to_string())?;
            Ok(format_balanced_trit(balanced))
        }
        [cmd, digit] if cmd == "balanced-to-u3" => {
            let trit = digit
                .trim()
                .parse::<i8>()
                .map_err(|e| format!("invalid balanced trit: {e}"))?;
            let unbalanced = balanced_to_u3(trit).map_err(|e| e.to_string())?;
            Ok(unbalanced.to_string())
        }
        _ => Err("invalid command or arguments".into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Error: {error}");
            usage();
            ExitCode::FAILURE
        }
    }
}
