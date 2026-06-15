/**
 * JSON Schema **subset** validator — a UX aid for the dev shim (`type`, `required`,
 * `properties`, `items`, `enum`, `pattern`). It mirrors `hfp-core`'s `schema.rs`, but is
 * deliberately *not* the security boundary: the host (Rust core) re-validates before
 * signing. Unknown keywords are ignored.
 */
import type { ValidationError, ValidationResult } from "@openhfp/types";

type Json = unknown;
type JsonObject = Record<string, Json>;

export function validateData(schema: Json, data: Json): ValidationResult {
  const errors: ValidationError[] = [];
  check(schema, data, "", errors);
  return { valid: errors.length === 0, errors };
}

function isObject(v: Json): v is JsonObject {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function jsonType(v: Json): string {
  if (v === null) return "null";
  if (Array.isArray(v)) return "array";
  if (typeof v === "number") return Number.isInteger(v) ? "integer/number" : "number";
  return typeof v; // "string" | "boolean" | "object" | ...
}

function typeMatches(ty: string, v: Json): boolean {
  switch (ty) {
    case "object": return isObject(v);
    case "array": return Array.isArray(v);
    case "string": return typeof v === "string";
    case "number": return typeof v === "number";
    case "integer": return typeof v === "number" && Number.isInteger(v);
    case "boolean": return typeof v === "boolean";
    case "null": return v === null;
    default: return true; // Unknown type keyword: do not constrain.
  }
}

function push(errors: ValidationError[], path: string, message: string): void {
  errors.push({ path: path === "" ? "(root)" : path, message });
}

function join(path: string, key: string): string {
  return path === "" ? key : `${path}.${key}`;
}

function check(schema: Json, data: Json, path: string, errors: ValidationError[]): void {
  if (!isObject(schema)) return; // A non-object schema constrains nothing.

  if (typeof schema.type === "string" && !typeMatches(schema.type, data)) {
    push(errors, path, `expected type ${schema.type}, got ${jsonType(data)}`);
    return; // Further checks assume the type matched.
  }

  if (Array.isArray(schema.enum) && !schema.enum.some((v) => deepEqual(v, data))) {
    push(errors, path, "value is not one of the allowed enum values");
  }

  if (typeof schema.pattern === "string" && typeof data === "string") {
    try {
      if (!new RegExp(schema.pattern).test(data)) {
        push(errors, path, `does not match pattern /${schema.pattern}/`);
      }
    } catch {
      push(errors, path, `schema has an invalid pattern /${schema.pattern}/`);
    }
  }

  if (isObject(data)) {
    if (Array.isArray(schema.required)) {
      for (const name of schema.required) {
        if (typeof name === "string" && !(name in data)) {
          push(errors, join(path, name), "required property is missing");
        }
      }
    }
    if (isObject(schema.properties)) {
      for (const [name, subschema] of Object.entries(schema.properties)) {
        if (name in data) check(subschema, data[name], join(path, name), errors);
      }
    }
  }

  if (schema.items !== undefined && Array.isArray(data)) {
    data.forEach((value, i) => check(schema.items, value, `${path}[${i}]`, errors));
  }
}

function deepEqual(a: Json, b: Json): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}
