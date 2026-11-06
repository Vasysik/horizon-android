package dev.horizon.android;

/** SAF result callback. Broad storage access is not requested; user-selected documents use SAF. */
public final class HorizonActivity extends android.app.NativeActivity {
    static { System.loadLibrary("horizon_android"); }
    private native void nativeDocumentResult(int request, int result, String uri);

    @Override protected void onCreate(android.os.Bundle state) {
        // Tell Rust where the data folder is before the native code starts.
        HorizonHome.publish(this);
        super.onCreate(state);
    }

    @Override protected void onActivityResult(int request, int result, android.content.Intent data) {
        super.onActivityResult(request, result, data);
        if (request != 4101) return;
        nativeDocumentResult(request, result,
                data == null || data.getData() == null ? null : data.getData().toString());
    }
}
