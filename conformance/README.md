# HFP Conformance Corpus

> **Draft / pre-alpha — corpus to be populated during Spike A.**

This directory is the binding contract for canonicalization and verification. Every
implementation (the native `hfp-core`, its WASM build, the Filler) must run it in CI.

## Intended layout

```
cases/
  <case-name>/
    input.hfp            the document under test
    canonical.bytes      expected canonical bytes (see spec/canonicalization.md)
    canonical.sha256     expected SHA-256 of canonical.bytes
    expect.json          { "canonicalize": "ok|fail", "verify": "ok|fail", "reason": "..." }
```

## Coverage targets

- Benign mutations that MUST yield the same hash: formatter output, minification,
  reordered attributes, CRLF/CR/LF, BOM present/absent, whitespace in and between tags,
  HTML comments adjacent to `#hfp-data`.
- Inputs that MUST hard-fail: duplicate `id="hfp-data"`, missing required block.
- Signature cases: valid chain, untrusted CA, tampered data, replayed data, expired and
  revoked certificates.
