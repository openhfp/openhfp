//! WASM bindings for the HFP **read side** — the one engine, now callable from JS.
//!
//! Exposes `canonicalizeSha256`, `extract` and `validate` so the browser dev shim (and any
//! web SDK) share `hfp-core`'s exact canonicalization and schema validation instead of a
//! JS re-implementation. Signing/verification stay native (they need keys / are the
//! security boundary), so this build pulls no CMS/RSA/getrandom dependencies.

use wasm_bindgen::prelude::*;

/// SHA-256 (lowercase hex) of the canonical bytes — the value signatures are computed over.
#[wasm_bindgen(js_name = canonicalizeSha256)]
pub fn canonicalize_sha256(html: &str) -> Result<String, JsError> {
    hfp_core::canonical_sha256_hex(html.as_bytes()).map_err(to_js)
}

/// The embedded `#hfp-data` JSON (trimmed). Errors if missing or not valid JSON.
#[wasm_bindgen]
pub fn extract(html: &str) -> Result<String, JsError> {
    hfp_core::extract(html.as_bytes()).map_err(to_js)
}

/// Validate `#hfp-data` against `#hfp-schema`. Returns a JSON string
/// `{ "valid": bool, "errors": [{ "path", "message" }] }` (same shape as `ValidationResult`).
#[wasm_bindgen]
pub fn validate(html: &str) -> Result<String, JsError> {
    let report = hfp_core::validate(html.as_bytes()).map_err(to_js)?;
    Ok(report_to_json(&report))
}

/// Validate already-extracted `data` JSON against `schema` JSON. Returns the same JSON shape
/// as [`validate`]. This is what the dev shim wires in (it holds the parsed blocks).
#[wasm_bindgen(js_name = validateData)]
pub fn validate_data(schema_json: &str, data_json: &str) -> Result<String, JsError> {
    let report = hfp_core::validate_values(schema_json, data_json).map_err(to_js)?;
    Ok(report_to_json(&report))
}

fn report_to_json(report: &hfp_core::ValidationReport) -> String {
    let errors: Vec<serde_json::Value> = report
        .errors
        .iter()
        .map(|e| serde_json::json!({ "path": e.path, "message": e.message }))
        .collect();
    serde_json::json!({ "valid": report.valid, "errors": errors }).to_string()
}

fn to_js(e: hfp_core::Error) -> JsError {
    JsError::new(&e.to_string())
}
