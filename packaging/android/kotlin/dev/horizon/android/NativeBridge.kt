package dev.horizon.android

import android.app.Activity

/** The Rust launcher's state and actions (`crates/horizon-client/src/android/bridge.rs`). */
object NativeBridge {
    init {
        System.loadLibrary("horizon_android")
    }

    /** Registers [activity] for Android services and prepares the launcher once. */
    @JvmStatic external fun start(activity: Activity, home: String)

    /** The whole launcher state as JSON; call from the main thread. */
    @JvmStatic external fun snapshot(): String?

    /** Runs a launcher action: `{"ok": …}` or `{"error": "…"}`. */
    @JvmStatic external fun call(method: String, arg: String): String?

    /** Hands a picked document to the content importer. */
    @JvmStatic external fun importDocument(activity: Activity, kind: String, uri: String?)
}
