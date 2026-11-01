#!/usr/bin/env python3
"""Build a tiny isolated SAF provider probe, never the game or its player data."""
from pathlib import Path
import argparse
import os
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]

JAVA = r'''
package dev.horizon.storageprobe;
import android.app.Activity;
import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.database.Cursor;
import android.net.Uri;
import android.provider.DocumentsContract;
import android.util.Log;
import java.io.*;
public final class Probe extends Activity {
    private static native void nativeCheck(String home);
    private void check(boolean ok, String what) { if (!ok) throw new AssertionError(what); }
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        try {
            System.loadLibrary("horizon_android");
            nativeCheck(getFilesDir().getAbsolutePath());
            String authority = getPackageName() + ".documents";
            Uri roots = DocumentsContract.buildRootsUri(authority);
            try (Cursor c = getContentResolver().query(roots, null, null, null, null)) {
                check(c != null && c.moveToFirst(), "root");
            }
            Uri mods = DocumentsContract.buildDocumentUri(authority, "p0/mods");
            Uri children = DocumentsContract.buildChildDocumentsUri(authority, "horizon");
            try (Cursor c = getContentResolver().query(children, null, null, null, null)) {
                check(c != null && c.getCount() == 5, "five directories");
            }
            Uri file = DocumentsContract.createDocument(getContentResolver(), mods, "application/zip", "probe.hmod");
            check(file != null, "create");
            try (OutputStream s = getContentResolver().openOutputStream(file, "wt")) { s.write(new byte[]{1,2,3}); }
            try (InputStream s = getContentResolver().openInputStream(file)) {
                check(s.read() == 1 && s.read() == 2 && s.read() == 3 && s.read() == -1, "round trip");
            }
            Uri renamed = DocumentsContract.renameDocument(getContentResolver(), file, "renamed.hmod");
            check(renamed != null, "rename");
            DocumentsContract.Path path = DocumentsContract.findDocumentPath(getContentResolver(),
                    DocumentsContract.buildDocumentUriUsingTree(
                        DocumentsContract.buildTreeDocumentUri(authority, "p0/mods"), "p0/mods"));
            check(path != null && path.getPath().contains("p0/mods"), "navigation path");
            check(DocumentsContract.deleteDocument(getContentResolver(), renamed), "delete");
            Uri worlds = DocumentsContract.buildDocumentUri(authority, "p0/worlds");
            Uri world = DocumentsContract.createDocument(getContentResolver(), worlds, DocumentsContract.Document.MIME_TYPE_DIR, "World Transfer Probe");
            check(world != null, "create world folder");
            Uri level = DocumentsContract.createDocument(getContentResolver(), world, "application/octet-stream", "level.dat");
            try (OutputStream s = getContentResolver().openOutputStream(level, "wt")) { s.write(new byte[]{9,8,7}); }
            try (InputStream s = getContentResolver().openInputStream(level)) { check(s.read() == 9 && s.read() == 8 && s.read() == 7, "world file round trip"); }
            check(DocumentsContract.deleteDocument(getContentResolver(), level), "delete fixture level");
            check(DocumentsContract.deleteDocument(getContentResolver(), world), "delete fixture world");
            Uri secret = DocumentsContract.buildDocumentUri(authority, "p0/mods/../android-launcher.json");
            boolean denied = false;
            try (ParcelFileDescriptor fd = getContentResolver().openFileDescriptor(secret, "r")) {}
            catch (FileNotFoundException error) { denied = true; }
            check(denied, "traversal denied");
            Log.i("HorizonStorageProbe", "PASS roots, create, write, read, rename, delete, traversal and Bionic flock");
            android.widget.TextView text = new android.widget.TextView(this);
            text.setText("Horizon storage probe: PASS"); setContentView(text);
            if (getIntent().getBooleanExtra("browse", false)) {
                android.content.Intent view = new android.content.Intent(android.content.Intent.ACTION_VIEW);
                view.setDataAndType(mods, DocumentsContract.Document.MIME_TYPE_DIR);
                view.addFlags(3); startActivity(view);
            }
        } catch (Throwable error) { Log.e("HorizonStorageProbe", "FAIL", error); }
    }
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--serial', help='Install only on this ADB device; omitted means build only')
    parser.add_argument('--browse', action='store_true')
    args = parser.parse_args()
    sdk = Path(os.environ.get('ANDROID_HOME', Path.home() / 'AppData/Local/Android/Sdk'))
    jdk = Path(os.environ.get('JAVA_HOME', 'C:/Program Files/Android/Android Studio/jbr'))
    bt = sdk / 'build-tools/36.1.0'
    ndk = sdk / 'ndk/28.2.13676358/toolchains/llvm/prebuilt/windows-x86_64/bin'
    out = ROOT / '.local/android/storage-probe'
    (out / 'src').mkdir(parents=True, exist_ok=True)
    (out / 'classes').mkdir(exist_ok=True)
    (out / 'dex').mkdir(exist_ok=True)
    (out / 'Cargo.toml').write_text('''[package]
name = "horizon-storage-probe"
version = "0.0.0"
edition = "2024"
[lib]
name = "horizon_android"
crate-type = ["cdylib"]
[workspace]
[dependencies]
jni = "0.21"
serde_json = "1"
rustix = { version = "1", features = ["fs"] }
''', encoding='utf-8')
    source = ROOT / 'crates/horizon-client/src'
    modules = '\n'.join(f'#[path = "{(source / path).as_posix()}"] mod {name};'
        for name, path in [('document_provider', 'android/document_provider.rs'),
                           ('document_store', 'android/document_store.rs'), ('file_lock', '../../horizon-client-render/src/file_lock.rs')])
    (out / 'src/lib.rs').write_text('''#![allow(dead_code)]
''' + modules + r'''
#[unsafe(no_mangle)]
extern "system" fn Java_dev_horizon_storageprobe_Probe_nativeCheck(
    mut env: jni::JNIEnv, _: jni::objects::JClass, home: jni::objects::JString,
) {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let home: String = env.get_string(&home)?.into();
        let home = std::path::PathBuf::from(home);
        let file = std::fs::File::create(home.join("lock-probe"))?;
        file_lock::exclusive(&file)?;
        let second = std::fs::OpenOptions::new().read(true).write(true).open(home.join("lock-probe"))?;
        let (sender, receiver) = std::sync::mpsc::channel();
        let waiter = std::thread::spawn(move || {
            let result = file_lock::exclusive(&second);
            let _ = sender.send(result);
        });
        if receiver.recv_timeout(std::time::Duration::from_millis(50)).is_ok() {
            return Err("second cache writer did not wait for the file lock".into());
        }
        drop(file);
        receiver.recv_timeout(std::time::Duration::from_secs(2))??;
        waiter.join().map_err(|_| "cache-lock waiter failed")?;
        std::fs::write(home.join("android-launcher.json"), "{}")?;
        let mods = home.join("horizon-mods");
        std::fs::create_dir_all(&mods)?;
        let _ = std::fs::remove_file(mods.join("probe.hmod"));
        let _ = std::fs::remove_file(mods.join("renamed.hmod"));
        let link = mods.join("escape");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&home, &link)?;
        if document_store::call(&home, "document", "p0/mods/escape/android-launcher.json", "").is_ok() {
            return Err("symlink was exposed".into());
        }
        std::fs::remove_file(link)?;
        Ok(())
    })();
    if let Err(error) = result { let _ = env.throw_new("java/lang/AssertionError", error.to_string()); }
}
''', encoding='utf-8')
    (out / 'Probe.java').write_text(JAVA, encoding='utf-8')
    manifest = '''<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="dev.horizon.storageprobe">
<uses-sdk android:minSdkVersion="28" android:targetSdkVersion="35"/>
<application android:debuggable="true" android:label="Horizon Storage Probe" android:extractNativeLibs="true">
<provider android:name="dev.horizon.android.HorizonDocumentsProvider" android:authorities="dev.horizon.storageprobe.documents" android:exported="true" android:grantUriPermissions="true" android:permission="android.permission.MANAGE_DOCUMENTS"><intent-filter><action android:name="android.content.action.DOCUMENTS_PROVIDER"/></intent-filter></provider>
<activity android:name="dev.horizon.storageprobe.Probe" android:exported="true"/>
</application></manifest>'''
    (out / 'AndroidManifest.xml').write_text(manifest, encoding='utf-8')
    env = os.environ.copy()
    env['CARGO_TARGET_DIR'] = str(out / 'target')
    env['CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER'] = str(ndk / 'aarch64-linux-android28-clang.cmd')
    def run(command, **kwargs):
        subprocess.run([str(v) for v in command], check=True, cwd=ROOT, **kwargs)
    run(['cargo', 'build', '--manifest-path', out / 'Cargo.toml', '--target', 'aarch64-linux-android'], env=env)
    run([jdk / 'bin/javac.exe', '-source', '8', '-target', '8', '-classpath', sdk / 'platforms/android-35/android.jar',
         '-d', out / 'classes', out / 'Probe.java', ROOT / 'packaging/android/java/dev/horizon/android/HorizonDocumentsProvider.java'])
    classes = sorted((out / 'classes').rglob('*.class'))
    run([jdk / 'bin/java.exe', '-cp', bt / 'lib/d8.jar', 'com.android.tools.r8.D8', '--min-api', '28',
         '--lib', sdk / 'platforms/android-35/android.jar', '--output', out / 'dex', *classes])
    unsigned = out / 'unsigned.apk'
    run([bt / 'aapt2.exe', 'link', '-I', sdk / 'platforms/android-35/android.jar', '--manifest', out / 'AndroidManifest.xml', '-o', unsigned])
    with zipfile.ZipFile(unsigned, 'a') as archive:
        archive.write(out / 'dex/classes.dex', 'classes.dex')
        archive.write(out / 'target/aarch64-linux-android/debug/libhorizon_android.so', 'lib/arm64-v8a/libhorizon_android.so')
    apk = out / 'probe.apk'
    run([bt / 'zipalign.exe', '-f', '4', unsigned, apk])
    run([jdk / 'bin/java.exe', '-jar', bt / 'lib/apksigner.jar', 'sign', '--ks', ROOT / '.local/android/debug.keystore',
         '--ks-key-alias', 'androiddebugkey', '--ks-pass', 'pass:android', apk])
    if args.serial:
        adb = sdk / 'platform-tools/adb.exe'
        run([adb, '-s', args.serial, 'install', '--no-incremental', '-r', apk])
        run([adb, '-s', args.serial, 'shell', 'am', 'start', '-n', 'dev.horizon.storageprobe/.Probe', '--ez', 'browse', str(args.browse).lower()])
    print(apk)


if __name__ == '__main__':
    main()
