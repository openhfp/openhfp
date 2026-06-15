# HFP Conformance Corpus

> **Canonicalization cases populated by Spike A.** Signature cases come later (Spike B).

This directory is the binding contract for canonicalization and verification. Every
implementation (the native `hfp-core`, its WASM build, the Filler) must run it in CI.

## Layout

```
cases/
  <case-name>/
    input.hfp            the document under test
    canonical.bytes      expected canonical bytes (ok cases only)
    canonical.sha256     expected SHA-256 of canonical.bytes (ok cases only)
    expect.json          { "canonicalize": "ok|fail", "reason": "..." }
```

## Running it

```sh
cargo build -p hfp-cli --release
cargo build -p hfp-cli --release --target wasm32-wasip1
node conformance/run.mjs    # native == wasm == stored, for every case
node conformance/gen.mjs    # (re)generate cases + expected bytes after a rules change
```

`run.mjs` runs each case through **both** the native binary and the WASM build (via
`node:wasi`, see `wasi-run.mjs`) and asserts they are byte-identical — the explicit pass
criterion of Spike A. `gen.mjs` authors the benign-mutation inputs from `cases/base` and
the expected bytes from the reference engine.

## Current cases

- **ok (must share one hash):** `base`, plus benign mutations `crlf`, `bom`,
  `reordered-attrs`, `minified`, `formatted` (extra whitespace + adjacent comment),
  `different-data` (data is emptied before hashing).
- **fail (must hard-fail):** `dup-data` (duplicate `id="hfp-data"`), `missing-data`,
  `invalid-utf8`.

## Coverage targets

- Benign mutations that MUST yield the same hash: formatter output, minification,
  reordered attributes, CRLF/CR/LF, BOM present/absent, whitespace in and between tags,
  HTML comments adjacent to `#hfp-data`.
- Inputs that MUST hard-fail: duplicate `id="hfp-data"`, missing required block.
- Signature cases: valid chain, untrusted CA, tampered data, replayed data, expired and
  revoked certificates.
