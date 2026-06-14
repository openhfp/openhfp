# HFP Threat Model

> **Draft / pre-alpha.**

This document states plainly what HFP protects against and what it does not. It exists
because HFP deliberately chooses **attribution over security** (principle 1): the format
guarantees *who* authored a document and *who* filled in the data, and nothing more.

## What HFP guarantees

- **Document authorship.** The author signs the canonical document (CMS/PKCS#7 over the
  canonical bytes, see [canonicalization.md](canonicalization.md)). Any change to the
  form invalidates the signature.
- **Data authorship.** The filler signs the canonical data together with the `hfp-id`
  and a hash of the author's signature. This binds the data to one specific form version
  and prevents replaying data from one form into another.
- **Tamper evidence.** Verification fails if either signature does not match.

## What HFP explicitly does NOT guarantee

These are accepted residual risks, mitigated by tooling and policy — never by restricting
the format (which would violate the load-bearing principles).

| Risk | Why it is out of scope | Mitigation |
|------|------------------------|------------|
| A signed form can still be a phishing vector | A signature attributes, it does not vet intent | UI never says "safe", only "signed by: <subject>"; `hfp audit` |
| A trusted author can ship buggy or malicious JS | Signed malware is still malware | `hfp audit` (heuristic JS scan); enterprise policy in the Filler |
| Enterprise mail/AV may block `.html` attachments | Outside the format's control | Distribute via intranet/DMS; ship an AV/proxy whitelisting guide; register a MIME type |
| Offline revocation cannot be checked in an air-gapped Filler | CRL/OCSP need network | `--no-revocation-check` with explicit "valid at signing time" vs "valid now" semantics |

## Trust decisions

HFP surfaces facts (signature valid/invalid, certificate subject, trusted/untrusted per
the configured CA whitelist). The **decision to trust** stays with the user and with
enterprise policy enforced in the Filler — not with the format.

## Runtime containment (Filler)

The desktop Filler renders forms in an air-gapped WebView: no network, no filesystem.
This is a trust/determinism decision (no exfiltration, offline verifiability), not an
attempt to "secure" arbitrary JavaScript. It restricts one capability (live external
requests), not the form's UX or rendering freedom. v1 supports embedded data sources only.
