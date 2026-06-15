//! Reference signer (Phase 1.2 write side) — builds detached CMS/PKCS#7 signatures with
//! RustCrypto, closing the sign→verify loop in our own code.
//!
//! Signing needs a private key, so in production it lives in the Filler / CLI (OS keystore),
//! not in the portable verify path. This module is the reference implementation used by
//! `hfp sign` and the tests: given the author and filler signing identities it produces a
//! fully signed `.hfp`.

use base64::Engine;
use cms::builder::{SignedDataBuilder, SignerInfoBuilder};
use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
use cms::content_info::ContentInfo;
use cms::signed_data::{EncapsulatedContentInfo, SignerIdentifier};
use der::{DecodePem, Encode};
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::RsaPrivateKey;
use sha2::{Digest, Sha256};
use spki::AlgorithmIdentifierOwned;
use x509_cert::Certificate;

use crate::{Error, Result};

const OID_SHA256: &str = "2.16.840.1.101.3.4.2.1";
const OID_PKCS7_DATA: &str = "1.2.840.113549.1.7.1";

/// A signing identity: a PEM certificate and its PEM PKCS#8 private key.
pub struct SigningIdentity {
    pub cert_pem: Vec<u8>,
    pub key_pem: Vec<u8>,
}

/// Produce a fully signed `.hfp`: author signature over the canonical document, then the
/// data signature over the bound data payload. The output is the normalized (canonical)
/// document with both signature blocks filled — re-canonicalization on verify is idempotent.
pub(crate) fn sign(
    raw: &[u8],
    author: &SigningIdentity,
    filler: &SigningIdentity,
) -> Result<Vec<u8>> {
    // Author signs the canonical document (data + both signature blocks emptied).
    let author_content = crate::canon::canonical_author_bytes(raw)?;
    let author_cms = sign_detached(&author_content, author)?;
    let author_b64 = b64(&author_cms);
    let with_author =
        crate::canon::serialize_with_inserts(raw, &[("hfp-author-signature", &author_b64)])?;

    // Filler signs the data payload (now that the author signature is in place).
    let data_payload = crate::verify::data_signing_payload(&with_author)?;
    let data_cms = sign_detached(&data_payload, filler)?;
    let data_b64 = b64(&data_cms);

    crate::canon::serialize_with_inserts(&with_author, &[("hfp-data-signature", &data_b64)])
}

/// Build a detached CMS `SignedData` (RSA PKCS#1 v1.5 + SHA-256) over `content`.
pub(crate) fn sign_detached(content: &[u8], id: &SigningIdentity) -> Result<Vec<u8>> {
    let cert = Certificate::from_pem(&id.cert_pem).map_err(|_| Error::Crypto("signer cert PEM"))?;
    let private_key = RsaPrivateKey::from_pkcs8_pem(
        std::str::from_utf8(&id.key_pem).map_err(|_| Error::InvalidUtf8)?,
    )
    .map_err(|_| Error::Crypto("signer key PEM"))?;
    let signing_key = SigningKey::<Sha256>::new(private_key);

    let sid = SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
        issuer: cert.tbs_certificate.issuer.clone(),
        serial_number: cert.tbs_certificate.serial_number.clone(),
    });
    let digest_alg = AlgorithmIdentifierOwned {
        oid: const_oid::ObjectIdentifier::new_unwrap(OID_SHA256),
        parameters: None,
    };
    // Detached: no embedded content, supply the external message digest instead.
    let eci = EncapsulatedContentInfo {
        econtent_type: const_oid::ObjectIdentifier::new_unwrap(OID_PKCS7_DATA),
        econtent: None,
    };
    let message_digest = Sha256::digest(content);

    let sib = SignerInfoBuilder::new(
        &signing_key,
        sid,
        digest_alg.clone(),
        &eci,
        Some(&message_digest),
    )
    .map_err(|_| Error::Crypto("signer info"))?;

    let mut sdb = SignedDataBuilder::new(&eci);
    let ci: ContentInfo = sdb
        .add_digest_algorithm(digest_alg)
        .and_then(|b| b.add_certificate(CertificateChoices::Certificate(cert.clone())))
        .and_then(|b| b.add_signer_info::<_, rsa::pkcs1v15::Signature>(sib))
        .and_then(|b| b.build())
        .map_err(|_| Error::Crypto("CMS build"))?;

    ci.to_der().map_err(|_| Error::Crypto("CMS DER encode"))
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The full sign -> verify roundtrip runs in conformance/verify (needs a PKI, which is
    // generated, never committed). Here we only check that bad key material is rejected
    // cleanly rather than panicking.
    const TEMPLATE: &str = r#"<!DOCTYPE html><html><head>
        <meta name="hfp-id" content="ID-XYZ">
        <script id="hfp-data" type="application/json">{"a":1}</script>
        <script id="hfp-author-signature" type="application/pkcs7-signature"></script>
        <script id="hfp-data-signature" type="application/pkcs7-signature"></script>
        </head><body><p>form</p></body></html>"#;

    #[test]
    fn rejects_bad_key_material() {
        let bad = SigningIdentity {
            cert_pem: b"-----BEGIN CERTIFICATE-----\nnope\n-----END CERTIFICATE-----".to_vec(),
            key_pem: b"-----BEGIN PRIVATE KEY-----\nnope\n-----END PRIVATE KEY-----".to_vec(),
        };
        assert_eq!(
            sign(TEMPLATE.as_bytes(), &bad, &bad),
            Err(Error::Crypto("signer cert PEM"))
        );
    }
}
