# HFP Starter Templates

> **Draft / pre-alpha — to be added in Phase 1.3.**

Minimal, copyable starting points for form authors. Planned:

- `plain-html/` — no framework; binds inputs by `name` via `bindForm()`.
- `react/` — React owns state and calls `setData(state)` on change.
- `vue/` — Vue owns state and calls `setData(state)` on change.

Each template runs in a plain browser via `@openhfp/devtools` and unchanged inside the
Filler. UX patterns (i18n, comboboxes, autocomplete, dirty tracking, XLSX import) are the
author's domain, demonstrated here as best practice — they are not part of the format.
