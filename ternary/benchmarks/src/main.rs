//! Reproducible microbenchmark harness for TRION Phase 2 representations.
//! Results are diagnostic only; run on a quiet, identified host and retain raw output.

use std::hint::black_box;
use std::time::Instant;
use trion_ternary::Ternary;
use trion_ternary_memory::PackedTritBuffer;

const TRIT_COUNT: usize = 100_000;
const ITERATIONS: usize = 10;

fn elapsed_ns(start: Instant) -> u128 {
    start.elapsed().as_nanos()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let values: Vec<i8> = (0..TRIT_COUNT)
        .map(|index| match index % 3 {
            0 => -1,
            1 => 0,
            _ => 1,
        })
        .collect();

    let start = Instant::now();
    let mut binary_copy_bytes = 0usize;
    for _ in 0..ITERATIONS {
        let copy = black_box(values.clone());
        binary_copy_bytes = black_box(copy.len());
    }
    let binary_copy_ns = elapsed_ns(start);

    let start = Instant::now();
    let mut packed_bytes = 0usize;
    for _ in 0..ITERATIONS {
        let packed = PackedTritBuffer::from_balanced_trits(black_box(&values))?;
        packed_bytes = black_box(packed.packed_byte_len());
    }
    let pack_ns = elapsed_ns(start);

    let packed = PackedTritBuffer::from_balanced_trits(&values)?;
    let start = Instant::now();
    let mut decoded_count = 0usize;
    for _ in 0..ITERATIONS {
        let decoded = packed.to_balanced_trits()?;
        decoded_count = black_box(decoded.len());
    }
    let unpack_ns = elapsed_ns(start);

    let integer_samples: Vec<i128> = (0..10_000).map(|n| n as i128 - 5_000).collect();
    let start = Instant::now();
    let mut ternary_digits = 0usize;
    for _ in 0..ITERATIONS {
        for value in &integer_samples {
            let encoded = Ternary::from_i128(black_box(*value));
            ternary_digits = black_box(ternary_digits.wrapping_add(encoded.trits_lsf().len()));
            black_box(encoded.to_i128()?);
        }
    }
    let conversion_ns = elapsed_ns(start);

    println!("TRION Phase 2 representation microbenchmarks");
    println!("host_arch={}", std::env::consts::ARCH);
    println!("iterations={ITERATIONS}");
    println!("logical_trits={TRIT_COUNT}");
    println!("binary_i8_bytes={binary_copy_bytes}");
    println!("packed_backing_bytes={packed_bytes}");
    println!(
        "packed_storage_ratio={:.4}",
        packed_bytes as f64 / binary_copy_bytes.max(1) as f64
    );
    println!("binary_copy_total_ns={binary_copy_ns}");
    println!("balanced_pack_total_ns={pack_ns}");
    println!("balanced_unpack_total_ns={unpack_ns}");
    println!("i128_balanced_conversion_total_ns={conversion_ns}");
    println!("decoded_trits={decoded_count}");
    println!("conversion_digit_accumulator={ternary_digits}");
    println!("warning=single-host microbenchmark; not a general performance claim");
    Ok(())
}
