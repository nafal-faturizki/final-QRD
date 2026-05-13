package qrd

import (
	"os"
	"testing"
)

func TestNewFileReader(t *testing.T) {
	// Test non-existent file
	_, err := NewFileReader("/nonexistent/path", nil)
	if err == nil {
		t.Error("should reject non-existent file")
	}

	// Test with valid file but too short
	tmpFile, err := os.CreateTemp("", "test_*.qrd")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(tmpFile.Name())

	tmpFile.Write([]byte("SHORT"))
	tmpFile.Close()

	_, err = NewFileReader(tmpFile.Name(), nil)
	if err == nil {
		t.Error("should reject file too short")
	}
}

func TestInspectHeader(t *testing.T) {
	// Create temp file with valid header
	tmpFile, err := os.CreateTemp("", "test_*.qrd")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(tmpFile.Name())

	// Write valid header
	header := make([]byte, 32)
	header[0], header[1], header[2] = 0x51, 0x52, 0x44 // "QRD"
	header[4], header[5] = 1, 0                         // format_major = 1
	header[6], header[7] = 0, 0                         // format_minor = 0
	copy(header[8:16], []byte{0, 1, 2, 3, 4, 5, 6, 7}) // schema_id
	header[16], header[17] = 0, 0                       // flags
	copy(header[20:32], []byte("qrd-0.1.0\x00\x00\x00"))

	tmpFile.Write(header)
	tmpFile.Close()

	reader, err := NewFileReader(tmpFile.Name(), nil)
	if err != nil {
		t.Fatal(err)
	}

	result, err := reader.InspectHeader()
	if err != nil {
		t.Fatal(err)
	}

	if result.FormatMajor != 1 {
		t.Errorf("expected format_major=1, got %d", result.FormatMajor)
	}

	if result.FormatMinor != 0 {
		t.Errorf("expected format_minor=0, got %d", result.FormatMinor)
	}
}

func TestInspectHeaderRejectsBadMagic(t *testing.T) {
	tmpFile, err := os.CreateTemp("", "test_*.qrd")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(tmpFile.Name())

	// Write invalid magic
	header := make([]byte, 32)
	header[0], header[1], header[2] = 0x58, 0x59, 0x5A // "XYZ"

	tmpFile.Write(header)
	tmpFile.Close()

	reader, err := NewFileReader(tmpFile.Name(), nil)
	if err != nil {
		t.Fatal(err)
	}

	_, err = reader.InspectHeader()
	if err == nil {
		t.Error("should reject invalid magic bytes")
	}
}

func TestNewFileWriter(t *testing.T) {
	tmpFile, err := os.CreateTemp("", "write_*.qrd")
	if err != nil {
		t.Fatal(err)
	}
	tmpFile.Close()
	defer os.Remove(tmpFile.Name())

	schema := Schema{
		"id":    "int32",
		"value": "float32",
	}

	writer, err := NewFileWriter(tmpFile.Name(), schema)
	if err != nil {
		t.Fatal(err)
	}

	if len(writer.schema) != 2 {
		t.Errorf("expected schema length 2, got %d", len(writer.schema))
	}
}

func TestNewFileWriterRejectsEmptySchema(t *testing.T) {
	tmpFile, err := os.CreateTemp("", "write_*.qrd")
	if err != nil {
		t.Fatal(err)
	}
	tmpFile.Close()
	defer os.Remove(tmpFile.Name())

	_, err = NewFileWriter(tmpFile.Name(), Schema{})
	if err == nil {
		t.Error("should reject empty schema")
	}
}

func TestFileWriterWriteRow(t *testing.T) {
	tmpFile, err := os.CreateTemp("", "write_*.qrd")
	if err != nil {
		t.Fatal(err)
	}
	tmpFile.Close()
	defer os.Remove(tmpFile.Name())

	schema := Schema{"id": "int32", "name": "utf8"}
	writer, err := NewFileWriter(tmpFile.Name(), schema)
	if err != nil {
		t.Fatal(err)
	}

	// Valid row
	err = writer.WriteRow(Row{"id": 1, "name": "test"})
	if err != nil {
		t.Fatal(err)
	}

	// Row with missing field
	err = writer.WriteRow(Row{"id": 2})
	if err == nil {
		t.Error("should reject row with missing field")
	}
}

func TestFileWriterFinish(t *testing.T) {
	tmpFile, err := os.CreateTemp("", "write_*.qrd")
	if err != nil {
		t.Fatal(err)
	}
	tmpFile.Close()
	defer os.Remove(tmpFile.Name())

	schema := Schema{"id": "int32"}
	writer, err := NewFileWriter(tmpFile.Name(), schema)
	if err != nil {
		t.Fatal(err)
	}

	writer.WriteRow(Row{"id": 1})

	// First finish should succeed
	err = writer.Finish()
	if err != nil {
		t.Fatal(err)
	}

	// Second finish should fail
	err = writer.Finish()
	if err == nil {
		t.Error("should reject duplicate finish")
	}
}

func TestFileWriterWriteAfterFinish(t *testing.T) {
	tmpFile, err := os.CreateTemp("", "write_*.qrd")
	if err != nil {
		t.Fatal(err)
	}
	tmpFile.Close()
	defer os.Remove(tmpFile.Name())

	schema := Schema{"id": "int32"}
	writer, err := NewFileWriter(tmpFile.Name(), schema)
	if err != nil {
		t.Fatal(err)
	}

	writer.Finish()

	err = writer.WriteRow(Row{"id": 1})
	if err == nil {
		t.Error("should reject write after finish")
	}
}

func TestConvenienceFunctions(t *testing.T) {
	tmpFile, err := os.CreateTemp("", "test_*.qrd")
	if err != nil {
		t.Fatal(err)
	}

	// Write valid header
	header := make([]byte, 32)
	header[0], header[1], header[2] = 0x51, 0x52, 0x44

	tmpFile.Write(header)
	tmpFile.Close()
	defer os.Remove(tmpFile.Name())

	// Test InspectHeader convenience function
	result, err := InspectHeader(tmpFile.Name())
	if err != nil {
		t.Fatal(err)
	}

	if result == nil {
		t.Error("expected non-nil header")
	}
}
