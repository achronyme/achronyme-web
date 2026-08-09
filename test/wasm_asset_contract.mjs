import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const assetRoot = join(projectRoot, "public", "wasm");
const receipt = JSON.parse(readFileSync(join(assetRoot, "release.json"), "utf8"));
const javascriptPath = join(assetRoot, "achronyme_wasm.js");
const wasmPath = join(assetRoot, "achronyme_wasm_bg.wasm");

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

assert.equal(receipt.coreVersion, "0.1.1");
assert.equal(receipt.coreRevision, "b1774e88671a1889146804cb812cb099eb9cc006");
assert.equal(receipt.editorVersion, "0.3.1");
assert.equal(receipt.javascriptSha256, sha256(javascriptPath));
assert.equal(receipt.wasmSha256, sha256(wasmPath));

const module = await import(pathToFileURL(javascriptPath));
module.initSync({ module: readFileSync(wasmPath) });

const result = module.run("print(6 * 7)");
assert.equal(result.success, true, result.error);
assert.equal(result.output, "42");
result.free();

const diagnostics = JSON.parse(module.check("let ="));
assert.ok(diagnostics.length > 0);

const support = JSON.parse(module.runtime_support());
assert.equal(support.ambient_authority, false);

console.log("wasm asset contract verified: receipt, hashes, execution, diagnostics, authority");
