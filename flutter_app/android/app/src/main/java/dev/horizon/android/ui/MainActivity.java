package dev.horizon.android.ui;

import android.Manifest;
import android.app.Activity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.BitmapFactory;
import android.net.Uri;
import android.os.Build;

import io.flutter.embedding.android.FlutterActivity;
import io.flutter.embedding.engine.FlutterEngine;
import io.flutter.plugin.common.MethodChannel;

import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.HashMap;
import java.util.Map;

public final class MainActivity extends FlutterActivity {
    private static final String CHANNEL = "dev.horizon.ui/bridge";
    private static final int NOTIFICATION_PERMISSION = 71;
    private static final int PICK_SKIN = 72;
    private MethodChannel.Result pendingSkinResult;

    @Override
    public void configureFlutterEngine(FlutterEngine flutterEngine) {
        super.configureFlutterEngine(flutterEngine);
        MethodChannel channel = new MethodChannel(
                flutterEngine.getDartExecutor().getBinaryMessenger(), CHANNEL);
        channel.setMethodCallHandler((call, result) -> {
            try {
                switch (call.method) {
                    case "paths": {
                        Map<String, Object> paths = new HashMap<>();
                        paths.put("filesDir", getFilesDir().getAbsolutePath());
                        paths.put("cacheDir", getCacheDir().getAbsolutePath());
                        paths.put("versionName", getPackageManager()
                                .getPackageInfo(getPackageName(), 0).versionName);
                        result.success(paths);
                        break;
                    }
                    case "prepareEngineData":
                        prepareEngineData();
                        result.success(null);
                        break;
                    case "startDownloadService": {
                        String statusPath = call.argument("statusPath");
                        maybeRequestNotifications();
                        startDownloadService(statusPath);
                        result.success(null);
                        break;
                    }
                    case "stopDownloadService":
                        stopService(new Intent(this, DownloadService.class));
                        result.success(null);
                        break;
                    case "requestNotificationPermission":
                        maybeRequestNotifications();
                        result.success(null);
                        break;
                    case "launchGame": {
                        Intent intent = new Intent(this, dev.horizon.android.HorizonActivity.class);
                        intent.putExtra("horizon.direct_play", "1");
                        startActivity(intent);
                        overridePendingTransition(android.R.anim.fade_in, android.R.anim.fade_out);
                        result.success(null);
                        break;
                    }
                    case "openUrl": {
                        String url = call.argument("url");
                        if (url != null) {
                            startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(url)));
                        }
                        result.success(null);
                        break;
                    }
                    case "pickSkin":
                        pickSkin(result);
                        break;
                    case "clearSkin":
                        clearSkin();
                        result.success(null);
                        break;
                    default:
                        result.notImplemented();
                }
            } catch (Throwable error) {
                result.error("horizon_bridge", error.getMessage(), null);
            }
        });
    }

    private void prepareEngineData() throws Exception {
        System.loadLibrary("horizon_android");
        File target = new File(getFilesDir(), "engine-data.zip");
        File partial = new File(getFilesDir(), "engine-data.partial");
        try (InputStream input = getAssets().open("engine-data.zip");
             OutputStream output = new FileOutputStream(partial)) {
            byte[] buffer = new byte[65536];
            int count;
            while ((count = input.read(buffer)) != -1) {
                output.write(buffer, 0, count);
            }
        }
        if (target.exists() && !target.delete()) {
            throw new IllegalStateException("engine-data.zip could not be replaced");
        }
        if (!partial.renameTo(target)) {
            throw new IllegalStateException("engine-data.zip could not be prepared");
        }
    }

    private void pickSkin(MethodChannel.Result result) {
        if (pendingSkinResult != null) {
            result.error("skin_picker_busy", "A skin picker is already open", null);
            return;
        }
        pendingSkinResult = result;
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.setType("image/png");
        startActivityForResult(intent, PICK_SKIN);
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != PICK_SKIN) {
            return;
        }
        MethodChannel.Result result = pendingSkinResult;
        pendingSkinResult = null;
        if (result == null) {
            return;
        }
        if (resultCode != Activity.RESULT_OK || data == null || data.getData() == null) {
            result.success(null);
            return;
        }
        File target = new File(getFilesDir(), "horizon-skin.png");
        File partial = new File(getFilesDir(), "horizon-skin.partial");
        try (InputStream input = getContentResolver().openInputStream(data.getData());
             OutputStream output = new FileOutputStream(partial)) {
            if (input == null) {
                throw new IllegalStateException("The selected file could not be opened");
            }
            byte[] buffer = new byte[16384];
            long total = 0;
            int count;
            while ((count = input.read(buffer)) != -1) {
                total += count;
                if (total > 1024 * 1024) {
                    throw new IllegalStateException("The skin is larger than 1 MiB");
                }
                output.write(buffer, 0, count);
            }
        } catch (Throwable error) {
            partial.delete();
            result.error("invalid_skin", error.getMessage(), null);
            return;
        }
        if (target.exists() && !target.delete()) {
            partial.delete();
            result.error("invalid_skin", "The old skin could not be replaced", null);
            return;
        }
        if (!partial.renameTo(target)) {
            partial.delete();
            result.error("invalid_skin", "The skin could not be saved", null);
            return;
        }
        BitmapFactory.Options options = new BitmapFactory.Options();
        options.inJustDecodeBounds = true;
        BitmapFactory.decodeFile(target.getAbsolutePath(), options);
        if (!(options.outWidth == 64 && (options.outHeight == 64 || options.outHeight == 32))) {
            target.delete();
            result.error("invalid_skin", "Skin must be a 64x64 or 64x32 PNG", null);
            return;
        }
        result.success(target.getAbsolutePath());
    }

    private void clearSkin() {
        File target = new File(getFilesDir(), "horizon-skin.png");
        if (target.exists()) {
            target.delete();
        }
    }

    private void maybeRequestNotifications() {
        if (Build.VERSION.SDK_INT >= 33
                && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)
                != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[]{Manifest.permission.POST_NOTIFICATIONS},
                    NOTIFICATION_PERMISSION);
        }
    }

    private void startDownloadService(String statusPath) {
        Intent intent = new Intent(this, DownloadService.class);
        intent.putExtra(DownloadService.EXTRA_STATUS_PATH, statusPath);
        startForegroundService(intent);
    }
}
