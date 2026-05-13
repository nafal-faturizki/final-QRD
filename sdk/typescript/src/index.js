export class FileReader {
  constructor(path, masterKey) {
    this.path = path;
    this.masterKey = masterKey;
  }

  async inspectHeader() {
    throw new Error("WASM binding not added yet");
  }

  async inspectFooter() {
    throw new Error("WASM binding not added yet");
  }
}

export class FileWriter {
  constructor(path, schema) {
    this.path = path;
    this.schema = schema;
  }

  async writeRow(row) {
    void row;
    throw new Error("WASM binding not added yet");
  }

  async finish() {
    throw new Error("WASM binding not added yet");
  }
}

export async function initWasm() {
  throw new Error("WASM binding not added yet");
}
