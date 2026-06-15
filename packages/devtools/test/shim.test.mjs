// Runtime smoke tests for the dev shim, using a minimal fake `doc` (Node's EventTarget +
// getElementById/querySelector) so no jsdom is needed. Run: node --test (after a build).
import { test } from "node:test";
import assert from "node:assert/strict";
import { createDevShimFromDocument, setByPath } from "@openhfp/devtools";

const SCHEMA = JSON.stringify({
  type: "object",
  required: ["technician", "device"],
  properties: {
    technician: { type: "string" },
    device: { type: "string", pattern: "^[A-Z]" },
  },
});

function fakeDoc(blocks, metas = {}) {
  const target = new EventTarget();
  return Object.assign(target, {
    getElementById: (id) => (id in blocks ? { textContent: blocks[id] } : null),
    querySelector: (sel) => {
      const m = sel.match(/meta\[name="(.+)"\]/);
      const name = m?.[1];
      return name && name in metas ? { getAttribute: () => metas[name] } : null;
    },
  });
}

test("reads data and schema from the document", () => {
  const doc = fakeDoc(
    { "hfp-schema": SCHEMA, "hfp-data": '{"technician":"Jan","device":"Pump A"}' },
    { "hfp-id": "ID-1", "hfp-version": "1.0" },
  );
  const hfp = createDevShimFromDocument({ doc });
  assert.equal(hfp.hfpId, "ID-1");
  assert.equal(hfp.version, "1.0");
  assert.deepEqual(hfp.getData(), { technician: "Jan", device: "Pump A" });
  assert.equal(hfp.validate().valid, true);
});

test("validate reports schema problems by path", () => {
  const doc = fakeDoc({ "hfp-schema": SCHEMA, "hfp-data": '{"device":"pump"}' });
  const hfp = createDevShimFromDocument({ doc });
  const result = hfp.validate();
  assert.equal(result.valid, false);
  assert.ok(result.errors.some((e) => e.path === "technician")); // required missing
  assert.ok(result.errors.some((e) => e.path === "device")); // pattern ^[A-Z]
});

test("setData replaces data and revalidates", async () => {
  const doc = fakeDoc({ "hfp-schema": SCHEMA, "hfp-data": "{}" });
  const hfp = createDevShimFromDocument({ doc });
  const result = await hfp.setData({ technician: "Eva", device: "Valve B" });
  assert.equal(result.valid, true);
  assert.deepEqual(hfp.getData(), { technician: "Eva", device: "Valve B" });
});

test("dispatches hfp:ready with data + schema + status", async () => {
  const doc = fakeDoc({ "hfp-schema": SCHEMA, "hfp-data": '{"technician":"Jan","device":"Pump A"}' });
  const ready = new Promise((resolve) => doc.addEventListener("hfp:ready", (e) => resolve(e.detail)));
  createDevShimFromDocument({ doc });
  const detail = await ready;
  assert.deepEqual(detail.data, { technician: "Jan", device: "Pump A" });
  assert.equal(detail.status.isTrusted, false);
});

test("getData returns a copy (no external mutation)", () => {
  const doc = fakeDoc({ "hfp-schema": "{}", "hfp-data": '{"a":1}' });
  const hfp = createDevShimFromDocument({ doc });
  const d = hfp.getData();
  d.a = 999;
  assert.equal(hfp.getData().a, 1);
});

test("setByPath handles dotted and indexed paths", () => {
  const obj = {};
  setByPath(obj, "device", "X");
  setByPath(obj, "faults[0]", "leak");
  setByPath(obj, "meta.author", "Eva");
  assert.deepEqual(obj, { device: "X", faults: ["leak"], meta: { author: "Eva" } });
});
