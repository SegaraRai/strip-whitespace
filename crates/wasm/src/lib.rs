//! wasm-bindgen exports.
//!
//! This module exposes the whitespace stripper to JavaScript via `wasm-bindgen`.
//! The underlying logic lives in the [`strip_whitespace`] crate.

use wasm_bindgen::prelude::*;

use strip_whitespace::{
    Language as LanguageInner,
    strip::{
        CodeAndSourcemap, StripConfig as StripConfigInner, strip_whitespace,
        strip_whitespace_no_sourcemap,
    },
};

/// Supported template languages for whitespace stripping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, tsify::Tsify)]
#[tsify(from_wasm_abi)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Astro,
    Svelte,
}

impl From<Language> for LanguageInner {
    fn from(value: Language) -> Self {
        match value {
            Language::Astro => LanguageInner::Astro,
            Language::Svelte => LanguageInner::Svelte,
        }
    }
}

/// Configuration options for whitespace stripping.
#[derive(Debug, Clone, Copy, Default, serde::Deserialize, tsify::Tsify)]
#[tsify(from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct StripConfig {
    /// Preserve blank-line gaps ("\n\n" / "\r\n\r\n") between nodes
    pub preserve_blank_lines: bool,
}

impl From<StripConfig> for StripConfigInner {
    fn from(value: StripConfig) -> Self {
        StripConfigInner {
            preserve_blank_lines: value.preserve_blank_lines,
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
    pub map: String,
}

impl From<CodeAndSourcemap> for StripOutput {
    fn from(value: CodeAndSourcemap) -> Self {
        StripOutput {
            code: value.code,
            map: value.map,
        }
    }
}

/// Strip inter-node whitespace and create a brand-new sourcemap.
///
/// `source_name` is recorded as the sourcemap's source filename.
/// `language` specifies the template language (e.g., "astro" or "svelte").
#[wasm_bindgen(js_name = "stripWhitespace")]
pub fn wasm_strip_whitespace(
    code: String,
    source_name: String,
    language: Language,
    config: StripConfig,
) -> Result<StripOutput, JsValue> {
    console_error_panic_hook::set_once();

    let output = strip_whitespace(&code, &source_name, language.into(), &config.into())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(output.into())
}

/// Strip inter-node whitespace without producing a sourcemap.
///
/// `language` specifies the template language (e.g., "astro" or "svelte").
#[wasm_bindgen(js_name = "stripWhitespaceNoSourcemap")]
pub fn wasm_strip_whitespace_no_sourcemap(
    code: String,
    language: Language,
    config: StripConfig,
) -> Result<String, JsValue> {
    console_error_panic_hook::set_once();

    let output = strip_whitespace_no_sourcemap(&code, language.into(), &config.into())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(output)
}
