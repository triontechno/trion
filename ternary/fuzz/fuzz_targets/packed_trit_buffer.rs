#![no_main]

use libfuzzer_sys::fuzz_target;
use trion_ternary_memory::PackedTritBuffer;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }
    let mut length_bytes = [0u8; 8];
    length_bytes.copy_from_slice(&data[..8]);
    let logical_len = u64::from_le_bytes(length_bytes);
    let Ok(logical_len) = usize::try_from(logical_len) else {
        return;
    };
    // Bound attacker-controlled allocation while still exploring partial-byte,
    // empty, exact-capacity, and multi-byte lengths.
    if logical_len > 1_000_000 {
        return;
    }
    if let Ok(buffer) = PackedTritBuffer::from_packed_bytes(logical_len, &data[8..]) {
        let expanded = buffer.to_balanced_trits().expect("validated buffer must decode");
        let reencoded = PackedTritBuffer::from_balanced_trits(&expanded)
            .expect("decoded trits must re-encode");
        assert_eq!(reencoded, buffer);
    }
});
