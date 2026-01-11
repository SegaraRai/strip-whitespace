#![no_main]

use libfuzzer_sys::fuzz_target;
use strip_whitespace::{Language, StripConfig, strip_whitespace};

fuzz_target!(|data: &[u8]| {
    let data = if data.len() > 256 * 1024 {
        &data[..256 * 1024]
    } else {
        data
    };

    let source = String::from_utf8_lossy(data);

    for &language in &[Language::Astro, Language::Svelte] {
        for config in &[
            StripConfig {
                preserve_blank_lines: false,
            },
            StripConfig {
                preserve_blank_lines: true,
            },
        ] {
            let filename = match language {
                Language::Astro => "input.astro",
                Language::Svelte => "input.svelte",
            };

            if let Ok(out) = strip_whitespace(&source, filename, language, config) {
                // If creation succeeds, the sourcemap must be parseable JSON.
                // Any panic here is a bug we want the fuzzer to catch.
                let _ = serde_json::from_str::<serde_json::Value>(&out.map)
                    .expect("sourcemap must be valid JSON when create() returns Ok");
            }
        }
    }
});
