# HFP Canonicalization

> **Draft / pre-alpha.** The exact algorithm is being pinned down in Spike A; this is the
> intended shape. The conformance corpus (`conformance/`) is the binding contract.

Signatures are computed over a **canonical byte representation** of the document, derived
deterministically so that benign reformatting (a formatter, a minifier, reordered
attributes, line-ending changes) does not invalidate a signature.

## Algorithm (intended)

```
canonicalize(raw_bytes):
  1. UTF-8 decode (hard fail if invalid)
  2. Normalize line endings CRLF/CR -> LF; strip a leading BOM
  3. Parse with a real HTML parser (lol-html or html5ever — decided in Spike A),
     not a regex
  4. Locate the elements with id="hfp-data" and id="hfp-data-signature"
       - duplicate id            -> HARD FAIL
       - missing required element -> HARD FAIL
  5. Empty only their inner content (keep the tags)
  6. Serialize back to UTF-8 bytes
  -> SHA-256 of those bytes is the signed digest
```

- The author signature covers the canonical document with both `#hfp-data` and
  `#hfp-data-signature` emptied.
- The data signature covers the canonical data plus the `hfp-id` and a hash of the author
  signature.

## Determinism requirements

- One implementation, byte-identical everywhere. The reference engine (`hfp-core`) is
  intended to run natively and as WASM so the CLI, the Filler and the browser dev shim all
  produce the same bytes. Whether WASM and native are byte-identical is the explicit pass
  criterion of Spike A.
- `hfp canonicalize --explain` emits exactly what was hashed, as an audit anchor.

## Conformance

The `conformance/` corpus holds `.hfp` inputs with their expected canonical bytes and
SHA-256, plus verify pass/fail expectations. Every implementation runs it in CI.
