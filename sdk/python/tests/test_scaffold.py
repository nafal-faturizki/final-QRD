"""Test suite for QRD Python SDK.

Tests cover:
- File reading and inspection
- Header/footer parsing
- Schema validation
- Error handling
"""

import pytest
import tempfile
from pathlib import Path
from qrd import FileReader, FileWriter, inspect_footer, inspect_header


class TestFileReader:
    """Tests for FileReader class."""

    def test_reader_validates_file_exists(self):
        """Reader should raise FileNotFoundError for non-existent files."""
        with pytest.raises(FileNotFoundError):
            FileReader("/nonexistent/path/file.qrd")

    def test_reader_requires_valid_header(self):
        """Reader should reject files with invalid headers."""
        with tempfile.NamedTemporaryFile(suffix=".qrd", delete=False) as f:
            # Write invalid header (too short)
            f.write(b"INVALID")
            f.flush()
            temp_path = f.name
        
        try:
            reader = FileReader(temp_path)
            with pytest.raises(ValueError, match="too short"):
                reader.inspect_header()
        finally:
            Path(temp_path).unlink()

    def test_reader_rejects_bad_magic_bytes(self):
        """Reader should reject files without QRD magic bytes."""
        with tempfile.NamedTemporaryFile(suffix=".qrd", delete=False) as f:
            # Write 32 bytes with wrong magic
            f.write(b"XXXX" + b"\x00" * 28)
            f.flush()
            temp_path = f.name
        
        try:
            reader = FileReader(temp_path)
            with pytest.raises(ValueError, match="Invalid QRD magic bytes"):
                reader.inspect_header()
        finally:
            Path(temp_path).unlink()

    def test_reader_parses_valid_header(self):
        """Reader should parse valid QRD headers correctly."""
        with tempfile.NamedTemporaryFile(suffix=".qrd", delete=False) as f:
            # Write valid header with QRD magic bytes
            header = bytearray(32)
            header[0:4] = b"QRD\x00"  # Magic
            header[4:6] = (1).to_bytes(2, "little")  # format_major
            header[6:8] = (0).to_bytes(2, "little")  # format_minor
            header[8:16] = bytes(range(8))  # schema_id
            header[16:18] = (0).to_bytes(2, "little")  # flags
            header[20:32] = b"qrd-0.1.0\x00\x00\x00"
            
            f.write(bytes(header))
            f.flush()
            temp_path = f.name
        
        try:
            reader = FileReader(temp_path)
            header_data = reader.inspect_header()
            
            assert header_data["format_major"] == 1
            assert header_data["format_minor"] == 0
            assert header_data["schema_id"] == list(range(8))
            assert "writer_version" in header_data
        finally:
            Path(temp_path).unlink()


class TestFileWriter:
    """Tests for FileWriter class."""

    def test_writer_accepts_schema(self):
        """Writer should accept valid schema."""
        schema = {
            "device_id": "utf8",
            "temperature": "float32",
            "humidity": "float32",
        }
        with tempfile.TemporaryDirectory() as tmpdir:
            writer = FileWriter(f"{tmpdir}/output.qrd", schema)
            assert writer.schema == schema

    def test_writer_rejects_duplicate_finish(self):
        """Writer should reject multiple finish() calls."""
        schema = {"id": "int32"}
        with tempfile.TemporaryDirectory() as tmpdir:
            writer = FileWriter(f"{tmpdir}/output.qrd", schema)
            writer.finish()
            
            with pytest.raises(RuntimeError, match="already finished"):
                writer.finish()

    def test_writer_rejects_write_after_finish(self):
        """Writer should reject writes after finish()."""
        schema = {"id": "int32"}
        with tempfile.TemporaryDirectory() as tmpdir:
            writer = FileWriter(f"{tmpdir}/output.qrd", schema)
            writer.finish()
            
            with pytest.raises(RuntimeError, match="already finished"):
                writer.write_row({"id": 1})

    def test_writer_validates_row_schema(self):
        """Writer should validate rows against schema."""
        schema = {"id": "int32", "name": "utf8"}
        with tempfile.TemporaryDirectory() as tmpdir:
            writer = FileWriter(f"{tmpdir}/output.qrd", schema)
            
            # Missing required field
            with pytest.raises(ValueError, match="missing field"):
                writer.write_row({"id": 1})

    def test_writer_accepts_valid_rows(self):
        """Writer should accept rows matching schema."""
        schema = {"id": "int32", "value": "float32"}
        with tempfile.TemporaryDirectory() as tmpdir:
            path = f"{tmpdir}/output.qrd"
            writer = FileWriter(path, schema)
            
            writer.write_row({"id": 1, "value": 3.14})
            writer.write_row({"id": 2, "value": 2.71})
            
            assert writer._row_count == 2
            assert len(writer._current_row_group) == 2


class TestConvenienceFunctions:
    """Tests for module-level convenience functions."""

    def test_inspect_header_convenience(self):
        """inspect_header() should work like FileReader.inspect_header()."""
        with tempfile.NamedTemporaryFile(suffix=".qrd", delete=False) as f:
            # Write valid header
            header = bytearray(32)
            header[0:4] = b"QRD\x00"
            header[4:6] = (1).to_bytes(2, "little")
            header[6:8] = (0).to_bytes(2, "little")
            
            f.write(bytes(header))
            f.flush()
            temp_path = f.name
        
        try:
            result = inspect_header(temp_path)
            assert result["format_major"] == 1
            assert result["format_minor"] == 0
        finally:
            Path(temp_path).unlink()


class TestErrorHandling:
    """Tests for error conditions."""

    def test_reader_handles_empty_file(self):
        """Reader should handle empty files gracefully."""
        with tempfile.NamedTemporaryFile(suffix=".qrd", delete=False) as f:
            f.flush()
            temp_path = f.name
        
        try:
            reader = FileReader(temp_path)
            with pytest.raises(ValueError, match="too short"):
                reader.inspect_header()
        finally:
            Path(temp_path).unlink()

    def test_writer_creates_output_file(self):
        """Writer should create output file on finish()."""
        schema = {"id": "int32"}
        with tempfile.TemporaryDirectory() as tmpdir:
            path = f"{tmpdir}/test_output.qrd"
            writer = FileWriter(path, schema)
            writer.finish()
            
            assert Path(path).exists()
