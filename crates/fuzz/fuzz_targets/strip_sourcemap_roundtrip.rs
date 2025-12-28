#![no_main]

use astro_strip_whitespace::strip::{
    StripConfig,
    strip_astro_whitespace_sourcemap_create,
    strip_astro_whitespace_sourcemap_rewrite,
};
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

        // Create a sourcemap, then try rewriting it.
        // Parse errors are expected outcomes and should be reported as Err, not crashes.
        if let Ok(created) = strip_astro_whitespace_sourcemap_create(&source, "fuzz.astro", &config) {
            let _ = strip_astro_whitespace_sourcemap_rewrite(&source, &created.sourcemap, &config);

            // Also try a corrupted map to exercise rewrite error handling.
            // (Rewrite should return Err, but must never crash.)
            let corrupted = if created.sourcemap.len() > 2 {
                created.sourcemap[..(created.sourcemap.len() / 2)].to_string()
            } else {
                "{".to_string()
            };
            let _ = strip_astro_whitespace_sourcemap_rewrite(&source, &corrupted, &config);
        }
    }
});
