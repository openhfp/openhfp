# HFP Canonicalization

> **Pinned by Spike A (v1).** The algorithm below is implemented in `crates/hfp-core`
> (`canon.rs`) and exercised by the conformance corpus (the binding contract). See
> [spike-a-findings.md](spike-a-findings.md) for the decisions, results and open questions
> (notably the inline-whitespace caveat). Parser chosen: **html5ever + a custom
> deterministic serializer**.

Signatures are computed over a **canonical byte representation** of the document, derived
deterministically so that benign reformatting (a formatter, a minifier, reordered
attributes, line-ending changes) does not invalidate a signature.

## Algorithm

```
canonicalize(raw_bytes):
  1. UTF-8 decode (hard fail if invalid)
  2. Strip a leading BOM; normalize line endings CRLF/CR -> LF
  3. Parse with the HTML5 algorithm (html5ever — chosen in Spike A), not a regex
  4. Locate the elements with id="hfp-data" and id="hfp-data-signature"
       - duplicate id            -> HARD FAIL
       - missing required element -> HARD FAIL
  5. Empty only their inner content (keep the tags)
  6. Serialize through the canonical serializer (sorted attributes, double-quoted;
     void elements unclosed; raw-text verbatim; comments dropped; whitespace-only
     inter-element text dropped) back to UTF-8 bytes
  -> SHA-256 of those bytes is the signed digest
```

The exact serialization rules and their rationale live in
[spike-a-findings.md](spike-a-findings.md) and `crates/hfp-core/src/canon.rs`.

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
