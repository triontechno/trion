//! Comparative binary, balanced-ternary, and hybrid microbenchmarks.
//!
//! This harness only produces measurements when explicitly run. It reports
//! raw samples and workload metadata; it never asserts a performance winner.

use std::hint::black_box;
use std::process::Command;
use std::time::Instant;
use trion_ternary::Ternary;
use trion_ternary_memory::PackedTritBuffer;

const VALUES: usize = 20_000;
const SAMPLES: usize = 7;

fn measure(
    mut workload: impl FnMut() -> Result<i128, Box<dyn std::error::Error>>,
) -> Result<(Vec<u128>, i128), Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(SAMPLES);
    let mut expected_result = None;
    for _ in 0..SAMPLES {
        let start = Instant::now();
        let result = black_box(workload()?);
        samples.push(start.elapsed().as_nanos());
        if let Some(expected) = expected_result {
            if result != expected {
                return Err("benchmark candidates produced different results".into());
            }
        } else {
            expected_result = Some(result);
        }
    }
    Ok((samples, expected_result.unwrap_or(0)))
}

fn median(samples: &[u128]) -> u128 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

fn host_cpu() -> String {
    if let Ok(identifier) = std::env::var("PROCESSOR_IDENTIFIER") {
        return identifier;
    }
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        if let Some(model) = cpuinfo.lines().find_map(|line| {
            line.strip_prefix("model name")
                .and_then(|value| value.split_once(':').map(|(_, value)| value.trim()))
        }) {
            return model.to_owned();
        }
    }
    if let Ok(output) = Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
    {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout).trim().to_owned();
        }
    }
    "unavailable; record host CPU manually".to_owned()
}

fn rustc_version() -> String {
    Command::new("rustc")
        .arg("-Vv")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).replace('\n', ";"))
        .unwrap_or_else(|| "unavailable; record rustc -Vv manually".to_owned())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inputs: Vec<i128> = (0..VALUES)
        .map(|index| (index as i128 % 2_001) - 1_000)
        .collect();
    let encoded_trits: Vec<i8> = inputs
        .iter()
        .flat_map(|value| Ternary::from_i128(*value).trits_lsf().to_vec())
        .collect();

    let binary = measure(|| {
        let mut total = 0i128;
        for value in &inputs {
            total = total
                .checked_add(black_box(*value))
                .ok_or("binary accumulator overflow")?;
        }
        Ok(total)
    })?;

    let ternary = measure(|| {
        let mut total = Ternary::zero();
        for value in &inputs {
            let encoded = Ternary::from_i128(black_box(*value));
            total = total.try_add(&encoded)?;
        }
        Ok(total.to_i128()?)
    })?;

    // Hybrid path: use ternary as an internal value representation and
    // convert explicitly at the conventional binary accumulation boundary.
    let hybrid = measure(|| {
        let mut total = 0i128;
        for value in &inputs {
            // Ternary internal representation with an explicit binary boundary
            // conversion before the conventional accumulator.
            let internal = Ternary::from_i128(black_box(*value));
            let boundary_value = internal.to_i128()?;
            total = total
                .checked_add(black_box(boundary_value))
                .ok_or("hybrid accumulator overflow")?;
        }
        Ok(total)
    })?;

    let packed_roundtrip = measure(|| {
        let packed = PackedTritBuffer::from_balanced_trits(black_box(&encoded_trits))?;
        let decoded = packed.to_balanced_trits()?;
        Ok(decoded.iter().map(|digit| i128::from(*digit)).sum())
    })?;

    let packed = PackedTritBuffer::from_balanced_trits(&encoded_trits)?;
    let binary_storage_bytes = inputs.len() * std::mem::size_of::<i128>();
    let logical_i8_storage_bytes = encoded_trits.len() * std::mem::size_of::<i8>();
    let packed_storage_bytes = packed.packed_byte_len();

    println!("TRION_PHASE2_BENCHMARK 2");
    println!("host_arch={}", std::env::consts::ARCH);
    println!("host_os={}", std::env::consts::OS);
    println!("host_cpu={}", host_cpu());
    println!("pointer_width_bits={}", usize::BITS);
    println!("rustc_version={}", rustc_version());
    println!("workload=deterministic_integer_sum_and_balanced_trit_roundtrip");
    println!("input_count={VALUES}");
    println!("samples={SAMPLES}");
    println!("binary_i128_storage_bytes={binary_storage_bytes}");
    println!("balanced_i8_storage_bytes={logical_i8_storage_bytes}");
    println!("packed_balanced_storage_bytes={packed_storage_bytes}");
    println!(
        "packed_vs_i8_ratio={:.6}",
        packed_storage_bytes as f64 / logical_i8_storage_bytes.max(1) as f64
    );
    println!(
        "packed_vs_i128_ratio={:.6}",
        packed_storage_bytes as f64 / binary_storage_bytes.max(1) as f64
    );
    let (binary_samples, binary_result) = binary;
    let (ternary_samples, ternary_result) = ternary;
    let (hybrid_samples, hybrid_result) = hybrid;
    let (packed_samples, packed_result) = packed_roundtrip;
    if binary_result != ternary_result || binary_result != hybrid_result {
        return Err("binary, ternary, and hybrid semantic results differ".into());
    }
    println!("binary_samples_ns={}", binary_samples.iter().map(u128::to_string).collect::<Vec<_>>().join(","));
    println!("ternary_samples_ns={}", ternary_samples.iter().map(u128::to_string).collect::<Vec<_>>().join(","));
    println!("hybrid_samples_ns={}", hybrid_samples.iter().map(u128::to_string).collect::<Vec<_>>().join(","));
    println!("packed_roundtrip_samples_ns={}", packed_samples.iter().map(u128::to_string).collect::<Vec<_>>().join(","));
    println!("binary_median_ns={}", median(&binary_samples));
    println!("ternary_median_ns={}", median(&ternary_samples));
    println!("hybrid_median_ns={}", median(&hybrid_samples));
    println!("packed_roundtrip_median_ns={}", median(&packed_samples));
    println!("result_binary={binary_result}");
    println!("result_ternary={ternary_result}");
    println!("result_hybrid={hybrid_result}");
    println!("packed_roundtrip_checksum={packed_result}");
    println!("note=single-host diagnostic only; compare identical toolchains and quiet hosts");
    Ok(())
}
