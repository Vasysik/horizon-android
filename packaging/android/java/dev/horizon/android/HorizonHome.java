package dev.horizon.android;

import android.content.Context;
import java.io.File;

/**
 * Horizon keeps its data in the app-private folder. User-selected file imports and exports use SAF.
 */
final class HorizonHome {
    /** Rust reads the data folder from this file inside the private folder. */
    static final String POINTER_FILE = "home-path.txt";

    private HorizonHome() {}

    static File dir(Context context) {
        return context.getFilesDir();
    }

    /** Called before the native code starts so Rust and the DocumentsProvider agree. */
    static void publish(Context context) {
        try {
            File pointer = new File(context.getFilesDir(), POINTER_FILE);
            File chosen = dir(context);
            try (java.io.FileOutputStream out = new java.io.FileOutputStream(pointer, false)) {
                out.write(chosen.getAbsolutePath().getBytes(java.nio.charset.StandardCharsets.UTF_8));
            }
        } catch (java.io.IOException ignored) {
            // Rust falls back to the private folder when the pointer is missing.
        }
    }
}
