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
pub use canon::canonical_sha256_hex;

/// Errors returned by the core engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input was not valid UTF-8.
    InvalidUtf8,
    /// A required block (for example `#hfp-data`) was missing.
    MissingBlock(&'static str),
    /// A block id that must be unique appeared more than once.
    DuplicateBlock(&'static str),
    /// The operation is part of the API surface but is not implemented yet.
    NotImplemented(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidUtf8 => write!(f, "input is not valid UTF-8"),
            Error::MissingBlock(b) => write!(f, "required block is missing: {b}"),
            Error::DuplicateBlock(b) => write!(f, "duplicate block id: {b}"),
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
#[derive(Debug, Clone, Default)]
pub struct TrustConfig {
    /// SHA-256 thumbprints of the certificate authorities trusted to issue author
    /// and filler certificates. Empty means dev mode (OS trust store).
    pub allowed_ca_thumbprints: Vec<String>,
    /// When true, a certificate must chain to one of `allowed_ca_thumbprints`.
    pub require_from_allowed_ca: bool,
    /// Skip CRL/OCSP revocation checks (archival verification of older documents).
    pub no_revocation_check: bool,
}

/// Outcome of verifying a `.hfp` document.
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

/// Extract the embedded `#hfp-data` JSON as a string.
pub fn extract(_raw: &[u8]) -> Result<String> {
    Err(Error::NotImplemented("extract"))
}

/// Validate the embedded data against the `#hfp-schema` (a JSON Schema subset).
pub fn validate(_raw: &[u8]) -> Result<()> {
    Err(Error::NotImplemented("validate"))
}

/// Verify the author and data signatures against the given trust policy.
pub fn verify(_raw: &[u8], _trust: &TrustConfig) -> Result<VerifyReport> {
    Err(Error::NotImplemented("verify"))
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
    fn scaffold_operations_report_not_implemented() {
        // `canonicalize` is implemented as of Spike A; the rest are still scaffold.
        assert_eq!(
            verify(b"", &TrustConfig::default()).unwrap_err(),
            Error::NotImplemented("verify")
        );
        assert_eq!(extract(b"").unwrap_err(), Error::NotImplemented("extract"));
    }

    #[test]
    fn canonicalize_is_wired_up() {
        // Empty input has no required blocks, so it hard-fails (not NotImplemented).
        assert_eq!(canonicalize(b""), Err(Error::MissingBlock("hfp-data")));
    }
}
