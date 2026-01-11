#![no_main]

use libfuzzer_sys::fuzz_target;
use strip_whitespace::{Language, StripConfig, strip_whitespace_no_sourcemap};

fuzz_target!(|data: &[u8]| {
    // Limit input size to keep the fuzzer fast and avoid OOM in pathological cases.
    let data = if data.len() > 256 * 1024 {
        &data[..256 * 1024]
    } else {
        data
    };

    let source = String::from_utf8_lossy(data);

    // Exercise both configuration modes for every input.
    // Parse errors are expected outcomes and must never crash.
    for &language in &[Language::Astro, Language::Svelte] {
        for config in &[
            StripConfig {
                preserve_blank_lines: false,
            },
            StripConfig {
                preserve_blank_lines: true,
            },
        ] {
            let _ = strip_whitespace_no_sourcemap(&source, language, config);
        }
    }
});
