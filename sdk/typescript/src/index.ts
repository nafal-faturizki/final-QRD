"use strict";

export type QrdInspectResult = {
  formatMajor: number;
  formatMinor: number;
  schemaId: string;
};

export class FileReader {
  constructor(public readonly path: string, public readonly masterKey?: Uint8Array) {}

  async inspectHeader(): Promise<QrdInspectResult> {
    throw new Error("WASM binding not added yet");
  }

  async inspectFooter(): Promise<Record<string, unknown>> {
    throw new Error("WASM binding not added yet");
  }
}

export class FileWriter {
  constructor(public readonly path: string, public readonly schema: Record<string, unknown>) {}

  async writeRow(row: Record<string, unknown>): Promise<void> {
    void row;
    throw new Error("WASM binding not added yet");
  }

  async finish(): Promise<void> {
    throw new Error("WASM binding not added yet");
  }
}

export async function initWasm(): Promise<void> {
  throw new Error("WASM binding not added yet");
}
