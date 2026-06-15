// Proof of the WASM seam: the hfp-core read side, compiled to WASM, is callable from JS and
// produces the SAME canonical hash as the native `hfp` binary. Run after `npm run build`
// here and `cargo build -p hfp-cli --release`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, writeFileSync, rmSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import init, { canonicalizeSha256, validate, validateData, extract } from "@openhfp/core-wasm";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..", "..");
const NATIVE = join(ROOT, "target", "release", "hfp");

await init({ module_or_path: readFileSync(join(HERE, "..", "dist", "hfp_wasm_bg.wasm")) });

const DOC = `<!DOCTYPE html><html><head>
<script id="hfp-schema" type="application/json">{"type":"object","required":["device"],"properties":{"device":{"type":"string"}}}</script>
<script id="hfp-data" type="application/json">{"device":"Pump A"}</script>
<script id="hfp-data-signature" type="application/pkcs7-signature"></script>
</head><body><p>x</p></body></html>`;

test("WASM canonicalizeSha256 matches the native hfp binary", () => {
  const f = join(HERE, "tmp.hfp");
  writeFileSync(f, DOC);
  try {
    const native = spawnSync(NATIVE, ["canonicalize", "--sha", f]).stdout.toString().trim();
    assert.match(native, /^[0-9a-f]{64}$/, "native sha looks valid");
    assert.equal(canonicalizeSha256(DOC), native, "WASM == native canonical hash");
  } finally {
    rmSync(f, { force: true });
  }
});

test("WASM validate runs and accepts valid data", () => {
  const report = JSON.parse(validate(DOC));
  assert.equal(report.valid, true);
});

test("WASM validateData reports schema violations by path", () => {
  const report = JSON.parse(validateData('{"required":["device"]}', "{}"));
  assert.equal(report.valid, false);
  assert.ok(report.errors.some((e) => e.path === "device"));
});

test("WASM extract returns the embedded data JSON", () => {
  assert.equal(extract(DOC), '{"device":"Pump A"}');
});
