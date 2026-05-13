package dev.qrd;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.HashMap;
import java.util.Map;

/**
 * Unit tests for QRD Java SDK.
 */
public final class QrdSmoke {
    private QrdSmoke() {}

    public static void main(String[] args) throws Exception {
        testFileReaderPathValidation();
        testFileWriterSchemaValidation();
        testFileWriterRowSchema();
        testFileWriterFinish();
        testConvenienceFunctions();
        System.out.println("All smoke tests passed!");
    }

    private static void testFileReaderPathValidation() throws IOException {
        // Non-existent file should throw
        try {
            new Qrd.FileReader("/nonexistent/path/file.qrd", null);
            throw new AssertionError("Should reject non-existent file");
        } catch (IllegalArgumentException e) {
            assert e.getMessage().contains("not found");
        }

        // File too short should throw
        Path tmpFile = Files.createTempFile("test_", ".qrd");
        try {
            Files.write(tmpFile, "SHORT".getBytes());
            new Qrd.FileReader(tmpFile.toString(), null);
            throw new AssertionError("Should reject short file");
        } catch (IllegalArgumentException e) {
            assert e.getMessage().contains("too short");
        } finally {
            Files.delete(tmpFile);
        }
    }

    private static void testFileWriterSchemaValidation() {
        // Empty schema should throw
        try {
            new Qrd.FileWriter("/tmp/out.qrd", new HashMap<>());
            throw new AssertionError("Should reject empty schema");
        } catch (IllegalArgumentException e) {
            assert e.getMessage().contains("empty");
        }

        // Valid schema should succeed
        Map<String, String> schema = new HashMap<>();
        schema.put("id", "int32");
        schema.put("value", "float32");
        Qrd.FileWriter writer = new Qrd.FileWriter("/tmp/out.qrd", schema);
        assert writer.path().equals("/tmp/out.qrd");
    }

    private static void testFileWriterRowSchema() {
        Map<String, String> schema = new HashMap<>();
        schema.put("id", "int32");
        schema.put("name", "utf8");

        Qrd.FileWriter writer = new Qrd.FileWriter("/tmp/test.qrd", schema);

        // Valid row should succeed
        Map<String, Object> validRow = new HashMap<>();
        validRow.put("id", 1);
        validRow.put("name", "test");
        writer.writeRow(validRow);

        // Row with missing field should throw
        try {
            Map<String, Object> invalidRow = new HashMap<>();
            invalidRow.put("id", 2);
            writer.writeRow(invalidRow);
            throw new AssertionError("Should reject row with missing field");
        } catch (IllegalArgumentException e) {
            assert e.getMessage().contains("Missing required field");
        }
    }

    private static void testFileWriterFinish() throws IOException {
        Map<String, String> schema = new HashMap<>();
        schema.put("id", "int32");

        Path tmpFile = Files.createTempFile("test_", ".qrd");
        try {
            Qrd.FileWriter writer = new Qrd.FileWriter(tmpFile.toString(), schema);

            // First finish should succeed
            writer.finish();

            // Second finish should fail
            try {
                writer.finish();
                throw new AssertionError("Should reject duplicate finish");
            } catch (IllegalStateException e) {
                assert e.getMessage().contains("already finished");
            }

            // Write after finish should fail
            try {
                Map<String, Object> row = new HashMap<>();
                row.put("id", 1);
                writer.writeRow(row);
                throw new AssertionError("Should reject write after finish");
            } catch (IllegalStateException e) {
                assert e.getMessage().contains("already finished");
            }
        } finally {
            Files.deleteIfExists(tmpFile);
        }
    }

    private static void testConvenienceFunctions() throws IOException {
        // Create temp file with valid header
        Path tmpFile = Files.createTempFile("test_", ".qrd");
        try {
            byte[] header = new byte[32];
            header[0] = 0x51; // 'Q'
            header[1] = 0x52; // 'R'
            header[2] = 0x44; // 'D'
            header[4] = 1;    // format_major = 1
            header[6] = 0;    // format_minor = 0

            Files.write(tmpFile, header);

            // Test convenience function
            Qrd.Header headerInfo = Qrd.inspectHeader(tmpFile.toString());
            assert headerInfo.formatMajor == 1;
            assert headerInfo.formatMinor == 0;
        } finally {
            Files.delete(tmpFile);
        }
    }
}
