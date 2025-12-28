#![no_main]

use strip_whitespace::strip::{StripConfig, strip_astro_whitespace_no_sourcemap};
use libfuzzer_sys::fuzz_target;

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
    let _ = strip_astro_whitespace_no_sourcemap(
        &source,
        &StripConfig {
            preserve_blank_lines: false,
        },
    );
    let _ = strip_astro_whitespace_no_sourcemap(
        &source,
        &StripConfig {
            preserve_blank_lines: true,
        },
    );
});
