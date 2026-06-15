// Spike B verify runner — checks each case's VerifyReport against its expectation.
//
//   node conformance/verify/run-verify.mjs
//
// For every case it runs the native `hfp verify` with the trust config from meta.json and
// asserts author_signature_valid / data_signature_valid / is_trusted match `expect`.
import { readFileSync, readdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const CASES = join(HERE, "cases");
const HFP = join(HERE, "..", "..", "target", "release", "hfp");

function verifyArgs(input, v) {
  const args = ["verify", input];
  for (const a of v.anchors) args.push("--anchor", join(HERE, a));
  for (const c of v.crls) args.push("--crl", join(HERE, c));
  for (const t of v.allow_thumbprints) args.push("--allow-thumbprint", t);
  if (v.require_allowed) args.push("--require-allowed");
  if (v.no_revocation_check) args.push("--no-revocation-check");
  return args;
}

function parse(stdout) {
  const o = {};
  for (const line of stdout.toString().trim().split("\n")) {
    const [k, val] = line.split("=");
    if (k) o[k] = val === "true";
  }
  return o;
}

let pass = 0;
const failures = [];

for (const name of readdirSync(CASES).sort()) {
  const meta = JSON.parse(readFileSync(join(CASES, name, "meta.json"), "utf8"));
  const input = join(CASES, name, "input.hfp");
  const r = spawnSync(HFP, verifyArgs(input, meta.verify), { maxBuffer: 1 << 24 });
  const got = parse(r.stdout);
  const errs = [];
  for (const key of ["author_signature_valid", "data_signature_valid", "is_trusted"]) {
    if (got[key] !== meta.expect[key]) {
      errs.push(`${key}: got ${got[key]}, want ${meta.expect[key]}`);
    }
  }
  if (errs.length) failures.push(`✗ ${name}: ${errs.join("; ")}`);
  else { pass++; console.log(`✓ ${name}`); }
}

console.log(`\n${pass} passed, ${failures.length} failed`);
for (const f of failures) console.error(f);
process.exit(failures.length ? 1 : 0);
