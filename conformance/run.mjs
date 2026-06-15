// Conformance runner — the binding check for canonicalization.
//
// For every case it runs BOTH the native `hfp` binary and the `hfp.wasm` (wasm32-wasip1)
// build, and asserts:
//   - ok cases:   native bytes == wasm bytes == stored canonical.bytes, sha matches.
//   - fail cases: native AND wasm both exit non-zero (hard fail).
//
// The native==wasm equality is the explicit pass criterion of Spike A. Run:
//
//   node conformance/run.mjs
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(fileURLToPath(import.meta.url));
const CASES = join(ROOT, "cases");
const NATIVE = join(ROOT, "..", "target", "release", "hfp");
const WASM = join(ROOT, "..", "target", "wasm32-wasip1", "release", "hfp.wasm");
const RUNNER = join(ROOT, "wasi-run.mjs");

function runNative(caseDir) {
  const r = spawnSync(NATIVE, ["canonicalize", join(caseDir, "input.hfp")], { maxBuffer: 1 << 24 });
  return { code: r.status, out: r.stdout ?? Buffer.alloc(0) };
}

function runWasm(caseDir) {
  const r = spawnSync("node", [RUNNER, WASM, caseDir, "canonicalize"], { maxBuffer: 1 << 24 });
  return { code: r.status, out: r.stdout ?? Buffer.alloc(0) };
}

let pass = 0;
const failures = [];

for (const name of readdirSync(CASES).sort()) {
  const caseDir = join(CASES, name);
  if (!existsSync(join(caseDir, "expect.json"))) continue;
  const expect = JSON.parse(readFileSync(join(caseDir, "expect.json"), "utf8"));
  const native = runNative(caseDir);
  const wasm = runWasm(caseDir);
  const errs = [];

  if (expect.canonicalize === "ok") {
    if (native.code !== 0) errs.push(`native exited ${native.code}`);
    if (wasm.code !== 0) errs.push(`wasm exited ${wasm.code}`);
    if (!native.out.equals(wasm.out)) errs.push("native bytes != wasm bytes");
    const stored = readFileSync(join(caseDir, "canonical.bytes"));
    if (!native.out.equals(stored)) errs.push("native bytes != stored canonical.bytes");
    const sha = createHash("sha256").update(native.out).digest("hex");
    const storedSha = readFileSync(join(caseDir, "canonical.sha256"), "utf8").trim();
    if (sha !== storedSha) errs.push(`sha ${sha} != stored ${storedSha}`);
  } else {
    if (native.code === 0) errs.push("native did not hard-fail");
    if (wasm.code === 0) errs.push("wasm did not hard-fail");
  }

  if (errs.length) failures.push(`✗ ${name}: ${errs.join("; ")}`);
  else { pass++; console.log(`✓ ${name}`); }
}

console.log(`\n${pass} passed, ${failures.length} failed`);
for (const f of failures) console.error(f);
process.exit(failures.length ? 1 : 0);
