package io.github.juanluci.starling;

import android.app.Activity;
import android.content.ContentResolver;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.database.Cursor;
import android.net.Uri;
import android.provider.DocumentsContract;
import android.provider.DocumentsContract.Document;
import android.widget.Toast;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import java.util.List;
import java.util.zip.Deflater;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;

/**
 * Horizon Android's worlds and resource packs brought into Starling, the two ports being one now.
 * Horizon Android shows its files to the system as a documents provider (its
 * HorizonDocumentsProvider: a root "horizon" holding each profile's "p0/worlds", "p0/packs"...),
 * so the system's folder picker, opened on it, gives Starling read access to what the player
 * picks: the whole folder, its Worlds or Resource Packs, or one world or pack. Each world goes
 * into the game's import inbox as a zip of its folder, each pack as it is (zipped when it is a
 * folder); the game installs them as it does a file opened with Starling
 * (crates/horizon-client/src/import.rs), a world under a name of its own beside the others.
 */
final class HorizonImport {
    static final String PACKAGE = "dev.horizon.android";
    static final String AUTHORITY = PACKAGE + ".documents";
    /** The provider's root document: its children are "p<profile>/<folder>" (document_store.rs). */
    private static final String ROOT = "horizon";
    /** Why the process is held in the foreground while the copy runs (WorkService). */
    private static final String REASON = "horizon-import";
    /** What a world keeps that is the running game's, not the world's. */
    private static final String LOCK = "session.lock";

    private HorizonImport() {}

    /** Whether Horizon Android is on the phone (its package is declared in the manifest's queries). */
    static boolean installed(Context context) {
        try {
            context.getPackageManager().getPackageInfo(PACKAGE, 0);
            return true;
        } catch (PackageManager.NameNotFoundException e) {
            return false;
        }
    }

    /** The system's folder picker, opened on Horizon Android's folder. */
    static Intent picker() {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
        intent.putExtra(DocumentsContract.EXTRA_INITIAL_URI,
                DocumentsContract.buildDocumentUri(AUTHORITY, ROOT));
        return intent;
    }

    /** Copy what `tree` holds into the game's import inbox, on a thread of its own. */
    static void start(final Activity activity, final Uri tree) {
        new Thread(new Runnable() {
            @Override
            public void run() {
                WorkService.hold(activity, REASON,
                        "Bringing your worlds over from Horizon Android", true);
                try {
                    String said = bring(activity, tree);
                    if (said != null) {
                        tell(activity, said);
                    }
                } finally {
                    WorkService.hold(activity, REASON, "", false);
                }
            }
        }, "starling-horizon-import").start();
    }

    /** One document of the tree: its id, its name, and whether it is a folder. */
    private static final class Item {
        final String id;
        final String name;
        final boolean folder;

        Item(String id, String name, String mime) {
            this.id = id;
            this.name = name;
            this.folder = Document.MIME_TYPE_DIR.equals(mime);
        }
    }

    /**
     * Each world and pack under the picked document into the inbox; what to tell the player when
     * the game will not tell it (its own notes say what each import added), else null.
     */
    private static String bring(Activity activity, Uri tree) {
        if (!AUTHORITY.equals(tree.getAuthority())) {
            return "That is not Horizon Android's folder: pick Horizon in the list of places";
        }
        ContentResolver files = activity.getContentResolver();
        File inbox = new File(activity.getFilesDir(), "import-inbox");
        inbox.mkdirs();
        String picked = DocumentsContract.getTreeDocumentId(tree);
        String[] path = picked.split("/");
        int brought = 0;
        try {
            if (path.length == 1) {
                // The whole folder: the current profile's Worlds and Resource Packs.
                for (Item folder : children(files, tree, picked)) {
                    String kind = last(folder.id);
                    if (!"worlds".equals(kind) && !"packs".equals(kind)) {
                        continue;
                    }
                    for (Item item : children(files, tree, folder.id)) {
                        brought += bring(files, tree, inbox, kind, item);
                    }
                }
            } else if (path.length == 2) {
                for (Item item : children(files, tree, picked)) {
                    brought += bring(files, tree, inbox, path[1], item);
                }
            } else {
                brought += bring(files, tree, inbox, path[1], document(files, tree, picked));
            }
        } catch (IOException | RuntimeException e) {
            android.util.Log.w("starling", "could not read Horizon Android's folder", e);
            return "Could not read Horizon Android's files: close Horizon Android, then try again";
        }
        return brought == 0 ? "Horizon Android has no worlds or resource packs there" : null;
    }

    /** One world or pack into the inbox: 1 when it went, 0 for anything else. */
    private static int bring(ContentResolver files, Uri tree, File inbox, String kind, Item item) {
        boolean world = "worlds".equals(kind);
        if (!(world || "packs".equals(kind)) || (world && !item.folder)) {
            return 0;
        }
        String name = safe(item.name);
        File part = null;
        try {
            if (item.folder) {
                part = arriving(inbox, name + ".zip");
                try (ZipOutputStream zip = new ZipOutputStream(new FileOutputStream(part))) {
                    // Region files are compressed already: the quickest level loses little.
                    zip.setLevel(Deflater.BEST_SPEED);
                    zipFolder(files, tree, item.id, name + "/", zip);
                }
            } else {
                part = arriving(inbox, name);
                try (InputStream in = open(files, tree, item.id);
                        OutputStream out = new FileOutputStream(part)) {
                    copy(in, out);
                }
            }
            String done = part.getName();
            if (!part.renameTo(new File(inbox, done.substring(0, done.length() - ".part".length())))) {
                throw new IOException("could not finish " + done);
            }
            return 1;
        } catch (IOException | RuntimeException e) {
            android.util.Log.w("starling", "could not bring " + item.name + " over", e);
            if (part != null) {
                part.delete();
            }
            return 0;
        }
    }

    /** A folder's files into `zip` under `prefix`, its own folders too, without the lock. */
    private static void zipFolder(ContentResolver files, Uri tree, String id, String prefix,
            ZipOutputStream zip) throws IOException {
        for (Item item : children(files, tree, id)) {
            if (item.folder) {
                zipFolder(files, tree, item.id, prefix + item.name + "/", zip);
            } else if (!LOCK.equals(item.name)) {
                zip.putNextEntry(new ZipEntry(prefix + item.name));
                try (InputStream in = open(files, tree, item.id)) {
                    copy(in, zip);
                }
                zip.closeEntry();
            }
        }
    }

    private static List<Item> children(ContentResolver files, Uri tree, String id)
            throws IOException {
        return items(files, DocumentsContract.buildChildDocumentsUriUsingTree(tree, id));
    }

    private static Item document(ContentResolver files, Uri tree, String id) throws IOException {
        List<Item> found = items(files, DocumentsContract.buildDocumentUriUsingTree(tree, id));
        if (found.isEmpty()) {
            throw new IOException("no document " + id);
        }
        return found.get(0);
    }

    private static List<Item> items(ContentResolver files, Uri uri) throws IOException {
        List<Item> items = new ArrayList<>();
        String[] columns = {
            Document.COLUMN_DOCUMENT_ID, Document.COLUMN_DISPLAY_NAME, Document.COLUMN_MIME_TYPE
        };
        try (Cursor cursor = files.query(uri, columns, null, null, null)) {
            if (cursor == null) {
                throw new IOException("nothing listed at " + uri);
            }
            while (cursor.moveToNext()) {
                items.add(new Item(cursor.getString(0), cursor.getString(1), cursor.getString(2)));
            }
        }
        return items;
    }

    private static InputStream open(ContentResolver files, Uri tree, String id)
            throws IOException {
        InputStream in =
                files.openInputStream(DocumentsContract.buildDocumentUriUsingTree(tree, id));
        if (in == null) {
            throw new IOException("could not open " + id);
        }
        return in;
    }

    private static void copy(InputStream in, OutputStream out) throws IOException {
        byte[] buffer = new byte[1 << 16];
        for (int n; (n = in.read(buffer)) > 0; ) {
            out.write(buffer, 0, n);
        }
    }

    /** A free `<name>.part` in the inbox (the game waits for it to lose ".part"). */
    private static File arriving(File inbox, String name) {
        String stem = name;
        String extension = "";
        int dot = name.lastIndexOf('.');
        if (dot > 0) {
            stem = name.substring(0, dot);
            extension = name.substring(dot);
        }
        File file = new File(inbox, name);
        for (int i = 2; file.exists() || new File(inbox, file.getName() + ".part").exists(); i++) {
            file = new File(inbox, stem + " (" + i + ")" + extension);
        }
        return new File(inbox, file.getName() + ".part");
    }

    /** The last part of a document id ("p0/worlds" is "worlds"). */
    private static String last(String id) {
        return id.substring(id.lastIndexOf('/') + 1);
    }

    /** A name the inbox can hold (LauncherActivity's rule): no folders, no ".part" at its end. */
    private static String safe(String name) {
        String clean = name == null || name.isEmpty() ? "Horizon Android" : name;
        clean = clean.replaceAll("[/\\\\:*?\"<>|]", "_");
        return clean.endsWith(".part") ? clean + "_" : clean;
    }

    private static void tell(final Activity activity, final String text) {
        activity.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                Toast.makeText(activity, text, Toast.LENGTH_LONG).show();
            }
        });
    }
}
