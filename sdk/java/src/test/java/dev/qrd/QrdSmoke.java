package dev.qrd;

/**
 * Minimal Java smoke scaffold for QRD.
 */
public final class QrdSmoke {
    private QrdSmoke() {
    }

    public static void main(String[] args) {
        Qrd.FileReader reader = new Qrd.FileReader("example.qrd");
        Qrd.FileWriter writer = new Qrd.FileWriter("output.qrd");

        System.out.println(reader.path());
        System.out.println(writer.getClass().getSimpleName());
    }
}
