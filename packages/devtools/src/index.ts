/**
 * `@openhfp/devtools` — browser dev shim for HFP forms.
 *
 * `createDevShimFromDocument()` reads `#hfp-data` / `#hfp-schema` from the document and
 * emulates the `window.hfp` API plus the host-to-form events, so an author can develop a
 * form in a plain browser and run the exact same code unchanged inside the Filler.
 *
 * Dev-mode caveats: validation is the JS subset validator (a UX aid, see `validate.ts`),
 * and signing is mocked. The real security boundary is the Filler's Rust `hfp-core`.
 */
import type { HfpApi, HfpReadyDetail, HfpStatusDetail, ValidationResult } from "@openhfp/types";
import { validateData } from "./validate.js";

/** Options for the dev shim. */
export interface DevShimOptions {
  /** Document to read blocks from and dispatch events on. Defaults to `document`. */
  doc?: Document;
  /**
   * Validator override. Defaults to the bundled JS subset validator (a UX aid). Pass a
   * `hfp-core` WASM-backed validator (e.g. wrapping `@openhfp/core-wasm`'s `validateData`)
   * to share the exact Rust implementation in the browser:
   * `validate: (schema, data) => JSON.parse(validateData(JSON.stringify(schema), JSON.stringify(data)))`.
   */
  validate?: (schema: object, data: object) => ValidationResult;
}

/** Dev status: nothing is signed in dev mode. */
const DEV_STATUS: HfpStatusDetail = {
  authorSignatureValid: false,
  dataSignatureValid: false,
  isTrusted: false,
};

function blockJson(doc: Document, id: string): object {
  const text = doc.getElementById(id)?.textContent?.trim();
  if (!text) return {};
  try {
    return JSON.parse(text) as object;
  } catch {
    return {};
  }
}

function metaContent(doc: Document, name: string): string | undefined {
  return doc.querySelector(`meta[name="${name}"]`)?.getAttribute("content") ?? undefined;
}

/** Build a development-mode `window.hfp` from the blocks embedded in the document. */
export function createDevShimFromDocument(options: DevShimOptions = {}): HfpApi {
  const doc = options.doc ?? document;
  const schema = blockJson(doc, "hfp-schema");
  let data = blockJson(doc, "hfp-data");
  const runValidate = options.validate ?? validateData;

  const dispatch = (name: string, detail: unknown): void => {
    doc.dispatchEvent(new CustomEvent(name, { detail }));
  };

  const api: HfpApi = {
    getData: () => structuredClone(data),
    setData: (next: object): Promise<ValidationResult> => {
      data = structuredClone(next);
      const result = runValidate(schema, data);
      dispatch("hfp:status", DEV_STATUS);
      return Promise.resolve(result);
    },
    patchData: (path: string, value: unknown): void => {
      setByPath(data, path, value);
    },
    getSchema: () => structuredClone(schema),
    validate: (): ValidationResult => runValidate(schema, data),
    save: (): void => {
      console.info("[hfp dev shim] save() is a no-op in dev mode");
    },
    sign: (): void => {
      dispatch("hfp:before-sign", undefined);
      console.info("[hfp dev shim] sign() is mocked in dev mode");
      dispatch("hfp:after-sign", undefined);
    },
    version: metaContent(doc, "hfp-version") ?? "0.0.0",
    hfpId: metaContent(doc, "hfp-id") ?? "",
    instanceId: readInstanceId(data) ?? crypto.randomUUID(),
  };

  // Fire `hfp:ready` after the caller has had a chance to add listeners.
  const ready: HfpReadyDetail = {
    data: api.getData(),
    schema: api.getSchema(),
    status: DEV_STATUS,
  };
  queueMicrotask(() => dispatch("hfp:ready", ready));

  return Object.freeze(api);
}

function readInstanceId(data: object): string | undefined {
  const id = (data as Record<string, unknown>)["instance-id"];
  return typeof id === "string" ? id : undefined;
}

/**
 * Bind a plain `<form>` to the host: on every input, collect named fields by path and push
 * them via `setData`. Inputs are addressed by their `name` (e.g. `faults[0]`, `device`).
 */
export function bindForm(form: HTMLFormElement, hfp: HfpApi): void {
  const collect = (): void => {
    const obj: Record<string, unknown> = {};
    for (const el of Array.from(form.querySelectorAll<HTMLInputElement>("[name]"))) {
      setByPath(obj, el.name, el.value);
    }
    void hfp.setData(obj);
  };
  form.addEventListener("input", collect);
  collect();
}

/** Set `value` at a dotted / indexed `path` (e.g. `a.b`, `faults[0]`) inside `obj`. */
export function setByPath(obj: object, path: string, value: unknown): void {
  const tokens = path
    .replace(/\[(\d+)\]/g, ".$1")
    .split(".")
    .filter((t) => t.length > 0);
  if (tokens.length === 0) return;
  let cur = obj as Record<string, unknown>;
  for (let i = 0; i < tokens.length - 1; i++) {
    const key = tokens[i]!;
    const nextIsIndex = /^\d+$/.test(tokens[i + 1]!);
    if (typeof cur[key] !== "object" || cur[key] === null) {
      cur[key] = nextIsIndex ? [] : {};
    }
    cur = cur[key] as Record<string, unknown>;
  }
  cur[tokens[tokens.length - 1]!] = value;
}
