use std::path::PathBuf;

use strip_whitespace::{
    Language,
    strip::{StripConfig, strip_whitespace, strip_whitespace_no_sourcemap},
};

fn fixtures_dir() -> PathBuf {
    #[cfg(target_os = "wasi")]
    {
        PathBuf::from("/workspace/fixtures")
    }

    #[cfg(not(target_os = "wasi"))]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
    }
}

fn read_fixture(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(name)).expect("read fixture")
}

fn detect_language(filename: &str) -> Language {
    if filename.ends_with(".astro") {
        Language::Astro
    } else if filename.ends_with(".svelte") {
        Language::Svelte
    } else {
        panic!("unknown file extension: {}", filename)
    }
}

#[test]
fn fixtures_match_expected_output() {
    let cases = [
        ("complex.astro", "complex.out.astro"),
        ("simple.astro", "simple.out.astro"),
        ("components.astro", "components.out.astro"),
        ("whitespace.astro", "whitespace.out.astro"),
        ("unicode.astro", "unicode.out.astro"),
        ("complex.svelte", "complex.out.svelte"),
        ("simple.svelte", "simple.out.svelte"),
        ("components.svelte", "components.out.svelte"),
        ("whitespace.svelte", "whitespace.out.svelte"),
        ("unicode.svelte", "unicode.out.svelte"),
    ];

    for (input_name, expected_name) in cases {
        let input = read_fixture(input_name);
        let expected = read_fixture(expected_name);
        let language = detect_language(input_name);

        let actual =
            strip_whitespace_no_sourcemap(&input, language, &StripConfig::default()).unwrap();
        assert_eq!(actual, expected, "fixture mismatch: {input_name}");

        // Idempotence: once stripped, stripping again should not change.
        let actual2 =
            strip_whitespace_no_sourcemap(&actual, language, &StripConfig::default()).unwrap();
        assert_eq!(actual2, actual, "not idempotent: {input_name}");
    }
}

#[test]
fn fixtures_emit_parseable_sourcemaps() {
    let cases = [
        ("complex.astro", "complex.out.astro"),
        ("simple.astro", "simple.out.astro"),
        ("components.astro", "components.out.astro"),
        ("whitespace.astro", "whitespace.out.astro"),
        ("unicode.astro", "unicode.out.astro"),
        ("complex.svelte", "complex.out.svelte"),
        ("simple.svelte", "simple.out.svelte"),
        ("components.svelte", "components.out.svelte"),
        ("whitespace.svelte", "whitespace.out.svelte"),
        ("unicode.svelte", "unicode.out.svelte"),
    ];

    for (input_name, expected_name) in cases {
        let input = read_fixture(input_name);
        let expected = read_fixture(expected_name);
        let language = detect_language(input_name);

        // Source filename is the fixture filename; tests assert this gets recorded.
        let res = strip_whitespace(&input, input_name, language, &StripConfig::default()).unwrap();
        assert_eq!(res.code, expected, "code mismatch: {input_name}");

        let map = sourcemap::SourceMap::from_slice(res.map.as_bytes())
            .expect("sourcemap JSON must parse");
        assert_eq!(map.get_source(0), Some(input_name));
    }
}
