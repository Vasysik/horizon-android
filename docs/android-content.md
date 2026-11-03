# Android add-on files

Use **Mods**, **Packs** or **Shaders → Import file**, or open **Installed** and
press Import beside the desired group. Select a file in Android's document picker.
**Open folder**, including the in-game Mod/Resource Pack/Shader Pack Folder
buttons, browses that directory in a system file manager. If the phone shows an
app chooser, select Files. Alternatively open the system Files app and select
**Horizon Android** in its navigation drawer.

The provider contains **Mods**, **Resource Packs**, **Shader Packs** for the
selected launcher profile, and **Reports**. Files stay in app storage; no migration
or second copy is needed. Android grants other apps access to documents you select.
Account files, settings, prepared game resources and worlds are not exposed.
Reports are read-only and can be copied or shared through Files. Screenshots
continue to appear in **Pictures/HorizonAndroid**.

To download a `.hmod` from a link, paste its HTTPS URL in the Mods search field
and press **Import .hmod** (the Search button changes when a URL is entered).
File import and link import are both available.

In **Shaders**, choose **Horizon** for native `.hshader` packs or **Modrinth**
for classic shader ZIPs. The selection is saved and each source has its own search
cache. Modrinth compatibility is experimental: downloading a pack does not mean
every effect is supported. Installation prefers a ZIP version listing the current
Minecraft version, otherwise the newest downloadable ZIP. Source checksums and
archive structure are checked before installation. An existing file belonging to
another project or imported manually is not replaced. Successful downloads appear
in **Installed > Shaders**, where they can be selected or removed.

Successful imports and catalog installs in the launcher open Installed with the
matching group expanded. Errors and cancellation keep the current page. Import
results appear in the status line; progress and conflict dialogs use the bundled font.

Catalog searches run separately from installation jobs, so switching tabs does
not wait for a network response. At most one search runs at once; rapid changes
replace the pending request, and replies for an old tab cannot replace the current
list. The last eight query pages are cached for 30 seconds; Search explicitly
refreshes its query. Changing tabs does not unpack installed `.hmod` WASM files.

Supported files:

- Mods: sandboxed `.hmod` archives; Java `.jar` mods are not accepted. Executing
  `.hmod` mods requires a 64-bit APK, but the files can be managed on 32-bit builds.
- Resource packs: ZIPs with valid `pack.mcmeta` at the archive root.
- Shaders: native `.hshader` packs, native shader ZIPs with `horizon-shaders.toml`
  (stored as `.hshader`), or classic ZIPs with shader sources under `shaders/`.
  A classic ZIP's structure being valid does not guarantee support for all of its
  shader features; the existing game loader reports compatibility errors.

Open Installed to expand the three groups. The list includes regular imported and
downloaded archive files in the current profile. Selection applies at the next
game launch. Removing an add-on requires confirmation and removes only that file,
its browser record and its enabled selection; worlds are not touched.

Imports copy the granted content URI to a temporary Android cache file and validate
it before committing. Existing files are never silently replaced: **Keep both**
chooses a new filename, **Replace** explicitly replaces the selected collision,
and **Cancel** leaves the existing file intact. A Horizon mod with the same manifest
ID must be replaced rather than loaded twice. Cancel is checked between copy and
validation blocks; a document provider may delay an in-flight read.

Limits: 256 MiB input, 512 MiB declared expansion, 32,768 entries and 1,024-byte
entry paths. Native mods are limited to 32 MiB input and 64 MiB expanded data.
Archive entry paths, duplicates, links, CRC/decompression errors and resource-pack
metadata are checked. Archives are kept as files, not extracted into arbitrary
filesystem paths. Invalid imports and interrupted temporary files do not replace
existing content. Original URI grants are not retained after copying.

The allowlisted DocumentsProvider supports browsing, copying files in and out,
editing, renaming files and deleting add-ons. Fixed directories cannot be removed;
nonempty subdirectories must be emptied explicitly. Hidden paths, traversal and
symlinks are rejected. It does not require MANAGE_EXTERNAL_STORAGE. World import/
export and moving the game to an external folder remain separate tasks.

Copying directly with a file manager bypasses the Import file validator; wait for
the copy to finish before starting the game, and use the game's pack selection to
reload changes. Import file remains the validated route with progress/cancellation
and explicit conflict choices. A system file manager controls its own copy and
overwrite dialogs. Deleting through Files removes the underlying add-on.

Developer checks: `python tools/test_android_content.py` covers the shared import
validator, streamed copy cancellation/limits and upload scheduling without starting
the game. Test the complete APK on Android for picker cancellation, file-provider
permissions, duplicate-name choices, deletion and keyboard/background recovery.
`python tools/test_android_storage.py --serial DEVICE --browse` tests the actual
provider and Bionic file lock in a separate small probe app with fresh app data.

For a native shader smoke test, package Horizon's included Golden Hour example:

```sh
cargo run -p horizon-shader-api --bin horizon-shader -- pack crates/horizon-shader-api/packs/golden_hour --out .local/golden_hour-1.0.0.hshader
```

Transfer the resulting file to the phone, import it under Shaders, then enable it
in Installed before Play, or in the game's Shader Packs settings. It changes the
sky and adds two post-processing passes; it is a functional example, not an
Android performance benchmark.

The Horizon catalog also lists source-only `.hnmod` mods so their availability is visible. These require rebuilding the game and cannot be installed into the current Android client; the list marks them as unsupported. `.hmod` file and HTTPS imports remain available.
