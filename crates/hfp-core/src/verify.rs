//! Signature verification (Spike B) — pure-Rust CMS/PKCS#7 over the canonical bytes.
//!
//! The author signature covers [`crate::canonical_author_bytes`]; the data signature
//! covers [`data_signing_payload`] (canonical data bound to `hfp-id` and a hash of the
//! author signature, so data cannot be replayed into another form). For each we:
//!   1. parse the detached CMS `SignedData`,
//!   2. check the `messageDigest` signed attribute equals SHA-256 of the content,
//!   3. verify the RSA (PKCS#1 v1.5 + SHA-256) signature over the signed attributes,
//!   4. check the signer certificate chains to a configured trust anchor and that the
//!      anchor's SHA-256 thumbprint is whitelisted,
//!   5. check the signer certificate is not on a supplied CRL (unless revocation is off).
//!
//! Signing is intentionally out of scope of the portable core (it needs a private key /
//! OS keystore); fixtures are produced by OpenSSL. The verify path is pure Rust so it
//! also compiles to WASM — revocation *data* (CRLs) is supplied by the host, since the
//! core does no network I/O.

use base64::Engine;
use cms::content_info::ContentInfo;
use cms::signed_data::{SignedData, SignerIdentifier, SignerInfo};
use der::{Decode, Encode};
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::RsaPublicKey;
use sha2::{Digest, Sha256};
use signature::Verifier;
use x509_cert::crl::CertificateList;
use x509_cert::Certificate;

use crate::{Error, Result, TrustConfig, VerifyReport};

const OID_MESSAGE_DIGEST: &str = "1.2.840.113549.1.9.4";
const OID_SHA256: &str = "2.16.840.1.101.3.4.2.1";

/// Outcome of verifying one detached CMS signature.
struct SigOutcome {
    signature_valid: bool,
    trusted: bool,
}

/// The bytes the data signature is computed over. See the module docs.
pub(crate) fn data_signing_payload(raw: &[u8]) -> Result<Vec<u8>> {
    let data = crate::canon::inner_text_by_id(raw, "hfp-data")?;
    let hfp_id = crate::canon::meta_content(raw, "hfp-id")?.ok_or(Error::MissingMeta("hfp-id"))?;
    let author_sig_b64 = crate::canon::inner_text_by_id(raw, "hfp-author-signature")?;
    let author_sig_der = base64::engine::general_purpose::STANDARD
        .decode(author_sig_b64.trim())
        .map_err(|_| Error::MissingBlock("hfp-author-signature"))?;
    let author_sig_hash = hex(&Sha256::digest(&author_sig_der));

    let mut payload = Vec::new();
    payload.extend_from_slice(b"hfp-data-sig-v1\n");
    payload.extend_from_slice(hfp_id.as_bytes());
    payload.push(b'\n');
    payload.extend_from_slice(author_sig_hash.as_bytes());
    payload.push(b'\n');
    payload.extend_from_slice(data.as_bytes());
    Ok(payload)
}

/// Verify the author and data signatures against `trust`.
pub(crate) fn verify(raw: &[u8], trust: &TrustConfig) -> Result<VerifyReport> {
    let mut report = VerifyReport::default();

    // --- Author signature over the canonical document. ---
    let author_content = crate::canon::canonical_author_bytes(raw)?;
    let author = match decode_block(raw, "hfp-author-signature") {
        Ok(der) => verify_cms(&der, &author_content, trust, "author", &mut report.notes),
        Err(note) => {
            report.notes.push(note);
            SigOutcome {
                signature_valid: false,
                trusted: false,
            }
        }
    };
    report.author_signature_valid = author.signature_valid;

    // --- Data signature over the bound data payload. ---
    let data = match data_signing_payload(raw) {
        Ok(payload) => match decode_block(raw, "hfp-data-signature") {
            Ok(der) => verify_cms(&der, &payload, trust, "data", &mut report.notes),
            Err(note) => {
                report.notes.push(note);
                SigOutcome {
                    signature_valid: false,
                    trusted: false,
                }
            }
        },
        Err(e) => {
            report
                .notes
                .push(format!("data: cannot build signing payload: {e}"));
            SigOutcome {
                signature_valid: false,
                trusted: false,
            }
        }
    };
    report.data_signature_valid = data.signature_valid;

    report.is_trusted = author.trusted && data.trusted;
    Ok(report)
}

/// Base64-decode the inner content of a signature block.
fn decode_block(raw: &[u8], id: &'static str) -> std::result::Result<Vec<u8>, String> {
    let b64 = crate::canon::inner_text_by_id(raw, id).map_err(|e| format!("{id}: {e}"))?;
    if b64.trim().is_empty() {
        return Err(format!("{id}: empty (document not signed)"));
    }
    base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| format!("{id}: not valid base64"))
}

fn verify_cms(
    sig_der: &[u8],
    content: &[u8],
    trust: &TrustConfig,
    role: &str,
    notes: &mut Vec<String>,
) -> SigOutcome {
    let untrusted = SigOutcome {
        signature_valid: false,
        trusted: false,
    };

    let Ok(ci) = ContentInfo::from_der(sig_der) else {
        notes.push(format!("{role}: not a CMS ContentInfo"));
        return untrusted;
    };
    let Ok(sd) = ci.content.decode_as::<SignedData>() else {
        notes.push(format!("{role}: not CMS SignedData"));
        return untrusted;
    };
    let Some(si) = sd.signer_infos.0.as_slice().first().cloned() else {
        notes.push(format!("{role}: no signer info"));
        return untrusted;
    };
    let Some(signer_cert) = find_signer_cert(&sd, &si) else {
        notes.push(format!("{role}: signer certificate not in CMS"));
        return untrusted;
    };

    // 1) message digest + 2) signature over signed attributes.
    let signature_valid = match check_signature(&si, &signer_cert, content) {
        Ok(()) => true,
        Err(why) => {
            notes.push(format!("{role}: signature invalid ({why})"));
            false
        }
    };

    let subject = signer_cert.tbs_certificate.subject.to_string();
    notes.push(format!("{role}: signed by {subject}"));

    // 4) chain to a whitelisted trust anchor.
    let mut trusted = false;
    match chain_to_anchor(&signer_cert, trust) {
        Some(anchor_thumb) => {
            let whitelisted = trust
                .allowed_ca_thumbprints
                .iter()
                .any(|t| t == &anchor_thumb);
            if trust.require_from_allowed_ca && !whitelisted {
                notes.push(format!("{role}: anchor {anchor_thumb} not in whitelist"));
            } else {
                trusted = signature_valid;
            }
        }
        None => notes.push(format!(
            "{role}: does not chain to a configured trust anchor"
        )),
    }

    // 5) revocation.
    if trusted && !trust.no_revocation_check {
        if is_revoked(&signer_cert, trust) {
            notes.push(format!("{role}: certificate is REVOKED"));
            trusted = false;
        }
    } else if trusted && trust.no_revocation_check {
        notes.push(format!("{role}: revocation check skipped"));
    }

    SigOutcome {
        signature_valid,
        trusted,
    }
}

/// Pull the signer certificate out of the CMS, matched by issuer + serial.
fn find_signer_cert(sd: &SignedData, si: &SignerInfo) -> Option<Certificate> {
    let SignerIdentifier::IssuerAndSerialNumber(ias) = &si.sid else {
        return None;
    };
    sd.certificates.as_ref()?.0.iter().find_map(|c| match c {
        cms::cert::CertificateChoices::Certificate(cert)
            if cert.tbs_certificate.serial_number == ias.serial_number
                && cert.tbs_certificate.issuer == ias.issuer =>
        {
            Some(cert.clone())
        }
        _ => None,
    })
}

/// Check the messageDigest attribute and the RSA signature over the signed attributes.
fn check_signature(
    si: &SignerInfo,
    signer_cert: &Certificate,
    content: &[u8],
) -> std::result::Result<(), &'static str> {
    if si.digest_alg.oid.to_string() != OID_SHA256 {
        return Err("unsupported digest algorithm (expected SHA-256)");
    }
    let signed_attrs = si.signed_attrs.as_ref().ok_or("no signed attributes")?;

    // messageDigest must equal SHA-256(content).
    let md_oid = const_oid::ObjectIdentifier::new_unwrap(OID_MESSAGE_DIGEST);
    let md_attr = signed_attrs
        .iter()
        .find(|a| a.oid == md_oid)
        .ok_or("no messageDigest attribute")?;
    let md_value = md_attr.values.iter().next().ok_or("empty messageDigest")?;
    let md = md_value
        .decode_as::<der::asn1::OctetString>()
        .map_err(|_| "messageDigest not an octet string")?;
    if md.as_bytes() != Sha256::digest(content).as_slice() {
        return Err("messageDigest does not match content");
    }

    // Signature over the DER SET OF signed attributes.
    let signed_attrs_der = signed_attrs.to_der().map_err(|_| "cannot encode attrs")?;
    let spki_der = signer_cert
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|_| "cannot encode spki")?;
    let pubkey = RsaPublicKey::from_public_key_der(&spki_der).map_err(|_| "signer key not RSA")?;
    let vk = VerifyingKey::<Sha256>::new(pubkey);
    let sig = Signature::try_from(si.signature.as_bytes()).map_err(|_| "bad signature value")?;
    vk.verify(&signed_attrs_der, &sig)
        .map_err(|_| "RSA signature does not verify")
}

/// If the signer cert is signed by one of the configured anchors, return that anchor's
/// SHA-256 thumbprint (lowercase hex of its DER).
fn chain_to_anchor(signer_cert: &Certificate, trust: &TrustConfig) -> Option<String> {
    let tbs_der = signer_cert.tbs_certificate.to_der().ok()?;
    let cert_sig = Signature::try_from(signer_cert.signature.raw_bytes()).ok()?;
    for anchor_der in &trust.trust_anchors {
        let Ok(anchor) = Certificate::from_der(anchor_der) else {
            continue;
        };
        if anchor.tbs_certificate.subject != signer_cert.tbs_certificate.issuer {
            continue;
        }
        let Ok(spki) = anchor.tbs_certificate.subject_public_key_info.to_der() else {
            continue;
        };
        let Ok(pubkey) = RsaPublicKey::from_public_key_der(&spki) else {
            continue;
        };
        let vk = VerifyingKey::<Sha256>::new(pubkey);
        if vk.verify(&tbs_der, &cert_sig).is_ok() {
            return Some(hex(&Sha256::digest(anchor_der)));
        }
    }
    None
}

/// True if a supplied CRL (issued by the signer's issuer) lists the signer's serial.
fn is_revoked(signer_cert: &Certificate, trust: &TrustConfig) -> bool {
    let serial = &signer_cert.tbs_certificate.serial_number;
    let issuer = &signer_cert.tbs_certificate.issuer;
    for crl_der in &trust.crls {
        let Ok(crl) = CertificateList::from_der(crl_der) else {
            continue;
        };
        if &crl.tbs_cert_list.issuer != issuer {
            continue;
        }
        if let Some(revoked) = &crl.tbs_cert_list.revoked_certificates {
            if revoked.iter().any(|r| &r.serial_number == serial) {
                return true;
            }
        }
    }
    false
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // `QUJD` is base64 for "ABC" — a stand-in author signature.
    const DOC: &str = r#"<!DOCTYPE html><html><head>
        <meta name="hfp-id" content="ID-1">
        <script id="hfp-data" type="application/json">{"a":1}</script>
        <script id="hfp-author-signature" type="application/pkcs7-signature">QUJD</script>
        <script id="hfp-data-signature" type="application/pkcs7-signature"></script>
        </head><body></body></html>"#;

    #[test]
    fn payload_binds_id_and_data() {
        let payload = data_signing_payload(DOC.as_bytes()).unwrap();
        let s = String::from_utf8(payload).unwrap();
        assert!(s.starts_with("hfp-data-sig-v1\nID-1\n"));
        assert!(s.ends_with(r#"{"a":1}"#));
    }

    #[test]
    fn payload_changes_when_author_signature_changes() {
        let a = data_signing_payload(DOC.as_bytes()).unwrap();
        let other = DOC.replace("QUJD", "WFla"); // base64 for "XYZ"
        let b = data_signing_payload(other.as_bytes()).unwrap();
        assert_ne!(a, b, "data payload must bind the author signature hash");
    }

    #[test]
    fn payload_requires_hfp_id() {
        let doc = DOC.replace(r#"<meta name="hfp-id" content="ID-1">"#, "");
        assert_eq!(
            data_signing_payload(doc.as_bytes()),
            Err(Error::MissingMeta("hfp-id"))
        );
    }

    #[test]
    fn verify_reports_unsigned_document() {
        // No real signatures + no trust anchors -> nothing valid, nothing trusted.
        let report = verify(DOC.as_bytes(), &TrustConfig::default()).unwrap();
        assert!(!report.author_signature_valid);
        assert!(!report.is_trusted);
    }
}
