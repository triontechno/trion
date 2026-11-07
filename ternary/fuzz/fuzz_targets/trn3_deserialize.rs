#![no_main]

use libfuzzer_sys::fuzz_target;
use trion_ternary::{deserialize_balanced, serialize_balanced};

fuzz_target!(|data: &[u8]| {
    if let Ok(value) = deserialize_balanced(data) {
        let canonical = serialize_balanced(&value).expect("decoded value must serialize");
        let decoded = deserialize_balanced(&canonical).expect("canonical output must decode");
        assert_eq!(decoded, value);
    }
});
