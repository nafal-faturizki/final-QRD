package dev.qrd;

/**
 * QRD Java SDK scaffold.
 *
 * The final implementation will bridge to the Rust core through JNI.
 */
public final class Qrd {
    private Qrd() {
    }

    public static final class FileReader {
        private final String path;

        public FileReader(String path) {
            this.path = path;
        }

        public String path() {
            return path;
        }

        public String inspectHeader() {
            throw new UnsupportedOperationException("JNI binding not added yet");
        }

        public String inspectFooter() {
            throw new UnsupportedOperationException("JNI binding not added yet");
        }
    }

    public static final class FileWriter {
        private final String path;

        public FileWriter(String path) {
            this.path = path;
        }

        public String path() {
            return path;
        }

        public void writeRow(java.util.Map<String, Object> row) {
            throw new UnsupportedOperationException("JNI binding not added yet for writer at " + path);
        }

        public void finish() {
            throw new UnsupportedOperationException("JNI binding not added yet for writer at " + path);
        }
    }
}
