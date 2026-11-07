#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = trion_executable::select_app_executable(data, &["Trion", "Runner", "Application"]);
});
