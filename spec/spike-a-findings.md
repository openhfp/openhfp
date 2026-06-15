# Spike A — Canonicalization Determinism (findings)

> **Status: PASS.** Both pass criteria met. This document records what was built, the
> decisions it forced, and the open questions handed to the spec.

## Goal

De-risk the foundation of the trust model: a canonical byte representation of a `.hfp`
document such that

1. **benign reformatting does not invalidate a signature** (stable SHA-256 across formatter
   output, minification, reordered attributes, CRLF/CR/LF, BOM, inter-tag whitespace and
   adjacent comments), and
2. the **WASM build is byte-identical to native** (so the CLI, the Filler and the browser
   dev shim all hash the same bytes).

## Result

| Pass criterion | Outcome |
|---|---|
| Stable hash across benign mutations | ✅ 7 benign variants → one SHA-256 (`61beb560…23bb`) |
| WASM == native, byte-for-byte | ✅ 10/10 conformance cases identical (`wasm32-wasip1` vs native release) |
| Hard-fail on malformed input | ✅ duplicate `#hfp-data`, missing required block, invalid UTF-8 all fail |

Run it:

```sh
cargo test -p hfp-core                 # unit tests for the canonical rules
cargo build -p hfp-cli --release
cargo build -p hfp-cli --release --target wasm32-wasip1
node conformance/gen.mjs               # (re)author the corpus from the reference engine
node conformance/run.mjs               # native == wasm == stored, for every case
```

## Decision 1 — parser: **html5ever + a custom serializer**

The intended algorithm ("parse, empty the data blocks, serialize") is *not* deterministic on
its own — determinism depends entirely on the serializer. The two candidates fail the same
way for opposite reasons:

- **lol-html** (streaming rewriter) preserves the source bytes, so reordered attributes and
  reformatting stay *different*.
- **html5ever's stock serializer** preserves source attribute order, so reordered attributes
  stay different.

Neither makes "formatter output → same hash" true. Canonicalization must *actively normalize*.
So we parse into a real DOM with **html5ever** (which already normalizes case, implied tags
and duplicate attributes via the HTML5 algorithm) and re-serialize through our own rules. See
`crates/hfp-core/src/canon.rs`.

## Decision 2 — the canonical serialization rules (v1)

1. UTF-8 decode (hard fail otherwise); strip a leading BOM; CRLF/CR → LF.
2. Parse with the HTML5 algorithm.
3. Empty the inner content of `#hfp-data` and `#hfp-data-signature` (keep the tags).
   Duplicate id → hard fail; missing required block → hard fail.
4. Serialize deterministically:
   - element/attribute names lowercased (parser does this for the HTML namespace);
   - **attributes sorted** by (namespace, local name), always double-quoted;
   - **void elements** emitted with no closing tag and no self-closing slash;
   - **raw-text elements** (`script`, `style`) emitted verbatim, not escaped;
   - text escaped as `&`/`<`/`>`; attribute values as `&`/`"`/` `;
   - **comments dropped** (so an adjacent comment is benign);
   - **whitespace-only text nodes between elements dropped**, except inside
     whitespace-significant elements (`pre`, `textarea`, `script`, `style`).

## Open questions handed to the spec

1. **Inline whitespace caveat.** Rule 4's whitespace-only-node dropping also removes a
   *significant* space between two inline elements (`<a>x</a> <a>y</a>` → `<a>x</a><a>y</a>`).
   Two documents that render differently could then share a hash — a (mild) attribution
   concern. Options: (a) accept for v1 (forms rarely rely on inter-element spaces in signed
   chrome); (b) scope the rule to flow/▸block content only; (c) collapse instead of drop.
   **Recommendation:** accept for v1, revisit if the threat model objects.
2. **Required-block policy.** v1 requires *both* `#hfp-data` and `#hfp-data-signature` to be
   present (an unsigned draft must carry empty placeholders). Confirm this matches the
   authoring flow, or relax `#hfp-data-signature` to optional for author-only signing.
3. **Doctype normalization.** Emitted as `<!DOCTYPE ` + parsed name + `>`. Fine for
   `<!DOCTYPE html>`; decide whether legacy/quirky doctypes should be rejected outright.
4. **Corpus authorship.** Expected bytes are currently authored *by the reference engine*
   (`gen.mjs`). That makes the engine self-consistent but not independently checked. Before
   v1.0 freeze, at least the `base` case's `canonical.bytes` should be reviewed by hand.

## Notes

- The WASM build needs no C toolchain (pure-Rust deps + bundled `rust-lld`); native does.
- No floats or hash-map iteration order leak into the output (attributes sorted into a
  `Vec`), which is why native and WASM agree byte-for-byte.
