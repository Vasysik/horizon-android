package io.github.juanluci.starling;

import android.Manifest;
import android.app.NativeActivity;
import android.content.ActivityNotFoundException;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.hardware.Sensor;
import android.hardware.SensorEvent;
import android.hardware.SensorEventListener;
import android.hardware.SensorManager;
import android.net.ConnectivityManager;
import android.net.LinkProperties;
import android.net.Network;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.CombinedVibration;
import android.os.Environment;
import android.os.VibrationEffect;
import android.os.Vibrator;
import android.os.VibratorManager;
import android.provider.DocumentsContract;
import android.view.InputDevice;
import android.view.View;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.view.ViewTreeObserver;
import android.view.inputmethod.InputMethodManager;
import android.view.WindowManager;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.system.ErrnoException;
import android.system.Os;
import java.io.File;
import java.net.InetAddress;
import java.util.List;
import java.security.Key;
import java.security.KeyStore;
import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

/**
 * The game runs in libhorizon_android (NativeActivity); this only does what the NDK cannot:
 * tells the game where its folder is (Starling in shared storage once the player allows it), hides the
 * system bars, keeps the screen on while the game is in front, tells the game how tall the
 * on-screen keyboard is (it no longer shrinks the window since edge-to-edge), and ends the
 * process with the activity, since the game's event loop cannot start twice in one process.
 * The game calls the public methods from its own thread (android_lib.rs): the file manager, the
 * file picker, the clipboard, the keyboard's editor (TextMirror), the browser, the keystore, the
 * mouse's capture and the controllers' makers and motors.
 */
public final class StarlingActivity extends NativeActivity {
    private static final int PICK_FILE = 1;
    /** Horizon Android's folder picked: its worlds and packs come over (HorizonImport). */
    private static final int PICK_HORIZON = 2;

    /** The build with native mods loaded in place of the APK's library (android_lib.rs). */
    private static final String GAME_LIBRARY = "STARLING_GAME_LIBRARY";

    /**
     * Loads the game library with this class loader, so the native methods resolve: a build with
     * native mods when one is chosen (NativeMods), else the APK's own. NativeActivity then loads
     * the APK's library, whose entry point (starling_on_create, android_lib.rs) hands over to the
     * build named by GAME_LIBRARY, the very library loaded here.
     */
    private void loadGame() {
        File build = NativeMods.chosen(this, versionName());
        if (build != null) {
            try {
                Os.setenv(GAME_LIBRARY, build.getPath(), true);
                System.load(build.getPath());
                return;
            } catch (UnsatisfiedLinkError | ErrnoException e) {
                try {
                    Os.unsetenv(GAME_LIBRARY);
                } catch (ErrnoException ignored) {
                    // Nothing was set.
                }
                NativeMods.failed(this, e.getMessage());
            }
        }
        System.loadLibrary("horizon_android");
    }

    /** The keyboard's height in window pixels, 0 when it is hidden (android_lib.rs). */
    private static native void onImeHeight(int pixels);

    /** The game folder in shared storage, or null to keep it in the app's own files. */
    private static native void onStorage(String folder);

    /** Android took back the pointer the game had captured (android_lib.rs). */
    private static native void onPointerCaptureLost();

    /** The game holds the pointer (capturePointer); on the UI thread only. */
    private boolean pointerWanted;

    private ClipboardManager clipboard;

    /** The game has drawn: the window may show (firstFrame). */
    private volatile boolean drawn;

    /** Shared storage's Starling folder when the player allowed the app to use it, else null:
     *  StarlingDev for Starling Dev (build_apk.py --dev) and StarlingPreview for Starling Preview
     *  (--preview), which keep out of the release's. */
    static String sharedFolder(Context context) {
        boolean allowed = Build.VERSION.SDK_INT >= 30
                ? Environment.isExternalStorageManager()
                : context.checkSelfPermission(Manifest.permission.WRITE_EXTERNAL_STORAGE)
                        == PackageManager.PERMISSION_GRANTED;
        String id = context.getPackageName();
        String name = id.endsWith(".dev") ? "StarlingDev"
                : id.endsWith(".preview") ? "StarlingPreview" : "Starling";
        return allowed
                ? new File(Environment.getExternalStorageDirectory(), name).getPath()
                : null;
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        loadGame();
        // Before super.onCreate starts the game's thread, which reads it first thing.
        onStorage(sharedFolder(this));
        super.onCreate(savedInstanceState);
        // Made here, on the UI thread: the game reads and writes it from its own.
        clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        // As games do: watching, a controller or a benchmark can go minutes without a touch.
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        // A NativeActivity's window counts as drawn at once, so the system's starting window (the
        // app icon) went and the screen behind showed until the game's first frame: the window
        // is held back until the game says it has drawn, or ten seconds at most.
        final View decor = getWindow().getDecorView();
        decor.getViewTreeObserver().addOnPreDrawListener(new ViewTreeObserver.OnPreDrawListener() {
            @Override
            public boolean onPreDraw() {
                if (!drawn) {
                    return false;
                }
                decor.getViewTreeObserver().removeOnPreDrawListener(this);
                return true;
            }
        });
        decor.postDelayed(new Runnable() {
            @Override
            public void run() {
                firstFrame();
            }
        }, 10000);
        if (Build.VERSION.SDK_INT >= 28) {
            WindowManager.LayoutParams attributes = getWindow().getAttributes();
            attributes.layoutInDisplayCutoutMode =
                    WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
            getWindow().setAttributes(attributes);
        }
        if (Build.VERSION.SDK_INT >= 30) {
            getWindow().getDecorView().setOnApplyWindowInsetsListener(
                    new View.OnApplyWindowInsetsListener() {
                        @Override
                        public WindowInsets onApplyWindowInsets(View view, WindowInsets insets) {
                            onImeHeight(insets.getInsets(WindowInsets.Type.ime()).bottom);
                            // The keyboard brings the navigation bar back, and it stayed after
                            // the keyboard went: home sat beside the chat's keys.
                            if (insets.isVisible(WindowInsets.Type.navigationBars())
                                    || insets.isVisible(WindowInsets.Type.statusBars())) {
                                view.post(new Runnable() {
                                    @Override
                                    public void run() {
                                        hideSystemBars();
                                    }
                                });
                            }
                            return view.onApplyWindowInsets(insets);
                        }
                    });
        }
        hideSystemBars();
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus) {
            hideSystemBars();
        }
    }

    /**
     * Capture the mouse for the game's mouse look, or let it go (the game's MouseHandler.grabMouse
     * and releaseMouse): captured, its pointer hides and its moves come to the game as relative
     * motion, through NativeActivity's input queue like every other event.
     */
    public void capturePointer(final boolean capture) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                pointerWanted = capture;
                View decor = getWindow().getDecorView();
                if (capture) {
                    decor.requestPointerCapture();
                } else {
                    decor.releasePointerCapture();
                }
            }
        });
    }

    @Override
    public void onPointerCaptureChanged(boolean hasCapture) {
        super.onPointerCaptureChanged(hasCapture);
        // Lost while the game holds it (the window's focus went): the game lets go too.
        if (!hasCapture && pointerWanted) {
            pointerWanted = false;
            onPointerCaptureLost();
        }
    }

    /** A controller's maker (its USB vendor id), for the game's button pictures; 0 unknown. */
    public int padVendor(int deviceId) {
        InputDevice device = InputDevice.getDevice(deviceId);
        return device == null ? 0 : device.getVendorId();
    }

    /** A controller's name, for the game's log; null once it is gone. */
    public String padName(int deviceId) {
        InputDevice device = InputDevice.getDevice(deviceId);
        return device == null ? null : device.getName();
    }

    /**
     * Vibrate a controller for `millis` ms, its heavy motor at `strong` and its light one at
     * `weak` (0 to 255, 0 still): each of its motors from Android 12, before that its one
     * vibrator at the stronger. A pad without motors stays still.
     */
    @SuppressWarnings("deprecation")
    public void vibratePad(int deviceId, int millis, int strong, int weak) {
        InputDevice device = InputDevice.getDevice(deviceId);
        if (device == null || millis <= 0) {
            return;
        }
        if (Build.VERSION.SDK_INT >= 31) {
            VibratorManager motors = device.getVibratorManager();
            int[] ids = motors.getVibratorIds();
            CombinedVibration.ParallelCombination all = CombinedVibration.startParallel();
            boolean any = false;
            for (int i = 0; i < ids.length; i++) {
                // A pad's first motor is its heavy one (the evdev rumble's strong magnitude).
                int level = Math.min(i == 0 ? strong : weak, 255);
                if (level > 0) {
                    all.addVibrator(ids[i], VibrationEffect.createOneShot(millis, level));
                    any = true;
                }
            }
            if (any) {
                motors.vibrate(all.combine());
            }
            return;
        }
        Vibrator motor = device.getVibrator();
        int level = Math.min(Math.max(strong, weak), 255);
        if (motor.hasVibrator() && level > 0) {
            motor.vibrate(VibrationEffect.createOneShot(millis, level));
        }
    }

    /** The sensors, for the gyroscope the touch controls can aim with. */
    private SensorManager sensors;
    /** What the gyroscope turned since the game last took it: radians about the device's own x
     *  and y (those of its natural portrait). */
    private final float[] gyroTurned = new float[2];
    /** The last gyroscope event's time (ns), 0 before the first one since it started. */
    private long gyroLast;
    private final SensorEventListener gyroListener = new SensorEventListener() {
        @Override
        public void onSensorChanged(SensorEvent event) {
            long last = gyroLast;
            gyroLast = event.timestamp;
            float seconds = (event.timestamp - last) * 1e-9f;
            // The first event, or one after a gap: nothing to integrate over.
            if (last == 0 || seconds <= 0 || seconds > 0.1f) {
                return;
            }
            synchronized (gyroTurned) {
                gyroTurned[0] += event.values[0] * seconds;
                gyroTurned[1] += event.values[1] * seconds;
            }
        }

        @Override
        public void onAccuracyChanged(Sensor sensor, int accuracy) {
        }
    };

    /** Start or stop listening to the gyroscope; false when the phone has none. */
    public boolean gyroscope(boolean on) {
        if (sensors == null) {
            sensors = (SensorManager) getSystemService(Context.SENSOR_SERVICE);
        }
        Sensor gyro = sensors == null ? null : sensors.getDefaultSensor(Sensor.TYPE_GYROSCOPE);
        if (gyro == null) {
            return false;
        }
        if (on) {
            gyroLast = 0;
            // Every 5 ms (200 Hz, the most an app gets without asking for more): a frame at
            // 120 fps still has a turn to take.
            sensors.registerListener(gyroListener, gyro, 5000);
        } else {
            sensors.unregisterListener(gyroListener);
        }
        synchronized (gyroTurned) {
            gyroTurned[0] = 0;
            gyroTurned[1] = 0;
        }
        return true;
    }

    /** What the gyroscope turned since the last call (radians about the device's x and y) and
     *  the display's rotation (Surface.ROTATION_*); it counts again from there. */
    @SuppressWarnings("deprecation")
    public float[] takeGyro() {
        int rotation = getWindowManager().getDefaultDisplay().getRotation();
        synchronized (gyroTurned) {
            float[] taken = {gyroTurned[0], gyroTurned[1], rotation};
            gyroTurned[0] = 0;
            gyroTurned[1] = 0;
            return taken;
        }
    }

    /**
     * Buzz the phone itself for `millis` ms at its default strength: the touch controls'
     * feedback (a block broken, a stack split), as Bedrock's. A phone without a motor stays still.
     */
    @SuppressWarnings("deprecation")
    public void vibrate(int millis) {
        if (millis <= 0) {
            return;
        }
        Vibrator motor;
        if (Build.VERSION.SDK_INT >= 31) {
            VibratorManager motors =
                (VibratorManager) getSystemService(Context.VIBRATOR_MANAGER_SERVICE);
            motor = motors == null ? null : motors.getDefaultVibrator();
        } else {
            motor = (Vibrator) getSystemService(Context.VIBRATOR_SERVICE);
        }
        if (motor != null && motor.hasVibrator()) {
            motor.vibrate(VibrationEffect.createOneShot(millis, VibrationEffect.DEFAULT_AMPLITUDE));
        }
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode == PICK_FILE && resultCode == RESULT_OK && data != null
                && data.getData() != null) {
            // The game shows the copy arriving in its inbox, then the import.
            LauncherActivity.receive(this, data.getData(), null);
        } else if (requestCode == PICK_HORIZON && resultCode == RESULT_OK && data != null
                && data.getData() != null) {
            HorizonImport.start(this, data.getData());
        }
    }

    @Override
    protected void onDestroy() {
        // The world was saved when the game left the screen (game/saving.rs, save_for_background).
        android.os.Process.killProcess(android.os.Process.myPid());
        super.onDestroy();
    }

    /** Put the on-screen keyboard away, however it was shown. */
    public void hideKeyboard() {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                InputMethodManager keyboard =
                        (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
                if (keyboard != null) {
                    keyboard.hideSoftInputFromWindow(getWindow().getDecorView().getWindowToken(), 0);
                }
            }
        });
    }

    /** The editor behind the keyboard (TextMirror), made the first time the game asks for it. */
    private TextMirror mirror;

    /**
     * The game's text field in the keyboard's editor, which takes the focus and shows the keyboard
     * (game/touch/text_mirror.rs; TextMirror.update says what the numbers are).
     */
    public void showTextMirror(final String text, final int cursor, final int anchor,
            final int maxLength, final int flags, final int generation) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                if (mirror == null) {
                    mirror = new TextMirror(StarlingActivity.this);
                    addContentView(mirror, new android.view.ViewGroup.LayoutParams(1, 1));
                }
                mirror.show(text, cursor, anchor, maxLength, flags, generation);
            }
        });
    }

    /** The game changed its field itself (Tab, the history, a line sent): the editor follows. */
    public void updateTextMirror(final String text, final int cursor, final int anchor,
            final int maxLength, final int flags, final int generation) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                if (mirror != null) {
                    mirror.update(text, cursor, anchor, maxLength, flags, generation);
                }
            }
        });
    }

    /** The keyboard and its editor put away. */
    public void hideTextMirror() {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                if (mirror != null) {
                    mirror.hide();
                }
            }
        });
    }

    /** The game drew its first frame: let the window show. */
    public void firstFrame() {
        NativeMods.started(this);
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                drawn = true;
                getWindow().getDecorView().invalidate();
            }
        });
    }

    /** Whether the player allowed the Starling folder (they may have just come back from it). */
    public boolean fileAccessAllowed() {
        return sharedFolder(this) != null;
    }

    /**
     * Android's setting that allows the Starling folder, in a task of its own: home and Starling's
     * icon bring the player back to the game, not to the setting.
     */
    public void openFileAccessSettings() {
        if (Build.VERSION.SDK_INT < 30) {
            runOnUiThread(new Runnable() {
                @Override
                public void run() {
                    requestPermissions(
                            new String[] {Manifest.permission.WRITE_EXTERNAL_STORAGE}, 2);
                }
            });
            return;
        }
        Intent[] settings = {
            new Intent(android.provider.Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                    Uri.parse("package:" + getPackageName())),
            new Intent(android.provider.Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION),
        };
        for (Intent intent : settings) {
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            try {
                startActivity(intent);
                return;
            } catch (ActivityNotFoundException e) {
                // The general list, then.
            }
        }
    }

    /**
     * Start the game again in a new process (the game folder is chosen at start): LauncherActivity,
     * in a process of its own, waits for this one to end.
     */
    public void restart() {
        Intent again = new Intent(this, LauncherActivity.class);
        again.putExtra(LauncherActivity.AFTER, android.os.Process.myPid());
        again.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        startActivity(again);
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                finish();
                // Straight to the launcher's restart screen, no window animation between.
                overridePendingTransition(0, 0);
            }
        });
    }

    /** The system's file picker; the file chosen goes to the game's import inbox. */
    public boolean pickFile() {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                intent.addCategory(Intent.CATEGORY_OPENABLE);
                intent.setType("*/*");
                try {
                    startActivityForResult(intent, PICK_FILE);
                } catch (ActivityNotFoundException e) {
                    // No picker: nothing to show.
                }
            }
        });
        return true;
    }

    /** Whether Horizon Android is on the phone, so its worlds can come over (the setup asks). */
    public boolean horizonAndroidInstalled() {
        return HorizonImport.installed(this);
    }

    /**
     * The system's folder picker on Horizon Android's folder; its worlds and packs go to the
     * game's import inbox (HorizonImport). False when it is not installed or nothing can pick.
     */
    public boolean importFromHorizonAndroid() {
        if (!HorizonImport.installed(this)) {
            return false;
        }
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                try {
                    startActivityForResult(HorizonImport.picker(), PICK_HORIZON);
                } catch (ActivityNotFoundException e) {
                    // No folder picker: nothing to show.
                }
            }
        });
        return true;
    }

    /** The system's file manager on a folder of shared storage; false where none can show it. */
    public boolean openFolder(String path) {
        String root = Environment.getExternalStorageDirectory().getPath();
        if (!path.startsWith(root)) {
            return false;
        }
        String relative = path.substring(root.length()).replaceFirst("^/+", "");
        Uri folder = DocumentsContract.buildDocumentUri(
                "com.android.externalstorage.documents", "primary:" + relative);
        // Any file manager that takes a folder, else the system's own (Files).
        String[] packages = {null, "com.google.android.documentsui", "com.android.documentsui"};
        for (String app : packages) {
            Intent intent = new Intent(Intent.ACTION_VIEW);
            intent.setDataAndType(folder, DocumentsContract.Document.MIME_TYPE_DIR);
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_GRANT_READ_URI_PERMISSION);
            if (app != null) {
                intent.setPackage(app);
            }
            try {
                startActivity(intent);
                return true;
            } catch (ActivityNotFoundException | SecurityException e) {
                // The next one.
            }
        }
        return false;
    }

    /**
     * Keeps the process in the foreground (WorkService) while the game waits for `reason`
     * (the sign-in in the browser, a download), with `line` in its notification; `on` false lets
     * that reason go.
     */
    public void keepWorking(String reason, String line, boolean on) {
        WorkService.hold(this, reason, line, on);
    }

    /** The system's browser on an https:// page (the Microsoft sign-in); false where none can. */
    public boolean openLink(String url) {
        if (!url.startsWith("https://")) {
            return false;
        }
        Intent intent = new Intent(Intent.ACTION_VIEW, Uri.parse(url));
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        try {
            startActivity(intent);
            return true;
        } catch (ActivityNotFoundException | SecurityException e) {
            return false;
        }
    }

    /** The active network's DNS servers, for the SRV records that name most big servers
     * (Android has no resolv.conf to read them from); empty when there is no network. */
    public String[] dnsServers() {
        ConnectivityManager connectivity =
                (ConnectivityManager) getSystemService(Context.CONNECTIVITY_SERVICE);
        Network network = connectivity == null ? null : connectivity.getActiveNetwork();
        LinkProperties link = network == null ? null : connectivity.getLinkProperties(network);
        if (link == null) {
            return new String[0];
        }
        List<InetAddress> servers = link.getDnsServers();
        String[] out = new String[servers.size()];
        for (int i = 0; i < out.length; i++) {
            out[i] = servers.get(i).getHostAddress();
        }
        return out;
    }

    /** The clipboard's text, or null when it holds none. */
    public String clipboardText() {
        ClipData clip = clipboard == null ? null : clipboard.getPrimaryClip();
        if (clip == null || clip.getItemCount() == 0) {
            return null;
        }
        CharSequence text = clip.getItemAt(0).coerceToText(this);
        return text == null ? null : text.toString();
    }

    public boolean setClipboardText(String text) {
        if (clipboard == null) {
            return false;
        }
        clipboard.setPrimaryClip(ClipData.newPlainText("Starling", text));
        return true;
    }

    /** The display's fastest refresh rate (Hz), which a game without a frame cap asks for: left
     * to itself, a phone's own policy may hold a game at 60 Hz on a 120 or 144 Hz screen. 0 where
     * Android does not say. */
    @SuppressWarnings("deprecation")
    public float maxRefreshRate() {
        android.view.Display display = Build.VERSION.SDK_INT >= 30
                ? getDisplay()
                : getWindowManager().getDefaultDisplay();
        float best = 0;
        if (display != null) {
            for (android.view.Display.Mode mode : display.getSupportedModes()) {
                best = Math.max(best, mode.getRefreshRate());
            }
        }
        return best;
    }

    /** This APK's version (versionName), which the update prompt compares with the latest
     * release's; null where Android does not say. */
    public String versionName() {
        try {
            return getPackageManager().getPackageInfo(getPackageName(), 0).versionName;
        } catch (PackageManager.NameNotFoundException e) {
            return null;
        }
    }

    /** Whether this is a release (signed with the release key, not debuggable): only a release
     * can install the next one over itself. */
    public boolean releaseBuild() {
        return (getApplicationInfo().flags & android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE)
                == 0;
    }

    /**
     * Android's package installer on an update the game downloaded to files/updates/: it asks the
     * player to allow installing from Starling the first time, then to update; once installed it
     * offers to open the game again. False where no installer takes it.
     */
    public boolean installUpdate(String name) {
        Intent intent = new Intent(Intent.ACTION_VIEW);
        intent.setDataAndType(UpdateProvider.uri(getPackageName(), name), UpdateProvider.TYPE);
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_ACTIVITY_NEW_TASK);
        try {
            startActivity(intent);
            return true;
        } catch (ActivityNotFoundException | SecurityException e) {
            return false;
        }
    }

    /** The Android Keystore key the account's tokens are sealed with; it never leaves the
     * Keystore. */
    private static final String SECRET_KEY = "starling-account";

    private static SecretKey secretKey() throws Exception {
        KeyStore store = KeyStore.getInstance("AndroidKeyStore");
        store.load(null);
        Key key = store.getKey(SECRET_KEY, null);
        if (key instanceof SecretKey) {
            return (SecretKey) key;
        }
        KeyGenerator generator =
                KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
        generator.init(new KeyGenParameterSpec.Builder(SECRET_KEY,
                KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build());
        return generator.generateKey();
    }

    /** `data` sealed with the Keystore key (AES-GCM): the IV's length, the IV, then the
     * ciphertext and its tag. Null when the Keystore refused. */
    public byte[] sealSecret(byte[] data) {
        try {
            Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.ENCRYPT_MODE, secretKey());
            byte[] iv = cipher.getIV();
            byte[] sealed = cipher.doFinal(data);
            byte[] out = new byte[1 + iv.length + sealed.length];
            out[0] = (byte) iv.length;
            System.arraycopy(iv, 0, out, 1, iv.length);
            System.arraycopy(sealed, 0, out, 1 + iv.length, sealed.length);
            return out;
        } catch (Exception e) {
            return null;
        }
    }

    /** What sealSecret sealed, or null when it was not sealed here (or was changed). */
    public byte[] openSecret(byte[] sealed) {
        try {
            int ivLength = sealed[0] & 0xFF;
            Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.DECRYPT_MODE, secretKey(),
                    new GCMParameterSpec(128, sealed, 1, ivLength));
            return cipher.doFinal(sealed, 1 + ivLength, sealed.length - 1 - ivLength);
        } catch (Exception e) {
            return null;
        }
    }

    @SuppressWarnings("deprecation")
    private void hideSystemBars() {
        if (Build.VERSION.SDK_INT >= 30) {
            WindowInsetsController controller = getWindow().getInsetsController();
            if (controller != null) {
                controller.hide(WindowInsets.Type.systemBars());
                controller.setSystemBarsBehavior(
                        WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
            }
        } else {
            getWindow().getDecorView().setSystemUiVisibility(
                    View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                            | View.SYSTEM_UI_FLAG_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_LAYOUT_STABLE);
        }
    }
}
