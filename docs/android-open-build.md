# Open storage build (modified)

Changes against the stock Android fork:

* **Data folder in shared storage.** With "All files access" granted (Android 11+) or storage
  permission (Android 9-10) the game keeps its data in `/storage/emulated/0/Horizon`. Without the
  permission it falls back to the private app folder, as before. To change the folder name, edit
  `EXTERNAL_DIR` in `packaging/android/java/dev/horizon/android/HorizonHome.java`.
* **"All files" document root.** The system Files app (Open Files in the launcher) now shows an
  extra `All files` folder: the whole data folder, dotfiles included, writable while the game runs.
  Account and session files are inside it, so do not share that folder.
* **Permissions.** `MANAGE_EXTERNAL_STORAGE`, `READ/WRITE_EXTERNAL_STORAGE`. The app asks on launch;
  after granting, reopen the app once so the new folder is used.

Existing worlds stay in the private folder; copy them over once (or use the `All files` root while
the old folder is still active). Skins (Wardrobe) and Ely.by sign-in were already part of this fork.
Not built or tested in the environment that produced this change.
