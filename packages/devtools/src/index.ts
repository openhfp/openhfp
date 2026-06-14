/**
 * `@openhfp/devtools` — browser dev shim for HFP forms.
 *
 * `createDevShimFromDocument()` reads the `#hfp-data` and `#hfp-schema` blocks from
 * the current document and emulates the `window.hfp` API plus the host-to-form
 * events, so an author can develop a form in a plain browser and run the exact same
 * code unchanged inside the Filler. Signing is mocked in dev mode.
 *
 * Status: pre-alpha scaffold — the real implementation will delegate canonicalization
 * and validation to the `hfp-core` WASM build once Spike A settles that decision.
 */

import type { HfpApi } from "@openhfp/types";

/** Options for the dev shim. */
export interface DevShimOptions {
  /** Document to read `#hfp-data` / `#hfp-schema` from. Defaults to `document`. */
  doc?: Document;
}

/**
 * Build a development-mode `window.hfp` from the blocks embedded in the document.
 *
 * Not implemented yet; declared so form code can already be written against the
 * stable shape: `const hfp = window.hfp ?? createDevShimFromDocument()`.
 */
export function createDevShimFromDocument(_options: DevShimOptions = {}): HfpApi {
  throw new Error("createDevShimFromDocument is not implemented yet (pre-alpha scaffold)");
}
