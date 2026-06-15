// Build the Spike B verify corpus under conformance/verify/cases/.
//
// For each case it runs the real sign flow:
//   1. fill #hfp-data, author-canonicalize (hfp canonicalize --author), sign -> author sig
//   2. compute the data payload (hfp data-payload), sign -> data sig
//   3. assemble the final input.hfp
// and writes a meta.json with the verify trust config + expected report.
//
// Run after gen-pki.sh:  node conformance/verify/build-cases.mjs
import { readFileSync, writeFileSync, mkdirSync, rmSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const PKI = join(HERE, "pki");
const CASES = join(HERE, "cases");
const TMP = join(HERE, ".tmp");
const HFP = join(HERE, "..", "..", "target", "release", "hfp");

const thumbs = Object.fromEntries(
  readFileSync(join(PKI, "thumbprints.txt"), "utf8")
    .trim()
    .split("\n")
    .map((l) => l.split("="))
);

const TEMPLATE = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="hfp-version" content="1.0">
  <meta name="hfp-id" content="b3c1e2a4-0000-4a1b-8c2d-000000000777">
  <meta name="hfp-author" content="CN=ACME Service s.r.o.">
  <title>Service Protocol</title>
  <script id="hfp-schema" type="application/json">{"type":"object"}</script>
  <script id="hfp-data" type="application/json">__DATA__</script>
  <script id="hfp-author-signature" type="application/pkcs7-signature">__AUTHOR_SIG__</script>
  <script id="hfp-data-signature" type="application/pkcs7-signature">__DATA_SIG__</script>
</head>
<body><h1>Service Protocol</h1><p id="marker">v1</p></body>
</html>
`;

const DATA = '{ "technician": "Jan Novak", "device": "Pump A-12" }';

function sh(bin, args, input) {
  return execFileSync(bin, args, { input, maxBuffer: 1 << 24 });
}

// Detached CMS over `contentBytes`, signed by <name>.crt/<name>.key; returns base64(DER).
function signDetached(contentBytes, name) {
  const contentPath = join(TMP, "content.bin");
  writeFileSync(contentPath, contentBytes);
  const der = sh("openssl", [
    "cms", "-sign", "-binary", "-in", contentPath,
    "-signer", join(PKI, `${name}.crt`), "-inkey", join(PKI, `${name}.key`),
    "-outform", "DER",
  ]);
  return der.toString("base64");
}

// Run the sign flow and return the final .hfp text. `doc` keeps the signature
// placeholders until each signature is computed and slotted in.
function buildSigned({ authorSigner, fillerSigner }) {
  let doc = TEMPLATE.replace("__DATA__", DATA);

  // Stage 1: author-canonicalize a copy with empty signature blocks, then sign it.
  const doc1Path = join(TMP, "doc1.hfp");
  writeFileSync(doc1Path, doc.replace("__AUTHOR_SIG__", "").replace("__DATA_SIG__", ""));
  const authorCanon = sh(HFP, ["canonicalize", "--author", doc1Path]);
  doc = doc.replace("__AUTHOR_SIG__", signDetached(authorCanon, authorSigner));

  // Stage 2: with the author signature in place, compute and sign the data payload.
  const doc2Path = join(TMP, "doc2.hfp");
  writeFileSync(doc2Path, doc.replace("__DATA_SIG__", ""));
  const dataPayload = sh(HFP, ["data-payload", doc2Path]);
  doc = doc.replace("__DATA_SIG__", signDetached(dataPayload, fillerSigner));

  return doc;
}

function writeCase(name, hfpText, meta) {
  const dir = join(CASES, name);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "input.hfp"), hfpText);
  writeFileSync(join(dir, "meta.json"), JSON.stringify(meta, null, 2) + "\n");
}

// Common trust config: both anchors known, only the legit CA whitelisted, revocation on.
const baseVerify = {
  anchors: ["pki/ca.der", "pki/untrusted-ca.der"],
  crls: ["pki/crl.der"],
  allow_thumbprints: [thumbs.ca],
  require_allowed: true,
  no_revocation_check: false,
};

rmSync(TMP, { recursive: true, force: true });
mkdirSync(TMP, { recursive: true });
if (existsSync(CASES)) rmSync(CASES, { recursive: true, force: true });

// 1) valid — both certs from the legit CA.
const valid = buildSigned({ authorSigner: "author", fillerSigner: "filler" });
writeCase("valid", valid, {
  verify: baseVerify,
  expect: { author_signature_valid: true, data_signature_valid: true, is_trusted: true },
});

// 2) untrusted-ca — author cert issued by the rogue CA (chains, but not whitelisted).
const untrusted = buildSigned({ authorSigner: "untrusted-author", fillerSigner: "filler" });
writeCase("untrusted-ca", untrusted, {
  verify: baseVerify,
  expect: { author_signature_valid: true, data_signature_valid: true, is_trusted: false },
});

// 3) tampered — flip a byte in the signed body after signing.
const tampered = valid.replace('<p id="marker">v1</p>', '<p id="marker">v2</p>');
writeCase("tampered", tampered, {
  verify: baseVerify,
  expect: { author_signature_valid: false, data_signature_valid: true, is_trusted: false },
});

// 4) revoked — data signed by a revoked filler cert; CRL lists it.
const revoked = buildSigned({ authorSigner: "author", fillerSigner: "filler-revoked" });
writeCase("revoked", revoked, {
  verify: baseVerify,
  expect: { author_signature_valid: true, data_signature_valid: true, is_trusted: false },
});

// 5) revoked-no-check — same document, revocation checking disabled -> trusted.
writeCase("revoked-no-check", revoked, {
  verify: { ...baseVerify, no_revocation_check: true },
  expect: { author_signature_valid: true, data_signature_valid: true, is_trusted: true },
});

rmSync(TMP, { recursive: true, force: true });
console.log("built cases: valid, untrusted-ca, tampered, revoked, revoked-no-check");
