# OpenHFP — HTML Form Package

> **Status: pre-alpha.** Specification and reference implementation are under active design.
> No stable release yet. See the [roadmap](#roadmap) for the current phase.

**HFP** (`.hfp`) is an open format for **self-contained form documents**: a single HTML
file that bundles the form, its data (embedded JSON), and digital signatures of both the
author and the person who fills it in. One file = form + data + attribution.

Philosophy: *PDF forms, done right — web technologies, no UX limits, a transparent trust model.*

## Two load-bearing principles

These are non-negotiable and act as the acceptance test for every feature:

1. **Attribution, not security.** The format guarantees only **authorship of the document
   and authorship of the data** — it does *not* claim the content is "safe". Instead of
   restricting the format, we ship **tools** to verify and inspect (`hfp verify`,
   `hfp audit`, `hfp canonicalize --explain`). The trust decision stays with the user and
   enterprise policy.
2. **Open HTML/CSS/JS, no UX limits.** The format never restricts how a form renders or
   behaves. The schema is a *data contract* only (shape of the data for validation and
   extraction), never a UX restriction.

## How it works

A `.hfp` file is a valid HTML document. Machine-readable blocks live in the `<head>`:

- `#hfp-schema` — field definitions, types, validation rules (JSON Schema subset)
- `#hfp-data` — filled-in data as JSON, readable without rendering
- `#hfp-author-signature` / `#hfp-data-signature` — CMS/PKCS#7 detached signatures
- `application/hfp-blob` script blocks — binary attachments as Base64

The author signs the canonical document; the filler signs the canonical data plus the
`hfp-id` and a hash of the author's signature (binding data to a specific form version).

## Ecosystem

| Component | What it is | Status |
|-----------|-----------|--------|
| `hfp-core` | Rust crate — parse, canonicalize, extract, validate, verify, sign (+ WASM target) | canonicalize, verify, extract, validate, sign done; `audit` pending |
| `hfp-cli` | Rust binary — `validate` / `extract` / `verify` / `canonicalize` / `sign` / `audit` | `canonicalize`, `extract`, `validate`, `verify`, `sign`, `data-payload` work |
| `@openhfp/core-wasm` | `hfp-core` read side compiled to WASM (canonicalize/extract/validate) | implemented; WASM == native proven in CI |
| `@openhfp/types` | TypeScript contract for the `window.hfp` runtime API | contract complete |
| `@openhfp/devtools` | Browser dev shim (`createDevShimFromDocument()`, `bindForm()`) | implemented; pluggable validator (JS UX aid, or WASM core); signing mocked |
| HFP Filler | Desktop app (Tauri + Rust) — open, verify, fill, sign, save, print | planned |

## Repository layout

```
spec/          format, runtime, canonicalization and threat-model specifications
crates/        Rust workspace — hfp-core, hfp-cli
packages/      JS/TS workspace — @openhfp/types, @openhfp/devtools
conformance/   .hfp test corpus + expected canonical bytes + verify expectations
templates/     starter form templates (plain HTML, React, Vue)
examples/      golden samples (e.g. service-protocol)
filler/        desktop Filler app (added later)
```

## Roadmap

- **Phase 0 — Specification** ✅ design complete (validated through structured review)
- **Phase 1.0 — De-risk spikes** ✅
  - Spike A: canonicalization determinism ✅ — stable hash across real-world mutations and
    WASM == native, both proven by the conformance corpus (see [spec/spike-a-findings.md](spec/spike-a-findings.md))
  - Spike B: end-to-end CMS/PKCS#7 sign + verify against a CA trust whitelist, incl. revocation ✅ —
    pure-Rust verify (also WASM), CRL revocation, proven by the verify corpus
    (see [spec/spike-b-findings.md](spec/spike-b-findings.md))
- **Phase 1.1 — `hfp-core`** ✅ — `canonicalize`, `verify`, `extract`, `validate`
  (JSON Schema subset) and `sign` (reference signer; sign→verify proven by the verify corpus)
- **Phase 1.2 — CLI** ✅ (core commands) — `canonicalize`/`extract`/`validate`/`verify`/`sign`/`data-payload`
  wired; `audit` JS scan and richer `info` still to come
- **Phase 1.3 — dev-tools + types + templates** ← current — `@openhfp/types` contract, the
  `@openhfp/devtools` browser dev shim (`createDevShimFromDocument`, `bindForm`), the
  `plain-html` starter template, and `@openhfp/core-wasm` (the `hfp-core` read side compiled
  to WASM; `canonicalizeSha256`/`validate` proven byte-identical to native, pluggable into
  the dev shim). Remaining: `verify` in the browser + framework templates.
- then **1.4 Filler PoC**, **1.5 pilot**

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). This repository is **English-only**; localized
documentation, if any, lives under language-specific docs folders.

## License

[MIT](LICENSE).
