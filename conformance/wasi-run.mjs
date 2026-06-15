// Run the `hfp.wasm` (wasm32-wasip1) CLI under Node's WASI and stream its stdout.
//
// Used by the conformance runner to prove the WASM build produces byte-identical
// output to the native binary. Invoked as a subprocess so the parent can capture
// fd 1 (the canonical bytes) verbatim:
//
//   node wasi-run.mjs <wasm-path> <case-dir> canonicalize [--sha]
//
// The case directory is preopened as /sandbox; the wasm reads /sandbox/input.hfp.
import { readFileSync } from "node:fs";
import { WASI } from "node:wasi";

const [wasmPath, caseDir, ...cliArgs] = process.argv.slice(2);

const wasi = new WASI({
  version: "preview1",
  args: ["hfp", ...cliArgs, "/sandbox/input.hfp"],
  env: {},
  preopens: { "/sandbox": caseDir },
  returnOnExit: true,
});

const wasm = await WebAssembly.compile(readFileSync(wasmPath));
const instance = await WebAssembly.instantiate(wasm, wasi.getImportObject());
const code = wasi.start(instance);
process.exit(code);
