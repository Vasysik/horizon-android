package dev.horizon.android.ui;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Intent;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;

import org.json.JSONObject;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;

public final class DownloadService extends Service {
    public static final String EXTRA_STATUS_PATH = "statusPath";

    private static final String CHANNEL_ID = "horizon_downloads";
    private static final int NOTIFICATION_ID = 1001;

    private final Handler handler = new Handler(Looper.getMainLooper());
    private String statusPath;
    private boolean polling;
    private boolean finished;

    private final Runnable poll = new Runnable() {
        @Override
        public void run() {
            if (!polling) {
                return;
            }
            tick();
            if (polling) {
                handler.postDelayed(this, 800);
            }
        }
    };

    @Override
    public void onCreate() {
        super.onCreate();
        createChannel();
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        if (intent != null && intent.getStringExtra(EXTRA_STATUS_PATH) != null) {
            statusPath = intent.getStringExtra(EXTRA_STATUS_PATH);
        }
        if (statusPath == null) {
            statusPath = new File(getFilesDir(), "horizon-install-status.json").getAbsolutePath();
        }
        finished = false;
        polling = true;
        startForeground(NOTIFICATION_ID, notification("Minecraft 26.3", "Hazırlanıyor", 0, 0, true));
        handler.removeCallbacks(poll);
        handler.post(poll);
        return START_NOT_STICKY;
    }

    @Override
    public void onDestroy() {
        polling = false;
        handler.removeCallbacks(poll);
        super.onDestroy();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    private void tick() {
        try {
            File file = new File(statusPath);
            if (!file.isFile()) {
                update("Minecraft 26.3", "Hazırlanıyor", 0, 0, true);
                return;
            }
            String text = new String(Files.readAllBytes(file.toPath()), StandardCharsets.UTF_8);
            JSONObject json = new JSONObject(text);
            String state = json.optString("state", "running");
            long done = json.optLong("done", 0);
            long total = json.optLong("total", 0);
            int stage = json.optInt("stage_id", 0);

            if ("done".equals(state)) {
                finish("Minecraft 26.3", "Hazır", 100, 100, false);
                return;
            }
            if ("failed".equals(state)) {
                finish("Minecraft 26.3", "İndirme tamamlanamadı", 0, 0, false);
                return;
            }

            String label;
            switch (stage) {
                case 0:
                    label = "Sürüm kontrol ediliyor";
                    break;
                case 1:
                    label = total > 0
                            ? "Kaynaklar indiriliyor · %" + Math.min(100, (int) (done * 100 / total))
                            : "Kaynaklar indiriliyor";
                    break;
                case 2:
                    label = "Dosyalar açılıyor";
                    break;
                case 3:
                    label = "Dünya verileri hazırlanıyor";
                    break;
                case 4:
                    label = total > 0
                            ? "Kontroller hazırlanıyor · " + done + "/" + total
                            : "Kontroller hazırlanıyor";
                    break;
                default:
                    label = "Son dokunuşlar";
                    break;
            }
            update("Minecraft 26.3", label, done, total, stage != 1);
        } catch (Exception error) {
            update("Minecraft 26.3", "İndirme sürüyor", 0, 0, true);
        }
    }

    private void update(String title, String text, long done, long total, boolean indeterminate) {
        NotificationManager manager = getSystemService(NotificationManager.class);
        manager.notify(NOTIFICATION_ID, notification(title, text, done, total, indeterminate));
    }

    private void finish(String title, String text, long done, long total, boolean indeterminate) {
        if (finished) {
            return;
        }
        finished = true;
        polling = false;
        handler.removeCallbacks(poll);
        NotificationManager manager = getSystemService(NotificationManager.class);
        manager.notify(NOTIFICATION_ID, notification(title, text, done, total, indeterminate));
        stopForeground(STOP_FOREGROUND_DETACH);
        stopSelf();
    }

    private Notification notification(
            String title, String text, long done, long total, boolean indeterminate) {
        PendingIntent content = PendingIntent.getActivity(
                this,
                0,
                new Intent(this, MainActivity.class).setFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        Notification.Builder builder = new Notification.Builder(this, CHANNEL_ID)
                .setContentTitle(title)
                .setContentText(text)
                .setSmallIcon(android.R.drawable.stat_sys_download)
                .setContentIntent(content)
                .setOnlyAlertOnce(true)
                .setOngoing(polling);
        if (indeterminate || total <= 0) {
            builder.setProgress(0, 0, polling);
        } else {
            builder.setProgress(100, (int) Math.min(100, done * 100 / total), false);
        }
        return builder.build();
    }

    private void createChannel() {
        NotificationManager manager = getSystemService(NotificationManager.class);
        NotificationChannel channel = new NotificationChannel(
                CHANNEL_ID, "Horizon indirmeleri", NotificationManager.IMPORTANCE_LOW);
        channel.setDescription("Minecraft 26.3 indirme ilerlemesi");
        channel.setShowBadge(false);
        manager.createNotificationChannel(channel);
    }
}
