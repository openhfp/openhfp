# Spike B — CMS/PKCS#7 Sign + Verify, CA Whitelist, Revocation (findings)

> **Status: PASS.** End-to-end signing (OpenSSL fixtures) and verification (pure-Rust
> `hfp-core`) work against a CA trust whitelist, with CRL revocation. The verify path
> also compiles to `wasm32-wasip1`.

## Goal

De-risk the trust model's second half: produce real CMS/PKCS#7 detached signatures and
verify them in the portable core against a configured CA whitelist, including revocation —
without depending on a system crypto library (so the core stays WASM-portable).

## Result

| Pass criterion | Outcome |
|---|---|
| Detached CMS sign + verify, end-to-end | ✅ OpenSSL-signed, pure-Rust verified (interop) |
| Chain to a whitelisted CA (SHA-256 thumbprint) | ✅ untrusted CA → `is_trusted=false` |
| Revocation (CRL) | ✅ revoked cert → not trusted; `--no-revocation-check` bypasses |
| Tamper evidence | ✅ a one-byte body edit invalidates the author signature |
| WASM portability of verify | ✅ `cargo build -p hfp-cli --target wasm32-wasip1` |

Verify corpus (`conformance/verify/`), all green:

| Case | author sig | data sig | trusted |
|---|---|---|---|
| `valid` | ✓ | ✓ | ✓ |
| `untrusted-ca` (rogue CA, not whitelisted) | ✓ | ✓ | ✗ |
| `tampered` (body edited after signing) | ✗ | ✓ | ✗ |
| `revoked` (filler cert on CRL) | ✓ | ✓ | ✗ |
| `revoked-no-check` (revocation disabled) | ✓ | ✓ | ✓ |

Run it:

```sh
cargo build -p hfp-cli --release
bash   conformance/verify/gen-pki.sh        # test PKI + CRL (OpenSSL)
node   conformance/verify/build-cases.mjs   # sign the .hfp cases
node   conformance/verify/run-verify.mjs    # assert each VerifyReport
```

## Decision 1 — crypto stack: **pure-Rust RustCrypto, no system OpenSSL**

The `openssl` crate would need `libssl-dev` and does not target WASM. Verification is
therefore built on RustCrypto: `cms`, `x509-cert`, `der`, `spki`, `rsa`, `sha2`. The whole
verify path compiles to `wasm32-wasip1`. **Signing stays out of the portable core** (it
needs a private key / OS keystore); test fixtures are produced with the OpenSSL CLI, which
doubles as an interop proof (OpenSSL-produced CMS verified by RustCrypto).

> Version note: `cms 0.2` pins the `der 0.7` / `const-oid 0.9` / `spki 0.7` generation, and
> `rsa 0.9` integrates with `digest 0.10`, so `sha2` is held at **0.10** (not 0.11). Mixing
> generations does not compile — this is the main packaging gotcha.

## Decision 2 — what each signature covers

- **Author signature**: detached CMS over [`canonical_author_bytes`] — the canonical
  document with `#hfp-data`, `#hfp-data-signature` **and `#hfp-author-signature`** emptied.
- **Data signature**: detached CMS over the data payload:

  ```
  "hfp-data-sig-v1\n" + hfp-id + "\n" + hex(sha256(author_sig_DER)) + "\n" + canonical_data
  ```

  binding the data to the form's `hfp-id` and to the specific author signature, so data
  cannot be replayed into another form/version.

## Decision 3 — trust & revocation are host-fed, not ambient

`TrustConfig` carries DER **trust anchors**, the **whitelist** of allowed CA SHA-256
thumbprints, and DER **CRLs**. The core does **no** network or OS-trust-store I/O: the
Filler/CLI supplies anchors and CRLs. This keeps verification deterministic and works in
the air-gapped Filler (CRLs simply absent → `--no-revocation-check`, "valid at signing
time" semantics per the threat model).

## Spec gap found (and fixed here)

The Spike A canonicalization only empties `#hfp-data` + `#hfp-data-signature`. If the author
signature covered that, it would include **its own** `#hfp-author-signature` bytes — a
cycle. Spike B adds [`canonical_author_bytes`] (empties all three). **`spec/canonicalization.md`
and the spec should state that the author signature uses the three-block canonical form.**

## Open questions handed to the spec

1. **Canonical data definition.** `canonical_data` is currently the LF-normalized, trimmed
   inner text of `#hfp-data`. A reformatting of the JSON would change it. Decide whether to
   canonicalize JSON (e.g. RFC 8785 / JCS) before signing.
2. **Algorithm agility.** v1 verifies RSA PKCS#1 v1.5 + SHA-256 only. Add ECDSA (P-256) and
   RSA-PSS before freeze; reject unknown algorithms explicitly (already done).
3. **Full path validation.** v1 validates a one-level chain (signer ← anchor) by signature
   and checks the anchor thumbprint. It does **not** yet check validity dates, basic
   constraints / path length, key usage, or multi-level intermediates. Needed for v1.0.
4. **OCSP.** Only CRL revocation is implemented; OCSP (stapling for online verify) is a
   follow-up. The threat model already scopes offline/air-gapped to CRL + explicit bypass.
5. **Fixtures are generated, not committed.** The verify corpus depends on freshly minted
   keys (never commit private keys), so CI regenerates PKI + cases each run. The Spike A
   corpus remains the byte-frozen contract; Spike B's is a generate-and-run check.
