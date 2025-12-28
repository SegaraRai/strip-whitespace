//! wasm-bindgen exports.
//!
//! This module exposes the whitespace stripper to JavaScript via `wasm-bindgen`.
//! The underlying logic lives in `strip.rs`.

use wasm_bindgen::prelude::*;

use strip_whitespace::strip::{
    StripConfig as StripConfigInner, strip_astro_whitespace, strip_astro_whitespace_no_sourcemap,
};

/// Configuration options for whitespace stripping.
#[derive(Debug, Clone, Copy, Default, serde::Deserialize, tsify::Tsify)]
#[tsify(from_wasm_abi)]
pub struct StripConfig {
    /// Preserve blank-line gaps ("\n\n" / "\r\n\r\n") between nodes
    pub preserve_blank_lines: bool,
}

impl From<StripConfig> for StripConfigInner {
    fn from(val: StripConfig) -> Self {
        StripConfigInner {
            preserve_blank_lines: val.preserve_blank_lines,
        }
    }
}

/// Output from the wasm API when a sourcemap is requested.
#[derive(Debug, Clone, serde::Serialize, tsify::Tsify)]
#[tsify(into_wasm_abi)]
pub struct StripOutput {
    /// The rewritten Astro source.
    pub code: String,
    /// The generated/re-written sourcemap JSON.
    pub sourcemap: String,
}

/// Strip inter-node whitespace and create a brand-new sourcemap.
///
/// `source_name` is recorded as the sourcemap's source filename.
#[wasm_bindgen]
pub fn strip_whitespace(
    code: String,
    source_name: String,
    config: StripConfig,
) -> Result<StripOutput, JsValue> {
    console_error_panic_hook::set_once();

    let res = strip_astro_whitespace(&code, &source_name, &config.into())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(StripOutput {
        code: res.code,
        sourcemap: res.sourcemap,
    })
}

/// Strip inter-node whitespace without producing a sourcemap.
///
/// `preserve_blank_lines` skips rewriting whitespace gaps that contain an empty line.
#[wasm_bindgen]
pub fn strip_whitespace_no_sourcemap(code: String, config: StripConfig) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();

    let res = strip_astro_whitespace_no_sourcemap(&code, &config.into())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(res)
}
