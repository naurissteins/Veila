#![no_main]

use libfuzzer_sys::fuzz_target;
use veila_common::AppConfig;

fuzz_target!(|data: &[u8]| {
    if data.len() > 64 * 1024 {
        return;
    }
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let _ = AppConfig::from_toml_str(input);
});
