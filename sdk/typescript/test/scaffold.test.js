import test from "node:test";
import assert from "node:assert/strict";

import { FileReader, FileWriter, initWasm } from "../src/index.js";

test("typescript sdk surface exists", () => {
  const reader = new FileReader("example.qrd");
  const writer = new FileWriter("output.qrd", { fields: [] });

  assert.equal(reader.path, "example.qrd");
  assert.equal(writer.path, "output.qrd");
});

test("typescript wasm placeholder raises", async () => {
  await assert.rejects(initWasm, /WASM binding not added yet/);
});
