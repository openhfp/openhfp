// Generate the conformance corpus from `cases/base/input.hfp`.
//
// Produces, for each case: input.hfp + expect.json, and for "ok" cases the expected
// canonical.bytes / canonical.sha256 (authored by the reference engine — the native
// `hfp` binary). Re-run after changing the canonicalization rules:
//
//   node conformance/gen.mjs
//
// The companion `run.mjs` then checks native == wasm == these stored expectations.
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(fileURLToPath(import.meta.url));
const CASES = join(ROOT, "cases");
const NATIVE = join(ROOT, "..", "target", "release", "hfp");

const base = readFileSync(join(CASES, "base", "input.hfp"), "utf8");

// --- Benign mutations: every one MUST canonicalize to the same bytes as `base`. ---
const benign = {
  "crlf": base.replace(/\n/g, "\r\n"),
  "bom": "﻿" + base,
  "reordered-attrs": base
    .replace('<html lang="en">', "<html lang=\"en\">") // (no-op anchor, html has one attr)
    .replace(
      '<meta name="hfp-id" content="b3c1e2a4-0000-4a1b-8c2d-000000000001">',
      '<meta content="b3c1e2a4-0000-4a1b-8c2d-000000000001" name="hfp-id">'
    ),
  // Collapse pure inter-tag whitespace; script text content is left intact.
  "minified": base.replace(/>\s+</g, "><"),
  // Extra indentation + an HTML comment adjacent to #hfp-data.
  "formatted": base.replace(
    '<script id="hfp-data"',
    "\n      <!-- filled at sign time -->\n      <script id=\"hfp-data\""
  ),
  // Different filled-in data: the #hfp-data content is emptied, so the hash is unchanged.
  "different-data": base.replace(
    '{ "technician": "Jan Novak", "device": "Pump A-12", "faults": ["seal leak"] }',
    '{ "technician": "ZZZ", "device": "QQQ-99", "faults": ["a","b","c"], "extra": 1 }'
  ),
};

// --- Hard-fail cases. ---
const failing = {
  "dup-data": {
    input: base.replace(
      '<script id="hfp-schema" type="application/json">',
      '<script id="hfp-data" type="application/json">{}</script>\n  <script id="hfp-schema" type="application/json">'
    ),
    reason: "duplicate id: hfp-data",
  },
  "missing-data": {
    input: base.replace(
      /<script id="hfp-data" type="application\/json">[\s\S]*?<\/script>/,
      ""
    ),
    reason: "required block is missing: hfp-data",
  },
};

function write(caseName, file, data) {
  const dir = join(CASES, caseName);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, file), data);
}

function nativeCanonical(caseName) {
  const input = join(CASES, caseName, "input.hfp");
  const bytes = execFileSync(NATIVE, ["canonicalize", input]);
  const sha = execFileSync(NATIVE, ["canonicalize", "--sha", input]).toString().trim();
  return { bytes, sha };
}

// base is an ok case too.
write("base", "expect.json", JSON.stringify({ canonicalize: "ok" }, null, 2) + "\n");

for (const [name, input] of Object.entries(benign)) {
  write(name, "input.hfp", input);
  write(name, "expect.json", JSON.stringify({ canonicalize: "ok" }, null, 2) + "\n");
}

for (const [name, { input, reason }] of Object.entries(failing)) {
  write(name, "input.hfp", input);
  write(name, "expect.json", JSON.stringify({ canonicalize: "fail", reason }, null, 2) + "\n");
}

// invalid-utf8: raw bytes, not derivable from a string mutation.
write("invalid-utf8", "input.hfp", Buffer.from([0x3c, 0x68, 0x74, 0x6d, 0x6c, 0x3e, 0xff, 0xfe]));
write(
  "invalid-utf8",
  "expect.json",
  JSON.stringify({ canonicalize: "fail", reason: "input is not valid UTF-8" }, null, 2) + "\n"
);

// Author expected canonical artifacts for every ok case from the native engine.
const okCases = ["base", ...Object.keys(benign)];
for (const name of okCases) {
  const { bytes, sha } = nativeCanonical(name);
  write(name, "canonical.bytes", bytes);
  write(name, "canonical.sha256", sha + "\n");
}

console.log(`generated ${okCases.length} ok + ${Object.keys(failing).length + 1} fail cases`);
