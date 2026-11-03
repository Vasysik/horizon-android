//! Horizon on Android (Starling): the cdylib's entry points.
//!
//! The crate root of horizon-android (crates/horizon-android/Cargo.toml). Android and the JVM look
//! these up by name in the library, so they live here, in the library that is linked last; each
//! one calls `horizon_client::android`, where the game's Android hooks and its start-up are.
#![cfg(target_os = "android")]

use winit::platform::android::activity::AndroidApp;

/// NativeActivity's entry point (the manifest's `android.app.func_name`): the game of this
/// library, or of the build with native mods StarlingActivity loaded in its place.
///
/// # Safety
///
/// NativeActivity's call: an `ANativeActivity` and its saved state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn starling_on_create(
    activity: *mut std::ffi::c_void,
    saved: *mut std::ffi::c_void,
    saved_size: usize,
) {
    // SAFETY: NativeActivity's own arguments, passed on.
    unsafe { horizon_client::android::on_create(activity, saved, saved_size) }
}

/// android-activity's entry point calls it on the game's thread.
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    horizon_client::android::main(app);
}

/// `StarlingActivity.onStorage`, called on the UI thread before the game's thread starts.
#[unsafe(no_mangle)]
extern "system" fn Java_io_github_juanluci_starling_StarlingActivity_onStorage(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    folder: jni::objects::JString,
) {
    if folder.is_null() {
        return;
    }
    if let Ok(folder) = env.get_string(&folder) {
        horizon_client::android::set_shared_folder(String::from(folder).into());
    }
}

/// `StarlingActivity.onImeHeight` (packaging/android/java), called on the UI thread.
#[unsafe(no_mangle)]
extern "system" fn Java_io_github_juanluci_starling_StarlingActivity_onImeHeight(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    pixels: i32,
) {
    horizon_client::android::set_ime_height(pixels);
}

/// `TextMirror.onMirrorEdit`, called on the UI thread.
#[unsafe(no_mangle)]
extern "system" fn Java_io_github_juanluci_starling_TextMirror_onMirrorEdit(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    text: jni::objects::JString,
    cursor: i32,
    anchor: i32,
    generation: i32,
) {
    let Ok(text) = env.get_string(&text) else {
        return;
    };
    horizon_client::android::mirror_edit(String::from(text), cursor, anchor, generation);
}

/// `TextMirror.onMirrorAction`, called on the UI thread.
#[unsafe(no_mangle)]
extern "system" fn Java_io_github_juanluci_starling_TextMirror_onMirrorAction(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
) {
    horizon_client::android::mirror_action();
}

/// `TextMirror.onMirrorKey`, called on the UI thread.
#[unsafe(no_mangle)]
extern "system" fn Java_io_github_juanluci_starling_TextMirror_onMirrorKey(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    code: i32,
) {
    horizon_client::android::mirror_key(code);
}

/// `StarlingActivity.onPointerCaptureLost`, called on the UI thread.
#[unsafe(no_mangle)]
extern "system" fn Java_io_github_juanluci_starling_StarlingActivity_onPointerCaptureLost(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
) {
    horizon_client::android::note_pointer_capture_lost();
}
