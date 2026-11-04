package dev.horizon.android;

import android.content.Context;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.Environment;
import java.io.File;

/**
 * Decides where Horizon keeps its data.
 *
 * Open build: when "All files access" (Android 11+) or storage permission (Android 9-10) is
 * granted, the data lives in shared storage so any file manager, USB cable or PC can see it.
 * Otherwise it falls back to the app's private folder, exactly like the original build.
 */
final class HorizonHome {
    /** PATCH THIS: folder name created in the root of shared storage (/storage/emulated/0/<name>). */
    static final String EXTERNAL_DIR = "Horizon";
    /** Rust reads the chosen folder from this file inside the private folder. */
    static final String POINTER_FILE = "home-path.txt";

    private HorizonHome() {}

    static boolean hasStorageAccess(Context context) {
        if (Build.VERSION.SDK_INT >= 30) return Environment.isExternalStorageManager();
        return context.checkSelfPermission(android.Manifest.permission.WRITE_EXTERNAL_STORAGE)
                == PackageManager.PERMISSION_GRANTED;
    }

    static File dir(Context context) {
        if (hasStorageAccess(context)) {
            File shared = new File(Environment.getExternalStorageDirectory(), EXTERNAL_DIR);
            if ((shared.isDirectory() || shared.mkdirs()) && shared.canWrite()) return shared;
        }
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
