# HFP Starter Templates

> **Pre-alpha.** `plain-html/` is available; framework templates are planned.

Minimal, copyable starting points for form authors.

- `plain-html/form.hfp` ✅ — no framework; binds inputs by `name` via `bindForm()`. The same
  code runs in a plain browser (via `@openhfp/devtools`) and unchanged inside the Filler:
  `const hfp = window.hfp ?? createDevShimFromDocument()`. To run locally: `npm run build`,
  then serve the repo root (e.g. `npx serve .`) and open the file.
- `react/` — React owns state and calls `setData(state)` on change. _(planned)_
- `vue/` — Vue owns state and calls `setData(state)` on change. _(planned)_

Each template runs in a plain browser via `@openhfp/devtools` and unchanged inside the
Filler. UX patterns (i18n, comboboxes, autocomplete, dirty tracking, XLSX import) are the
author's domain, demonstrated here as best practice — they are not part of the format.
