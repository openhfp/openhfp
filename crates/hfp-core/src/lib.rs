//! `hfp-core` — reference engine for the HFP (HTML Form Package) format.
//!
//! This crate is the single source of truth for the format's machine operations:
//! parsing, canonicalization, data extraction, schema validation and signature
//! verification. The same code is intended to run natively (CLI, Filler) and as a
//! WASM module (browser dev shim, web SDK). Only signing — which needs access to a
//! private key or OS keystore — lives outside this crate's portable surface.
//!
//! Status: pre-alpha scaffold. The public types describe the intended API; the
//! implementations are filled in during Phase 1.1 (see the repository roadmap).

#![forbid(unsafe_code)]

use std::fmt;

mod canon;
mod schema;
#[cfg(feature = "crypto")]
mod sign;
#[cfg(feature = "crypto")]
mod verify;
pub use canon::canonical_sha256_hex;
#[cfg(feature = "crypto")]
pub use sign::SigningIdentity;

/// Errors returned by the core engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input was not valid UTF-8.
    InvalidUtf8,
    /// A required block (for example `#hfp-data`) was missing.
    MissingBlock(&'static str),
    /// A block id that must be unique appeared more than once.
    DuplicateBlock(&'static str),
    /// A required `<meta>` element (for example `hfp-id`) was missing.
    MissingMeta(&'static str),
    /// A block that must contain JSON (for example `#hfp-data`) did not parse.
    InvalidJson(&'static str),
    /// A cryptographic operation failed (key/cert parsing, CMS build).
    Crypto(&'static str),
    /// The operation is part of the API surface but is not implemented yet.
    NotImplemented(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidUtf8 => write!(f, "input is not valid UTF-8"),
            Error::MissingBlock(b) => write!(f, "required block is missing: {b}"),
            Error::DuplicateBlock(b) => write!(f, "duplicate block id: {b}"),
            Error::MissingMeta(m) => write!(f, "required meta is missing: {m}"),
            Error::InvalidJson(b) => write!(f, "block is not valid JSON: {b}"),
            Error::Crypto(what) => write!(f, "cryptographic operation failed: {what}"),
            Error::NotImplemented(what) => write!(f, "not implemented yet: {what}"),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for core operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Trust policy shared by the CLI and the Filler.
///
/// Without a config this behaves as "dev mode" (use the OS trust store). With a
/// config, only the whitelisted CAs are accepted (enterprise mode).
#[cfg(feature = "crypto")]
#[derive(Debug, Clone, Default)]
pub struct TrustConfig {
    /// DER-encoded CA certificates that act as trust anchors (chains must terminate at
    /// one of these). The Filler/CLI loads these from policy; the portable core never
    /// touches the OS trust store. Empty means "no anchor configured" -> untrusted.
    pub trust_anchors: Vec<Vec<u8>>,
    /// SHA-256 thumbprints (lowercase hex of the DER) of the CAs allowed to issue author
    /// and filler certificates.
    pub allowed_ca_thumbprints: Vec<String>,
    /// When true, the anchor a chain terminates at must also be in
    /// `allowed_ca_thumbprints`.
    pub require_from_allowed_ca: bool,
    /// DER-encoded CRLs used for revocation. Supplied by the host (the portable core
    /// does no network I/O); in an air-gapped Filler this is empty.
    pub crls: Vec<Vec<u8>>,
    /// Skip CRL/OCSP revocation checks (archival verification of older documents).
    pub no_revocation_check: bool,
}

/// A single schema validation problem, addressed by JSON path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// JSON path to the offending value, e.g. `faults[0]` or `device`.
    pub path: String,
    /// Human-readable message (a UX hint, not a security boundary).
    pub message: String,
}

/// Result of validating `#hfp-data` against `#hfp-schema`.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    /// True when there are no validation errors.
    pub valid: bool,
    /// Every problem found (empty when `valid`).
    pub errors: Vec<ValidationError>,
}

/// Outcome of verifying a `.hfp` document.
#[cfg(feature = "crypto")]
#[derive(Debug, Clone, Default)]
pub struct VerifyReport {
    /// The author's signature over the canonical document is valid.
    pub author_signature_valid: bool,
    /// The filler's signature over the canonical data is valid.
    pub data_signature_valid: bool,
    /// Every signing certificate chains to a trusted CA per the [`TrustConfig`].
    pub is_trusted: bool,
    /// Human-readable notes (distinguished name, thumbprint, reason for failure).
    pub notes: Vec<String>,
}

/// Produce the canonical byte representation that signatures are computed over.
///
/// The canonical form empties the inner content of the `#hfp-data` and
/// `#hfp-data-signature` blocks (keeping their tags), normalizes line endings to LF
/// and requires valid UTF-8. It hard-fails on duplicate or missing required blocks.
pub fn canonicalize(raw: &[u8]) -> Result<Vec<u8>> {
    canon::canonicalize(raw)
}

/// Extract the embedded `#hfp-data` JSON as a string (the inner content, trimmed).
/// Errors if the block is missing or does not contain valid JSON.
pub fn extract(raw: &[u8]) -> Result<String> {
    schema::extract(raw)
}

/// Validate the embedded `#hfp-data` against the `#hfp-schema` (a JSON Schema subset:
/// `type`, `required`, `properties`, `items`, `enum`, `pattern`). Structural problems
/// (missing block, invalid JSON) are `Err`; schema violations populate the report.
pub fn validate(raw: &[u8]) -> Result<ValidationReport> {
    schema::validate(raw)
}

/// Validate already-extracted `data` JSON against `schema` JSON (same subset as
/// [`validate`]). For hosts that already hold the parsed blocks (e.g. the WASM dev shim).
pub fn validate_values(schema_json: &str, data_json: &str) -> Result<ValidationReport> {
    schema::validate_values(schema_json, data_json)
}

/// The bytes the data signature is computed over: the canonical data bound to the
/// `hfp-id` and a hash of the author signature. See [`verify`] and spike-b-findings.md.
#[cfg(feature = "crypto")]
pub fn data_signing_payload(raw: &[u8]) -> Result<Vec<u8>> {
    verify::data_signing_payload(raw)
}

/// Canonical bytes the author signature is computed over (data, data-signature and
/// author-signature blocks emptied).
#[cfg(feature = "crypto")]
pub fn canonical_author_bytes(raw: &[u8]) -> Result<Vec<u8>> {
    canon::canonical_author_bytes(raw)
}

/// Lowercase hex SHA-256 of arbitrary bytes (e.g. canonical bytes already in hand).
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Verify the author and data signatures against the given trust policy.
#[cfg(feature = "crypto")]
pub fn verify(raw: &[u8], trust: &TrustConfig) -> Result<VerifyReport> {
    verify::verify(raw, trust)
}

/// Produce a fully signed `.hfp`: the author signs the canonical document, then the filler
/// signs the bound data payload. Signing lives here as the reference implementation; in
/// production the Filler supplies key access (OS keystore).
#[cfg(feature = "crypto")]
pub fn sign(raw: &[u8], author: &SigningIdentity, filler: &SigningIdentity) -> Result<Vec<u8>> {
    sign::sign(raw, author, filler)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages_are_stable() {
        assert_eq!(
            Error::MissingBlock("hfp-data").to_string(),
            "required block is missing: hfp-data"
        );
        assert_eq!(
            Error::NotImplemented("verify").to_string(),
            "not implemented yet: verify"
        );
    }

    #[test]
    fn trust_config_defaults_to_dev_mode() {
        let t = TrustConfig::default();
        assert!(t.allowed_ca_thumbprints.is_empty());
        assert!(!t.require_from_allowed_ca);
        assert!(!t.no_revocation_check);
    }

    #[test]
    fn implemented_operations_are_wired_up() {
        // canonicalize (A), verify (B), extract + validate (1.1) all parse the document;
        // empty input has no required blocks, so each hard-fails structurally.
        assert_eq!(canonicalize(b""), Err(Error::MissingBlock("hfp-data")));
        assert_eq!(extract(b"").unwrap_err(), Error::MissingBlock("hfp-data"));
        assert_eq!(validate(b"").unwrap_err(), Error::MissingBlock("hfp-data"));
        assert_eq!(
            verify(b"", &TrustConfig::default()).unwrap_err(),
            Error::MissingBlock("hfp-data")
        );
    }
}
