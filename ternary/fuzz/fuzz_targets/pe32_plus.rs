#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = trion_executable::pe::inspect(data);
    let _ = trion_executable::pe::map_preferred(data);
});
