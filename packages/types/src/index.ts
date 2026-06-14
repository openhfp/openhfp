/**
 * `@openhfp/types` — the contract for the `window.hfp` runtime API.
 *
 * This package is the source of truth for how a form talks to its host. The Filler
 * (Rust) implements it, the browser dev shim (`@openhfp/devtools`) emulates it, and
 * form code consumes it. Status: pre-alpha scaffold.
 */

/** A single schema validation error, addressed by JSON path. */
export interface ValidationError {
  /** JSON path to the offending field, e.g. `faults[0].description`. */
  path: string;
  /** Human-readable message intended as a UX hint, not a security boundary. */
  message: string;
}

/** Result of validating the current data against the embedded schema. */
export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
}

/**
 * The frozen object the host injects as `window.hfp` before `DOMContentLoaded`.
 *
 * Validation here is a UX aid only. The security boundary is the host (Rust core):
 * it re-validates and refuses to sign data that does not match the schema.
 */
export interface HfpApi {
  // Data
  getData(): object;
  setData(data: object): Promise<ValidationResult>;
  patchData(path: string, value: unknown): void;
  getSchema(): object;

  // Lifecycle
  validate(): ValidationResult;
  save(): void;
  sign(): void;

  // Info (read-only)
  readonly version: string;
  /** Template id, set and signed by the author. */
  readonly hfpId: string;
  /** UUID of this filled instance, assigned by the host on first open. */
  readonly instanceId: string;
}

/** Detail payload of the `hfp:ready` event dispatched on `document`. */
export interface HfpReadyDetail {
  data: object;
  schema: object;
  status: HfpStatusDetail;
}

/** Detail payload of the `hfp:status` event dispatched on `document`. */
export interface HfpStatusDetail {
  authorSignatureValid: boolean;
  dataSignatureValid: boolean;
  isTrusted: boolean;
}

/** Names of the host-to-form events, dispatched one-way on `document`. */
export type HfpEventName =
  | "hfp:ready"
  | "hfp:status"
  | "hfp:before-sign"
  | "hfp:after-sign";

declare global {
  interface Window {
    /** Injected by the host before user scripts run. Absent in plain browsers. */
    hfp?: HfpApi;
  }
}
