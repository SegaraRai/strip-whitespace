//! wasm-bindgen exports.
//!
//! This module exposes the whitespace stripper to JavaScript via `wasm-bindgen`.
//! The underlying logic lives in `strip.rs`.

use wasm_bindgen::prelude::*;

use astro_strip_whitespace::strip::{
    StripConfig, strip_astro_whitespace_no_sourcemap, strip_astro_whitespace_sourcemap_create,
    strip_astro_whitespace_sourcemap_rewrite,
};

/// Output from the wasm API when a sourcemap is requested.
#[wasm_bindgen(getter_with_clone)]
pub struct StripOutput {
    /// The rewritten Astro source.
    pub code: String,
    /// The generated/re-written sourcemap JSON.
    pub sourcemap: String,
}

/// Strip inter-node whitespace without producing a sourcemap.
///
/// `preserve_blank_lines` skips rewriting whitespace gaps that contain an empty line.
#[wasm_bindgen]
pub fn strip_whitespace_no_sourcemap(
    code: String,
    preserve_blank_lines: bool,
) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();

    let cfg = StripConfig {
        preserve_blank_lines,
    };
    let res = strip_astro_whitespace_no_sourcemap(&code, &cfg)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(res)
}

/// Strip inter-node whitespace and create a brand-new sourcemap.
///
/// `source_name` is recorded as the sourcemap's source filename.
#[wasm_bindgen]
pub fn strip_whitespace_sourcemap_create(
    code: String,
    source_name: String,
    preserve_blank_lines: bool,
) -> Result<StripOutput, JsValue> {
    console_error_panic_hook::set_once();

    let cfg = StripConfig {
        preserve_blank_lines,
    };
    let res = strip_astro_whitespace_sourcemap_create(&code, &source_name, &cfg)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(StripOutput {
        code: res.code,
        sourcemap: res.sourcemap,
    })
}

/// Strip inter-node whitespace and rewrite an existing sourcemap.
///
/// `input_map` must be a sourcemap JSON string describing the current `code`.
#[wasm_bindgen]
pub fn strip_whitespace_sourcemap_rewrite(
    code: String,
    input_map: String,
    preserve_blank_lines: bool,
) -> Result<StripOutput, JsValue> {
    console_error_panic_hook::set_once();

    let cfg = StripConfig {
        preserve_blank_lines,
    };
    let res = strip_astro_whitespace_sourcemap_rewrite(&code, &input_map, &cfg)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(StripOutput {
        code: res.code,
        sourcemap: res.sourcemap,
    })
}
