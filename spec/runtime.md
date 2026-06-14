# HFP Runtime Contract

> **Draft / pre-alpha.** The authoritative shape lives in
> [`@openhfp/types`](../packages/types/src/index.ts).

The host (Filler, or the browser dev shim) injects a frozen `window.hfp` object **before**
`DOMContentLoaded` and user scripts. Communication is one-way per direction:

- **Form → host:** method calls on `window.hfp`.
- **Host → form:** `CustomEvent`s dispatched on `document` (never an `EventTarget` loop on
  `window.hfp`, which would let a form bypass validation).

## `window.hfp`

See [`HfpApi`](../packages/types/src/index.ts) for the exact interface. Key points:

- `setData(data)` returns a `ValidationResult` (path + message) as a **UX aid**. The
  security boundary is the host (Rust core): it re-validates and refuses to sign data that
  does not match the schema.
- `hfpId` (template, signed by the author) and `instanceId` (this filled instance) are
  read-only.

## Events (host → form)

| Event | Detail | When |
|-------|--------|------|
| `hfp:ready` | `{ data, schema, status }` | after load, before user scripts act |
| `hfp:status` | `{ authorSignatureValid, dataSignatureValid, isTrusted }` | trust info |
| `hfp:before-sign` / `hfp:after-sign` | — | signing lifecycle hooks |

## Authoring paths

1. **Plain HTML** — a `bindForm(formElement)` helper binds inputs by `name` path.
2. **Framework (React/Vue/Svelte)** — the framework owns state and calls `setData(state)`
   on change.

Forms target both the host and a plain browser with the same code:

```js
const hfp = window.hfp ?? createDevShimFromDocument();
document.addEventListener("hfp:ready", (e) => initForm(e.detail.data));
```
