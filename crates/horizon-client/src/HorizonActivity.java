package dev.horizon.android;

/** SAF result callback and storage-access prompt. The native activity, UI and file operations stay Rust. */
public final class HorizonActivity extends android.app.NativeActivity {
    static { System.loadLibrary("horizon_android"); }
    private static final int STORAGE_REQUEST = 4102;
    private native void nativeDocumentResult(int request, int result, String uri);

    @Override protected void onCreate(android.os.Bundle state) {
        // Tell Rust where the data folder is before the native code starts.
        HorizonHome.publish(this);
        super.onCreate(state);
        if (!HorizonHome.hasStorageAccess(this)) requestStorageAccess();
    }

    private void requestStorageAccess() {
        try {
            if (android.os.Build.VERSION.SDK_INT >= 30) {
                android.content.Intent intent = new android.content.Intent(
                        android.provider.Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                        android.net.Uri.parse("package:" + getPackageName()));
                startActivityForResult(intent, STORAGE_REQUEST);
            } else {
                requestPermissions(new String[] {
                    android.Manifest.permission.READ_EXTERNAL_STORAGE,
                    android.Manifest.permission.WRITE_EXTERNAL_STORAGE
                }, STORAGE_REQUEST);
            }
        } catch (RuntimeException error) {
            // The device has no such settings screen; keep using the private folder.
        }
    }

    /** The data folder only changes at process start, so restart once access was granted. */
    private void restartIfGranted() {
        if (!HorizonHome.hasStorageAccess(this)) return;
        HorizonHome.publish(this);
        android.widget.Toast.makeText(this,
                "Storage access granted. Reopen Horizon to use /" + HorizonHome.EXTERNAL_DIR,
                android.widget.Toast.LENGTH_LONG).show();
        finishAffinity();
        new android.os.Handler(android.os.Looper.getMainLooper()).postDelayed(
                () -> android.os.Process.killProcess(android.os.Process.myPid()), 1500);
    }

    @Override public void onRequestPermissionsResult(int request, String[] permissions, int[] results) {
        super.onRequestPermissionsResult(request, permissions, results);
        if (request == STORAGE_REQUEST) restartIfGranted();
    }

    @Override protected void onActivityResult(int request, int result, android.content.Intent data) {
        super.onActivityResult(request, result, data);
        if (request == STORAGE_REQUEST) {
            restartIfGranted();
            return;
        }
        if (request != 4101) return;
        nativeDocumentResult(request, result,
                data == null || data.getData() == null ? null : data.getData().toString());
    }
}
