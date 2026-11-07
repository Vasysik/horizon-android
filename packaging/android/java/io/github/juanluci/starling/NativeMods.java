package io.github.juanluci.starling;

import android.content.Context;
import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

/**
 * The game with native mods (game/native_builds.rs): a build of this version's game library with
 * the player's native mods compiled in, downloaded to files/native/KEY/ and chosen by
 * files/native/current. StarlingActivity loads it in place of the APK's own library. A build made
 * for another version is never loaded (after an update it waits for its rebuild), and one that
 * stopped twice before its first frame is set aside, with a notice the game shows.
 */
final class NativeMods {
    private NativeMods() {}

    static final String LIBRARY = "libhorizon_android.so";
    /** Starts that ended before the first frame, after which the build is set aside. */
    private static final int TRIES = 2;

    private static File dir(Context context) {
        return new File(context.getFilesDir(), "native");
    }

    /**
     * The chosen build's library for this `version` (the APK's versionName), or null to play the
     * APK's own; counts the start, which `started` ends.
     */
    static File chosen(Context context, String version) {
        File dir = dir(context);
        String key = read(new File(dir, "current"));
        if (key == null || key.isEmpty() || key.contains("/") || key.startsWith(".")) {
            return null;
        }
        File build = new File(dir, key);
        File library = new File(build, LIBRARY);
        if (!library.isFile() || version == null || !version.equals(read(new File(build, "for")))) {
            return null;
        }
        File starts = new File(dir, "starts");
        int count = 0;
        try {
            count = Integer.parseInt(String.valueOf(read(starts)));
        } catch (NumberFormatException e) {
            // None counted.
        }
        if (count >= TRIES) {
            setAside(dir, "The build with native mods closed twice before it showed, so "
                    + "Starling started without it.");
            return null;
        }
        write(starts, Integer.toString(count + 1));
        return library;
    }

    /** The game showed its first frame: the build started. */
    static void started(Context context) {
        new File(dir(context), "starts").delete();
    }

    /**
     * What the build at `library` says its mods need of Android, each with the mods that need it,
     * in build.json's order ({"microphone": ["voicechat"]}): native_build.sh finds it in their
     * source (native_mod_permissions.py); none from a build made before it did, or unreadable.
     */
    static Map<String, List<String>> permissions(File library) {
        Map<String, List<String>> wanted = new LinkedHashMap<>();
        String text = read(new File(library.getParentFile(), "build.json"));
        if (text == null) {
            return wanted;
        }
        try {
            JSONArray mods = new JSONObject(text).optJSONArray("mods");
            for (int i = 0; mods != null && i < mods.length(); i++) {
                JSONObject mod = mods.optJSONObject(i);
                JSONArray needs = mod == null ? null : mod.optJSONArray("permissions");
                for (int j = 0; needs != null && j < needs.length(); j++) {
                    String permission = needs.optString(j);
                    if (!wanted.containsKey(permission)) {
                        wanted.put(permission, new ArrayList<String>());
                    }
                    wanted.get(permission).add(mod.optString("id"));
                }
            }
        } catch (JSONException e) {
            // A damaged build.json asks for nothing.
        }
        return wanted;
    }

    /** Whether the player said Not now to the build's mods' permissions: not asked again for it. */
    static boolean permissionsDeclined(File library) {
        return new File(library.getParentFile(), "permissions-declined").exists();
    }

    static void declinePermissions(File library) {
        write(new File(library.getParentFile(), "permissions-declined"), "");
    }

    /** The build could not be loaded: set aside, saying why. */
    static void failed(Context context, String why) {
        setAside(dir(context), "The build with native mods could not be loaded (" + why
                + "), so Starling started without it.");
    }

    private static void setAside(File dir, String notice) {
        new File(dir, "current").delete();
        new File(dir, "starts").delete();
        write(new File(dir, "notice"), notice);
    }

    /** A small text file's trimmed contents, null when it cannot be read. */
    private static String read(File file) {
        try {
            return new String(Files.readAllBytes(file.toPath()), StandardCharsets.UTF_8).trim();
        } catch (IOException e) {
            return null;
        }
    }

    private static void write(File file, String text) {
        try {
            file.getParentFile().mkdirs();
            Files.write(file.toPath(), text.getBytes(StandardCharsets.UTF_8));
        } catch (IOException e) {
            // Best effort: the count or notice is lost.
        }
    }
}
