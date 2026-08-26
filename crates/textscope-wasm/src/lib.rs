#![forbid(unsafe_code)]
//! Thin WebAssembly interoperability boundary for TextScope.
//!
//! This crate may adapt the platform-independent core for browser consumers,
//! but domain rules must remain in `textscope-core`.

mod contract;

use wasm_bindgen::prelude::*;

/// Compares the plain-text clipboard representation with extracted HTML text
/// using the deterministic rule owned by the Rust core.
#[wasm_bindgen]
pub fn compare_clipboard_representations(
    plain_text: Option<String>,
    html_text: Option<String>,
) -> Result<String, JsValue> {
    contract::compare_clipboard_representations_json(plain_text.as_deref(), html_text.as_deref())
        .map_err(|_| JsValue::from_str("TextScope could not serialize the clipboard comparison."))
}

/// Scans text through the authoritative core and returns its JSON contract.
///
/// The returned string uses `snake_case` field names and must be parsed once
/// by the JavaScript consumer. Serialization errors become JavaScript
/// exceptions with a content-free technical message.
#[wasm_bindgen]
pub fn scan_text(text: &str) -> Result<String, JsValue> {
    contract::scan_text_json(text)
        .map_err(|_| JsValue::from_str("TextScope could not serialize the scan result."))
}

/// Reveals technical characters through the authoritative core implementation.
#[must_use]
#[wasm_bindgen]
pub fn reveal_text(text: &str) -> String {
    textscope_core::reveal_text(text)
}

/// Applies a supported TCS profile through the authoritative core.
///
/// The profile is a stable identifier returned by [`available_clean_profiles`].
/// Errors use fixed messages and never include the analyzed text.
#[wasm_bindgen]
pub fn clean_text(text: &str, profile: &str) -> Result<String, JsValue> {
    let profile = textscope_core::CleanProfile::from_id(profile)
        .ok_or_else(|| JsValue::from_str("TextScope received an unknown clean profile."))?;

    contract::clean_text_json(text, profile)
        .map_err(|_| JsValue::from_str("TextScope could not complete the clean operation."))
}

/// Returns supported and reserved clean-profile metadata as JSON.
#[wasm_bindgen]
pub fn available_clean_profiles() -> Result<String, JsValue> {
    contract::clean_profiles_json()
        .map_err(|_| JsValue::from_str("TextScope could not serialize clean profiles."))
}

/// Returns the formal TextScope Clean Standard version.
#[must_use]
#[wasm_bindgen]
pub fn tcs_version() -> String {
    textscope_core::tcs_version().to_owned()
}

/// Strictly analyzes local source bytes through the authoritative Rust core.
///
/// `bytes` crosses the generated boundary as a `Uint8Array`; it is never
/// Base64-encoded or represented as a JSON byte array. Domain failures are
/// returned as a structured JSON response without echoing source content.
#[wasm_bindgen]
pub fn analyze_text_bytes(bytes: &[u8], source_kind: &str) -> Result<String, JsValue> {
    contract::analyze_text_bytes_json(bytes, source_kind)
        .map_err(|_| JsValue::from_str("TextScope could not serialize the source analysis."))
}

/// Forensically analyzes untrusted HTML bytes through the authoritative Core.
///
/// The HTML is never rendered, executed, or used to resolve external
/// resources. `bytes` crosses the generated boundary as a `Uint8Array`.
#[wasm_bindgen]
pub fn analyze_html_bytes(bytes: &[u8]) -> Result<String, JsValue> {
    contract::analyze_html_bytes_json(bytes)
        .map_err(|_| JsValue::from_str("TextScope could not serialize the HTML analysis."))
}

/// Forensically analyzes an untrusted DOCX package through the authoritative Core.
///
/// The package stays in memory: TextScope does not extract it to the filesystem,
/// execute embedded content, or resolve internal or external relationships.
/// `bytes` crosses the generated boundary as a `Uint8Array`.
#[wasm_bindgen]
pub fn analyze_docx_bytes(bytes: &[u8]) -> Result<String, JsValue> {
    contract::analyze_docx_bytes_json(bytes)
        .map_err(|_| JsValue::from_str("TextScope could not serialize the DOCX analysis."))
}

/// Forensically analyzes an untrusted PDF snapshot through the authoritative
/// Rust Core. The PDF is never rendered, executed, fetched, extracted to disk,
/// or rewritten; only bounded structural and textual observations cross JSON.
#[wasm_bindgen]
pub fn analyze_pdf_bytes(bytes: &[u8]) -> Result<String, JsValue> {
    contract::analyze_pdf_bytes_json(bytes)
        .map_err(|_| JsValue::from_str("TextScope could not serialize the PDF analysis."))
}

#[cfg(test)]
mod tests;
