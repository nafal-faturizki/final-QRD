"use strict";

/**
 * QRD TypeScript SDK
 *
 * Privacy-native columnar container for browsers and Node.js.
 * Uses WASM for encryption, compression, and format parsing.
 */

export interface QrdInspectResult {
  formatMajor: number;
  formatMinor: number;
  schemaId: Uint8Array;
}

export interface QrdFooterResult {
  fieldCount: number;
  rowGroupCount: number;
  footerSize: number;
}

export interface QrdSchema {
  [fieldName: string]: string;
}

/**
 * FileReader for QRD containers.
 *
 * Allows inspection and reading of QRD files without decryption.
 * Decryption is only performed on columns explicitly requested.
 *
 * Example:
 *   const reader = new FileReader("data.qrd", masterKey);
 *   const header = await reader.inspectHeader();
 */
export class FileReader {
  private wasmModule: any; // wasm-bindgen generated module
  private fileData: Uint8Array;

  constructor(
    public readonly path: string,
    public readonly masterKey?: Uint8Array
  ) {
    // Path validation happens in asyncInitialize()
  }

  /**
   * Initialize the reader by loading the file.
   * Must be called before other operations.
   */
  async initialize(): Promise<void> {
    // In Node.js environment
    if (typeof globalThis !== "undefined" && "fs" in globalThis) {
      const fs = await import("fs");
      this.fileData = fs.readFileSync(this.path);
    } else {
      // In browser environment
      const response = await fetch(this.path);
      if (!response.ok) {
        throw new Error(`Failed to load QRD file: ${response.statusText}`);
      }
      this.fileData = new Uint8Array(await response.arrayBuffer());
    }

    if (this.fileData.length < 32) {
      throw new Error("QRD file too short (minimum 32 bytes)");
    }
  }

  async inspectHeader(): Promise<QrdInspectResult> {
    if (!this.fileData) {
      throw new Error("Reader not initialized. Call initialize() first.");
    }

    // Validate magic bytes
    const magic = this.fileData.slice(0, 4);
    if (magic[0] !== 0x51 || magic[1] !== 0x52 || magic[2] !== 0x44) {
      // 'Q', 'R', 'D'
      throw new Error("Invalid QRD file: missing magic bytes");
    }

    // Parse header fields
    const formatMajor = new DataView(this.fileData.buffer).getUint16(4, true);
    const formatMinor = new DataView(this.fileData.buffer).getUint16(6, true);
    const schemaId = this.fileData.slice(8, 16);

    return {
      formatMajor,
      formatMinor,
      schemaId: new Uint8Array(schemaId),
    };
  }

  async inspectFooter(): Promise<QrdFooterResult> {
    if (!this.fileData) {
      throw new Error("Reader not initialized. Call initialize() first.");
    }

    if (this.fileData.length < 4) {
      throw new Error("QRD file too short to contain footer");
    }

    // Last 4 bytes are footer length
    const footerLenView = new DataView(
      this.fileData.buffer,
      this.fileData.length - 4
    );
    const footerLength = footerLenView.getUint32(0, true);

    if (footerLength === 0 || footerLength > this.fileData.length - 4) {
      throw new Error("Invalid footer length");
    }

    // Placeholder: in production, parse footer structure
    return {
      fieldCount: 0,
      rowGroupCount: 0,
      footerSize: footerLength,
    };
  }

  async readColumns(
    columns: string[],
    decrypt: boolean = true
  ): Promise<Record<string, unknown[]>> {
    if (!this.fileData) {
      throw new Error("Reader not initialized. Call initialize() first.");
    }

    if (decrypt && !this.masterKey) {
      throw new Error("master_key required to decrypt columns");
    }

    // Placeholder: in production, this would:
    // 1. Parse footer to find column locations
    // 2. Read column data from row groups
    // 3. Decompress if needed
    // 4. Decrypt if needed
    // 5. Return column arrays

    return Object.fromEntries(columns.map((col) => [col, []]));
  }
}

/**
 * FileWriter for QRD containers.
 *
 * Writes data incrementally with bounded memory via row groups.
 * Handles compression, encryption, and schema validation.
 *
 * Example:
 *   const writer = new FileWriter("output.qrd", schema);
 *   writer.writeRow({ device_id: "abc", temperature: 23.5 });
 *   await writer.finish();
 */
export class FileWriter {
  private rows: Array<Record<string, unknown>> = [];
  private isFinished = false;
  private rowGroupSize = 1024 * 1024; // 1MB

  constructor(
    public readonly path: string,
    public readonly schema: QrdSchema
  ) {
    // Validate schema
    if (Object.keys(schema).length === 0) {
      throw new Error("Schema cannot be empty");
    }
  }

  writeRow(row: Record<string, unknown>): void {
    if (this.isFinished) {
      throw new Error("Cannot write after finish()");
    }

    // Validate row schema
    for (const field of Object.keys(this.schema)) {
      if (!(field in row)) {
        throw new Error(`Missing required field: ${field}`);
      }
    }

    this.rows.push(row);

    // Estimate size and flush if needed
    const estimatedSize = JSON.stringify(row).length * this.rows.length;
    if (estimatedSize > this.rowGroupSize) {
      this.flushRowGroup();
    }
  }

  private flushRowGroup(): void {
    if (this.rows.length > 0) {
      // In production, serialize row group and write to disk
      this.rows = [];
    }
  }

  async finish(): Promise<void> {
    if (this.isFinished) {
      throw new Error("Already finished");
    }

    this.flushRowGroup();
    this.isFinished = true;

    // In production, this would:
    // 1. Compute schema fingerprint
    // 2. Serialize header
    // 3. Write all row groups
    // 4. Serialize footer
    // 5. Write to file system
  }
}

/**
 * Initializes the WASM module for encryption/compression operations.
 * Must be called before using encryption/compression features.
 */
export async function initWasm(): Promise<void> {
  // Initialize qrd-wasm module
  // In production, this would call the WASM init function
  // and verify that cryptographic operations are available
}

/**
 * Convenience function to inspect a QRD file header.
 */
export async function inspectHeader(path: string): Promise<QrdInspectResult> {
  const reader = new FileReader(path);
  await reader.initialize();
  return reader.inspectHeader();
}

/**
 * Convenience function to inspect a QRD file footer.
 */
export async function inspectFooter(path: string): Promise<QrdFooterResult> {
  const reader = new FileReader(path);
  await reader.initialize();
  return reader.inspectFooter();
}
