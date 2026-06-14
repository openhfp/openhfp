# HFP Specification

> **Draft / pre-alpha.** These documents describe the intended design. They are
> normative targets for the reference implementation, not a frozen standard.

| Document | Scope |
|----------|-------|
| [format.md](format.md) | The `.hfp` file: structure, blocks, identity, attachments |
| [runtime.md](runtime.md) | The `window.hfp` runtime contract and host-to-form events |
| [canonicalization.md](canonicalization.md) | How the canonical bytes for signatures are derived |
| [threat-model.md](threat-model.md) | What HFP does and does not protect against |

## The two load-bearing principles

Every part of the specification is constrained by these:

1. **Attribution, not security.** HFP guarantees authorship of the document and of the
   data. It does not claim the content is safe.
2. **Open HTML/CSS/JS, no UX limits.** The format never restricts how a form renders or
   behaves; the schema is a data contract only.
