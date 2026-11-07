//! Comparative binary, balanced-ternary, and hybrid microbenchmarks.
//!
//! This harness only produces measurements when explicitly run. It reports
//! raw samples and workload metadata; it never asserts a performance winner.

use std::hint::black_box;
use std::time::Instant;
use trion_ternary::Ternary;
use trion_ternary_memory::PackedTritBuffer;

const VALUES: usize = 20_000;
const SAMPLES: usize = 7;

fn measure(mut workload: impl FnMut() -> Result<i128, Box<dyn std::error::Error>>) -> Result<Vec<u128>, Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        black_box(workload()?);
        samples.push(start.elapsed().as_nanos());
    }
    Ok(samples)
}

fn median(samples: &[u128]) -> u128 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inputs: Vec<i128> = (0..VALUES)
        .map(|index| (index as i128 % 2_001) - 1_000)
        .collect();
    let trits: Vec<i8> = inputs.iter().map(|value| match value.rem_euclid(3) {
        0 => -1,
        1 => 0,
        _ => 1,
    }).collect();

    let binary = measure(|| {
        let mut total = 0i128;
        for value in &inputs {
            total = total.checked_add(black_box(*value))
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

    // Hybrid path: pack logical balanced trits into conventional bytes,
    // unpack at the binary boundary, then perform the external accumulation.
    let hybrid = measure(|| {
        let packed = PackedTritBuffer::from_balanced_trits(black_box(&trits))?;
        let decoded = packed.to_balanced_trits()?;
        let mut total = 0i128;
        for digit in decoded {
            total = total.checked_add(i128::from(black_box(digit)))
                .ok_or("hybrid accumulator overflow")?;
        }
        Ok(total)
    })?;

    let packed = PackedTritBuffer::from_balanced_trits(&trits)?;
    let binary_storage_bytes = inputs.len() * std::mem::size_of::<i128>();
    let logical_i8_storage_bytes = trits.len() * std::mem::size_of::<i8>();
    let packed_storage_bytes = packed.packed_byte_len();

    println!("TRION_PHASE2_BENCHMARK 2");
    println!("host_arch={}", std::env::consts::ARCH);
    println!("host_os={}", std::env::consts::OS);
    println!("pointer_width_bits={}", usize::BITS);
    println!("rustc_version=record_with_rustc_-Vv");
    println!("workload=deterministic_integer_sum_and_balanced_trit_roundtrip");
    println!("input_count={VALUES}");
    println!("samples={SAMPLES}");
    println!("binary_i128_storage_bytes={binary_storage_bytes}");
    println!("balanced_i8_storage_bytes={logical_i8_storage_bytes}");
    println!("packed_balanced_storage_bytes={packed_storage_bytes}");
    println!("packed_vs_i8_ratio={:.6}", packed_storage_bytes as f64 / logical_i8_storage_bytes.max(1) as f64);
    println!("packed_vs_i128_ratio={:.6}", packed_storage_bytes as f64 / binary_storage_bytes.max(1) as f64);
    println!("binary_samples_ns={}", binary.iter().map(u128::to_string).collect::<Vec<_>>().join(","));
    println!("ternary_samples_ns={}", ternary.iter().map(u128::to_string).collect::<Vec<_>>().join(","));
    println!("hybrid_samples_ns={}", hybrid.iter().map(u128::to_string).collect::<Vec<_>>().join(","));
    println!("binary_median_ns={}", median(&binary));
    println!("ternary_median_ns={}", median(&ternary));
    println!("hybrid_median_ns={}", median(&hybrid));
    println!("result_binary={}", inputs.iter().sum::<i128>());
    println!("result_ternary={}", inputs.iter().sum::<i128>());
    println!("result_hybrid={}", trits.iter().map(|value| i128::from(*value)).sum::<i128>());
    println!("note=single-host diagnostic only; compare identical toolchains and quiet hosts");
    Ok(())
}
