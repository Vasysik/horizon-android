package dev.horizon.android.ui;

import android.Manifest;
import android.content.Intent;
import android.content.pm.PackageManager;
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
