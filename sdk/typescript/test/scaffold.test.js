import assert from "node:assert/strict";
import { test } from "node:test";
import { FileReader, FileWriter, initWasm } from "../src/index.js";

test("FileReader accepts path", () => {
  const reader = new FileReader("example.qrd");
  assert.strictEqual(reader.path, "example.qrd");
});

test("FileReader with master key", () => {
  const key = new Uint8Array(32);
  const reader = new FileReader("example.qrd", key);
  assert.strictEqual(reader.masterKey, key);
});

test("FileWriter accepts schema", () => {
  const schema = {
    device_id: "utf8",
    temperature: "float32",
  };
  const writer = new FileWriter("output.qrd", schema);
  assert.deepStrictEqual(writer.schema, schema);
});

test("FileWriter rejects empty schema", () => {
  assert.throws(
    () => {
      new FileWriter("output.qrd", {});
    },
    { message: /Schema cannot be empty/ }
  );
});

test("FileWriter validates row schema", () => {
  const writer = new FileWriter("output.qrd", {
    id: "int32",
    name: "utf8",
  });

  assert.throws(
    () => {
      writer.writeRow({ id: 1 }); // missing 'name'
    },
    { message: /Missing required field: name/ }
  );
});

test("FileWriter accepts valid rows", () => {
  const writer = new FileWriter("output.qrd", {
    id: "int32",
    value: "float32",
  });

  writer.writeRow({ id: 1, value: 3.14 });
  writer.writeRow({ id: 2, value: 2.71 });

  assert.strictEqual(writer.rows.length, 2);
});

test("FileWriter rejects write after finish", async () => {
  const writer = new FileWriter("output.qrd", {
    id: "int32",
  });

  await writer.finish();

  assert.throws(
    () => {
      writer.writeRow({ id: 1 });
    },
    { message: /Cannot write after finish/ }
  );
});

test("FileWriter rejects duplicate finish", async () => {
  const writer = new FileWriter("output.qrd", {
    id: "int32",
  });

  await writer.finish();

  assert.rejects(
    async () => {
      await writer.finish();
    },
    { message: /Already finished/ }
  );
});

test("FileWriter accumulates rows", () => {
  const writer = new FileWriter("output.qrd", {
    id: "int32",
    data: "utf8",
  });

  const rows = [
    { id: 1, data: "row1" },
    { id: 2, data: "row2" },
    { id: 3, data: "row3" },
  ];

  for (const row of rows) {
    writer.writeRow(row);
  }

  assert.strictEqual(writer.rows.length, 3);
});

test("initWasm is callable", async () => {
  // Should not throw
  await initWasm();
  assert(true);
});
