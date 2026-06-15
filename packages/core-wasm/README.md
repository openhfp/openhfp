# @openhfp/core-wasm

The HFP **read side** (`hfp-core`) compiled to WebAssembly — one engine, callable from JS.
Exposes `canonicalizeSha256`, `extract`, `validate` and `validateData`, so the browser dev
shim and any web SDK share the exact Rust canonicalization and schema validation instead of
a JS re-implementation.

Signing and signature verification stay native (they need keys / are the security boundary),
so this build pulls **no** CMS/RSA/getrandom dependencies — `hfp-core` is built with
`--no-default-features` (the `crypto` feature off).

## Build

Requires the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` (same version as the
`wasm-bindgen` crate):

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.125
npm run build --workspace @openhfp/core-wasm   # cargo build + wasm-bindgen -> dist/
```

The conformance test (`test/wasm.test.mjs`) asserts `canonicalizeSha256` is byte-identical
to the native `hfp` binary — the proof that the WASM and native engines agree.

## Use

```js
import init, { canonicalizeSha256, validateData } from "@openhfp/core-wasm";

await init();                                  // browser: fetches the .wasm
const hash = canonicalizeSha256(htmlString);   // hex SHA-256 of the canonical bytes
const report = JSON.parse(validateData(schemaJson, dataJson));
```

Wire it into the dev shim so the browser validates with the real engine:

```js
import { createDevShimFromDocument } from "@openhfp/devtools";
const hfp = window.hfp ?? createDevShimFromDocument({
  validate: (schema, data) => JSON.parse(validateData(JSON.stringify(schema), JSON.stringify(data))),
});
```
