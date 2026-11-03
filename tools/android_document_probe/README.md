# Android document result probe

Small standalone Rust NativeActivity regression probe. It imports the production
JNI registration, system-picker code, document reader and archive validator by
path; it does not link the game or touch player data.

Build this Cargo manifest for `aarch64-linux-android` with the NDK linker and a
throwaway target directory. Package the resulting `libhorizon_document_probe.so`
with the original `packaging/android/java/dev/horizon/android/HorizonActivity.java`
compiled to DEX. Use a separate application ID, e.g. `dev.horizon.documentprobe`,
that activity as its launcher, and `android.app.lib_name=horizon_document_probe`.
Do not replace the installed game. Keep generated classes, DEX and APKs in `.local/`.

On startup the probe invokes both native-method signatures through Java dispatch,
then opens the production system picker. Select a small valid resource ZIP,
`.hmod`, or `.hshader`; the probe reads it through ContentResolver, copies it to
its own cache, validates it and deletes the temporary copy. Restart the probe and
press Back in the picker to test real cancellation. Look for:

```text
PASS Java dispatch of cancel + success callbacks
PASS provider read + copy + validation: ...
PASS cancel callback
```

The synthetic success callback uses an unavailable provider URI; its read should
fail without terminating the process. It is separate from the selected-file test.
Logs use the `HorizonDocumentProbe` tag. Neither `System.loadLibrary` nor exported
JNI symbol lookup is used: production explicitly registers the callback on the
Activity class because NativeActivity's `dlopen` is outside Java's library lookup.

This covers the bridge and file reader, not the full game's Installed UI or import
collision dialogs. The shared host archive tests cover validation and size limits.
