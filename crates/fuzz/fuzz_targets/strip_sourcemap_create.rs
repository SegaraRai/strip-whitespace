#![no_main]

use astro_strip_whitespace::strip::{StripConfig, strip_astro_whitespace_sourcemap_create};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let data = if data.len() > 256 * 1024 {
        &data[..256 * 1024]
    } else {
        data
    };

    let source = String::from_utf8_lossy(data);

    for preserve_blank_lines in [false, true] {
        let config = StripConfig {
            preserve_blank_lines,
        };

        if let Ok(out) = strip_astro_whitespace_sourcemap_create(&source, "fuzz.astro", &config) {
            // If creation succeeds, the sourcemap must be parseable JSON.
            // Any panic here is a bug we want the fuzzer to catch.
            let _ = serde_json::from_str::<serde_json::Value>(&out.sourcemap)
                .expect("sourcemap must be valid JSON when create() returns Ok");
        }
    }
});
