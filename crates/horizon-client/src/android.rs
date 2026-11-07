//! Android hooks of the game (Starling): what `game` reaches through `crate::android::`, and the
//! process's start-up on a NativeActivity.
//!
//! The cdylib (horizon-android, `android_lib.rs`) is only the entry points Android and the JVM look
//! up by name (`android_main`, `starling_on_create`, the `Java_...` native methods): they call the
//! functions here. `main` gives the process what the desktop binary gets from its launcher (the
//! game folder as the current directory and `HORIZON_HOME`, the command line) and then starts the
//! game as `entry::main` does.
//!
//! The game folder is `Starling` in shared storage once the player allows it (StarlingActivity
//! says so before this starts), else the app's own files. The prepared resources, the caches,
//! the import inbox and the test arguments always stay in the app's own files
//! (`HORIZON_PRIVATE`).
use crate::logging;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use winit::platform::android::activity::AndroidApp;

static APP: OnceLock<AndroidApp> = OnceLock::new();
static ARGS: OnceLock<Vec<String>> = OnceLock::new();
/// Shared storage's Starling folder, when the player allowed the app to use it.
static SHARED_FOLDER: OnceLock<PathBuf> = OnceLock::new();

/// `StarlingActivity.onStorage`, called on the UI thread before the game's thread starts.
pub fn set_shared_folder(folder: PathBuf) {
    let _ = SHARED_FOLDER.set(folder);
}

/// NativeActivity's entry point (the manifest's `android.app.func_name`, through the cdylib's
/// `starling_on_create`): the game of this library, or of the build with native mods
/// StarlingActivity loaded in its place (`STARLING_GAME_LIBRARY`, NativeMods.java). The activity
/// loaded that build first, for its Java native methods; dlopen finds it loaded, so the activity
/// and the game share one library.
///
/// # Safety
///
/// NativeActivity's call: an `ANativeActivity` and its saved state.
pub unsafe fn on_create(
    activity: *mut std::ffi::c_void,
    saved: *mut std::ffi::c_void,
    saved_size: usize,
) {
    unsafe extern "C" {
        /// android-activity's entry point, which runs `android_main`.
        fn ANativeActivity_onCreate(
            activity: *mut std::ffi::c_void,
            saved: *mut std::ffi::c_void,
            saved_size: usize,
        );
    }
    type OnCreate = unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, usize);
    let own: OnCreate = ANativeActivity_onCreate;
    if let Some(path) = std::env::var_os("STARLING_GAME_LIBRARY") {
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::CString::new(path.as_bytes()).unwrap_or_default();
        // SAFETY: NUL-terminated names; RTLD_NOW (2). The symbol is android-activity's entry
        // point in a build of this same source, so it has the signature declared above.
        let entry = unsafe {
            let build = dlopen(path.as_ptr(), 2);
            if build.is_null() {
                std::ptr::null_mut()
            } else {
                dlsym(build, c"ANativeActivity_onCreate".as_ptr())
            }
        };
        if !entry.is_null() && entry != own as *mut std::ffi::c_void {
            // SAFETY: see above.
            let entry = unsafe { std::mem::transmute::<*mut std::ffi::c_void, OnCreate>(entry) };
            return unsafe { entry(activity, saved, saved_size) };
        }
        logcat(
            log::Level::Error,
            "the build with native mods has no entry point: playing the APK's own game",
        );
    }
    // SAFETY: NativeActivity's own arguments, passed on.
    unsafe { own(activity, saved, saved_size) }
}

/// Run `f` with the activity, on the calling thread (attached to the VM for the call). None
/// before the game starts, or when the call threw (the exception is logged and cleared).
fn with_activity<R>(
    f: impl FnOnce(&mut jni::JNIEnv, &jni::objects::JObject) -> jni::errors::Result<R>,
) -> Option<R> {
    let app = APP.get()?;
    // SAFETY: the VM and the activity are NativeActivity's, alive as long as the process; the
    // activity is a global reference, which this neither deletes nor keeps.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let activity = unsafe { jni::objects::JObject::from_raw(app.activity_as_ptr().cast()) };
    let result = f(&mut env, &activity);
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    result.ok()
}

/// The platform hooks (horizon_client::platform): StarlingActivity's public methods.
fn open_folder(dir: &Path) -> bool {
    with_activity(|env, activity| {
        let path = env.new_string(dir.to_string_lossy())?;
        env.call_method(
            activity,
            "openFolder",
            "(Ljava/lang/String;)Z",
            &[jni::objects::JValue::Object(&path)],
        )?
        .z()
    })
    .unwrap_or(false)
}

fn pick_file() -> bool {
    with_activity(|env, activity| env.call_method(activity, "pickFile", "()Z", &[])?.z())
        .unwrap_or(false)
}

fn clipboard_get() -> Option<String> {
    with_activity(|env, activity| {
        let text = env
            .call_method(activity, "clipboardText", "()Ljava/lang/String;", &[])?
            .l()?;
        if text.is_null() {
            return Ok(None);
        }
        Ok(Some(String::from(env.get_string(&text.into())?)))
    })
    .flatten()
}

/// The folder Android extracted the APK's libraries to (ApplicationInfo.nativeLibraryDir), where
/// the Java launchers are.
fn library_dir() -> Option<PathBuf> {
    with_activity(|env, activity| {
        let info = env
            .call_method(
                activity,
                "getApplicationInfo",
                "()Landroid/content/pm/ApplicationInfo;",
                &[],
            )?
            .l()?;
        let dir = env
            .get_field(&info, "nativeLibraryDir", "Ljava/lang/String;")?
            .l()?;
        Ok(PathBuf::from(String::from(env.get_string(&dir.into())?)))
    })
}

/// A file of the APK's assets/, whole.
fn asset(name: &str) -> Option<Vec<u8>> {
    let name = std::ffi::CString::new(name).ok()?;
    let mut asset = APP.get()?.asset_manager().open(&name)?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut asset, &mut bytes).ok()?;
    Some(bytes)
}

/// `data` sealed by StarlingActivity.sealSecret (the Android Keystore's key).
fn seal_secret(data: &[u8]) -> Option<Vec<u8>> {
    secret_call("sealSecret", data)
}

/// What [`seal_secret`] sealed, by StarlingActivity.openSecret.
fn open_secret(sealed: &[u8]) -> Option<Vec<u8>> {
    secret_call("openSecret", sealed)
}

fn secret_call(method: &str, data: &[u8]) -> Option<Vec<u8>> {
    with_activity(|env, activity| {
        let input = env.byte_array_from_slice(data)?;
        let out = env
            .call_method(
                activity,
                method,
                "([B)[B",
                &[jni::objects::JValue::Object(&input)],
            )?
            .l()?;
        if out.is_null() {
            return Ok(None);
        }
        Ok(Some(
            env.convert_byte_array(jni::objects::JByteArray::from(out))?,
        ))
    })
    .flatten()
}

fn clipboard_set(text: &str) -> bool {
    with_activity(|env, activity| {
        let text = env.new_string(text)?;
        env.call_method(
            activity,
            "setClipboardText",
            "(Ljava/lang/String;)Z",
            &[jni::objects::JValue::Object(&text)],
        )?
        .z()
    })
    .unwrap_or(false)
}

/// The active network's DNS servers (StarlingActivity.dnsServers), for SRV lookups.
fn dns_servers() -> Vec<std::net::SocketAddr> {
    with_activity(|env, activity| {
        let list = env
            .call_method(activity, "dnsServers", "()[Ljava/lang/String;", &[])?
            .l()?;
        let list = jni::objects::JObjectArray::from(list);
        let mut servers = Vec::new();
        for i in 0..env.get_array_length(&list)? {
            let ip = env.get_object_array_element(&list, i)?;
            let ip = String::from(env.get_string(&ip.into())?);
            // An IPv6 server may carry its interface (`fe80::1%wlan0`).
            if let Ok(ip) = ip.split('%').next().unwrap_or_default().parse() {
                servers.push(std::net::SocketAddr::new(ip, 53));
            }
        }
        Ok(servers)
    })
    .unwrap_or_default()
}

/// The foreground service for `reason`, by StarlingActivity.keepWorking (WorkService.java).
fn keep_working(reason: &str, line: &str, on: bool) {
    with_activity(|env, activity| {
        let reason = env.new_string(reason)?;
        let line = env.new_string(line)?;
        env.call_method(
            activity,
            "keepWorking",
            "(Ljava/lang/String;Ljava/lang/String;Z)V",
            &[
                jni::objects::JValue::Object(&reason),
                jni::objects::JValue::Object(&line),
                jni::objects::JValue::Bool(u8::from(on)),
            ],
        )
        .map(drop)
    });
}

/// The system's browser on an https:// page, by StarlingActivity.openLink.
fn open_link(url: &str) -> bool {
    with_activity(|env, activity| {
        let url = env.new_string(url)?;
        env.call_method(
            activity,
            "openLink",
            "(Ljava/lang/String;)Z",
            &[jni::objects::JValue::Object(&url)],
        )?
        .z()
    })
    .unwrap_or(false)
}

/// Whether the player has allowed the Starling folder in shared storage (now: they may have just
/// come back from the setting).
pub fn file_access_allowed() -> bool {
    with_activity(|env, activity| {
        env.call_method(activity, "fileAccessAllowed", "()Z", &[])?
            .z()
    })
    .unwrap_or(false)
}

/// Whether Horizon Android is on the phone: the setup offers to bring its worlds and packs over.
pub fn horizon_android_installed() -> bool {
    with_activity(|env, activity| {
        env.call_method(activity, "horizonAndroidInstalled", "()Z", &[])?
            .z()
    })
    .unwrap_or(false)
}

/// The system's folder picker on Horizon Android's folder: the worlds and packs picked arrive in
/// the import inbox (StarlingActivity, HorizonImport.java). False when it could not open.
pub fn import_horizon_android() -> bool {
    with_activity(|env, activity| {
        env.call_method(activity, "importFromHorizonAndroid", "()Z", &[])?
            .z()
    })
    .unwrap_or(false)
}

/// Android's setting that allows it, in a task of its own: home and the icon come back to the
/// game.
pub fn open_file_access_settings() {
    with_activity(|env, activity| {
        env.call_method(activity, "openFileAccessSettings", "()V", &[])
            .map(drop)
    });
}

/// The game drew its first frame: the activity lets its window show, and the system's
/// starting window (the app icon) goes.
pub fn first_frame_shown() {
    with_activity(|env, activity| {
        env.call_method(activity, "firstFrame", "()V", &[])
            .map(drop)
    });
}

/// Put the on-screen keyboard away, however it was shown.
pub fn hide_keyboard() {
    with_activity(|env, activity| {
        env.call_method(activity, "hideKeyboard", "()V", &[])
            .map(drop)
    });
}

/// Start the game again in a new process (the game folder is chosen at start).
pub fn restart_app() {
    with_activity(|env, activity| env.call_method(activity, "restart", "()V", &[]).map(drop));
}

/// This APK's version (versionName), and whether it is a release, which only can install the
/// next release over itself (game/update_offer.rs).
pub fn app_version() -> Option<(String, bool)> {
    with_activity(|env, activity| {
        let name = env
            .call_method(activity, "versionName", "()Ljava/lang/String;", &[])?
            .l()?;
        let release = env.call_method(activity, "releaseBuild", "()Z", &[])?.z()?;
        let name = if name.is_null() {
            String::new()
        } else {
            String::from(env.get_string(&name.into())?)
        };
        Ok((name, release))
    })
    .filter(|(name, _)| !name.is_empty())
}

/// Android's installer on `name`, an APK the game downloaded to files/updates/.
pub fn install_update(name: &str) -> bool {
    with_activity(|env, activity| {
        let name = env.new_string(name)?;
        env.call_method(
            activity,
            "installUpdate",
            "(Ljava/lang/String;)Z",
            &[jni::objects::JValue::Object(&name)],
        )?
        .z()
    })
    .unwrap_or(false)
}

/// What the player made, moved with the game folder when it goes to shared storage: options.txt
/// last, so a copy cut short is redone next time.
const PLAYER_FILES: &[&str] = &[
    "saves",
    "backups",
    "resourcepacks",
    "datapacks",
    "shaderpacks",
    "horizon-mods",
    "screenshots",
    "servers.dat",
    "horizon-mods.txt",
    "horizon-graphics.txt",
    "horizon-extras.txt",
    "horizon-controller.txt",
    "horizon-player.txt",
    "horizon.txt",
    "touch-layout.json",
    "options.txt",
];

/// Folders whose images are the game's own (world icons, skins, pack art), kept out of the
/// gallery; screenshots are not.
const NO_MEDIA: &[&str] = &[
    "saves",
    "backups",
    "resourcepacks",
    "datapacks",
    "shaderpacks",
    "horizon-mods",
    "assets",
];

/// The game folder: shared storage's once allowed (and what the player made in the app's own
/// files moved there the first time), else the app's own files. Notes for the log, which
/// starts after.
fn game_folder(private: &Path, notes: &mut Vec<String>) -> PathBuf {
    let Some(shared) = SHARED_FOLDER.get() else {
        return private.to_path_buf();
    };
    let earlier = shared.with_file_name("Horizon");
    if !shared.exists() && earlier.join("options.txt").exists() {
        match std::fs::rename(&earlier, shared) {
            Ok(()) => notes.push(format!(
                "Horizon: renamed {} to {}",
                earlier.display(),
                shared.display()
            )),
            Err(e) => notes.push(format!(
                "Horizon: could not rename {} to {}: {e}",
                earlier.display(),
                shared.display()
            )),
        }
    }
    let probe = shared.join(".horizon-probe");
    if let Err(e) = std::fs::create_dir_all(shared).and_then(|()| std::fs::write(&probe, b"")) {
        notes.push(format!("Horizon: cannot use {}: {e}", shared.display()));
        return private.to_path_buf();
    }
    let _ = std::fs::remove_file(&probe);
    if !shared.join("options.txt").exists() && private.join("options.txt").exists() {
        match move_player_files(private, shared) {
            Ok(n) => notes.push(format!(
                "Horizon: moved {n} files and folders to {} (the old copies are in {})",
                shared.display(),
                private.join("moved-to-shared").display()
            )),
            Err(e) => {
                notes.push(format!(
                    "Horizon: could not move the game folder to {}: {e}; staying in {}",
                    shared.display(),
                    private.display()
                ));
                return private.to_path_buf();
            }
        }
    }
    for name in NO_MEDIA {
        let dir = shared.join(name);
        if std::fs::create_dir_all(&dir).is_ok() && !dir.join(".nomedia").exists() {
            let _ = std::fs::write(dir.join(".nomedia"), b"");
        }
    }
    shared.clone()
}

/// Copy the player's files to shared storage, then set the private copies aside.
fn move_player_files(private: &Path, shared: &Path) -> std::io::Result<usize> {
    let present: Vec<&str> = PLAYER_FILES
        .iter()
        .copied()
        .filter(|name| private.join(name).exists())
        .collect();
    for name in &present {
        copy_tree(&private.join(name), &shared.join(name))?;
    }
    let aside = private.join("moved-to-shared");
    std::fs::create_dir_all(&aside)?;
    for name in &present {
        let _ = std::fs::rename(private.join(name), aside.join(name));
    }
    Ok(present.len())
}

/// Copy a file or a folder, checking each file's size (links are left out).
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(from)?;
    if meta.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else if meta.is_file() {
        std::fs::copy(from, to)?;
        if std::fs::metadata(to)?.len() != meta.len() {
            return Err(std::io::Error::other(format!(
                "{} came out short",
                to.display()
            )));
        }
    }
    Ok(())
}

/// The on-screen keyboard's height in window pixels, from StarlingActivity's insets listener.
static IME_HEIGHT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// `StarlingActivity.onImeHeight` (packaging/android/java), called on the UI thread.
pub fn set_ime_height(pixels: i32) {
    IME_HEIGHT.store(pixels.max(0) as u32, std::sync::atomic::Ordering::Relaxed);
}

/// How far the on-screen keyboard covers the window from the bottom, in pixels.
pub fn ime_height() -> u32 {
    IME_HEIGHT.load(std::sync::atomic::Ordering::Relaxed)
}

/// Start or stop listening to the gyroscope (StarlingActivity.gyroscope); false when the phone
/// has none.
pub fn gyroscope(on: bool) -> bool {
    with_activity(|env, activity| {
        env.call_method(
            activity,
            "gyroscope",
            "(Z)Z",
            &[jni::objects::JValue::Bool(u8::from(on))],
        )?
        .z()
    })
    .unwrap_or(false)
}

/// What the gyroscope turned since the last call (radians about the device's x and y) and the
/// display's rotation (StarlingActivity.takeGyro). Asked every frame, so its array goes with a
/// local frame of its own.
pub fn take_gyro() -> Option<[f32; 3]> {
    with_activity(|env, activity| {
        env.with_local_frame(4, |env| {
            let turned = env.call_method(activity, "takeGyro", "()[F", &[])?.l()?;
            let turned = jni::objects::JFloatArray::from(turned);
            let mut out = [0.0f32; 3];
            env.get_float_array_region(&turned, 0, &mut out)?;
            Ok(out)
        })
    })
}

/// A short buzz of the phone (StarlingActivity.vibrate): the touch controls' feedback.
pub fn vibrate(millis: u32) {
    let millis = jni::objects::JValue::Int(millis.min(1000) as i32);
    with_activity(|env, activity| {
        env.call_method(activity, "vibrate", "(I)V", &[millis])
            .map(drop)
    });
}

/// What the keyboard's editor (TextMirror.java) did, for the game thread to apply in order
/// (game/touch/text_mirror.rs).
pub enum MirrorEvent {
    /// The whole text after an edit, the cursor and the selection's other end in characters,
    /// and the game's update the edit was made on.
    Edit {
        text: String,
        cursor: usize,
        anchor: usize,
        generation: i32,
    },
    /// The keyboard's action key (Send, Done...).
    Action,
    /// A key for the game: Android's KEYCODE_*.
    Key(i32),
}

/// The editor's events, from the UI thread, until the game's next frame.
static MIRROR_EVENTS: std::sync::Mutex<Vec<MirrorEvent>> = std::sync::Mutex::new(Vec::new());

fn mirror_event(event: MirrorEvent) {
    let mut events = MIRROR_EVENTS.lock().unwrap_or_else(|e| e.into_inner());
    // Each edit holds the whole text: of a run of them, the last is enough.
    if matches!(event, MirrorEvent::Edit { .. })
        && matches!(events.last(), Some(MirrorEvent::Edit { .. }))
    {
        events.pop();
    }
    events.push(event);
}

/// The editor's events since the last call.
pub fn take_mirror_events() -> Vec<MirrorEvent> {
    std::mem::take(&mut *MIRROR_EVENTS.lock().unwrap_or_else(|e| e.into_inner()))
}

/// `TextMirror.onMirrorEdit`, called on the UI thread.
pub fn mirror_edit(text: String, cursor: i32, anchor: i32, generation: i32) {
    mirror_event(MirrorEvent::Edit {
        text,
        cursor: cursor.max(0) as usize,
        anchor: anchor.max(0) as usize,
        generation,
    });
}

/// `TextMirror.onMirrorAction`, called on the UI thread.
pub fn mirror_action() {
    mirror_event(MirrorEvent::Action);
}

/// `TextMirror.onMirrorKey`, called on the UI thread.
pub fn mirror_key(code: i32) {
    mirror_event(MirrorEvent::Key(code));
}

/// The text field in the keyboard's editor: shown, focused and the keyboard up
/// (StarlingActivity.showTextMirror), or told the field's own change (updateTextMirror).
/// Positions are in characters, -1 to keep the editor's; `max` 0 is no limit.
pub fn text_mirror(
    show: bool,
    text: &str,
    cursor: i32,
    anchor: i32,
    max: i32,
    flags: i32,
    generation: i32,
) {
    use jni::objects::JValue;
    let method = if show {
        "showTextMirror"
    } else {
        "updateTextMirror"
    };
    with_activity(|env, activity| {
        let text = env.new_string(text)?;
        let called = env.call_method(
            activity,
            method,
            "(Ljava/lang/String;IIIII)V",
            &[
                JValue::Object(&text),
                JValue::Int(cursor),
                JValue::Int(anchor),
                JValue::Int(max),
                JValue::Int(flags),
                JValue::Int(generation),
            ],
        );
        // A thread attached before the call stays attached, and keeps its local references.
        env.delete_local_ref(text)?;
        called.map(drop)
    });
}

/// The keyboard and its editor put away (StarlingActivity.hideTextMirror).
pub fn hide_text_mirror() {
    with_activity(|env, activity| {
        env.call_method(activity, "hideTextMirror", "()V", &[])
            .map(drop)
    });
}

/// Controllers, the mouse and the keyboard: what winit's Android backend makes touches of or drops
/// (a pad's sticks become a finger, its buttons unknown keys; the mouse's hover, wheel and buttons
/// are lost), taken from each input event before it does (`set_input_hook`, third_party/winit).
mod input {
    use horizon_client::{
        gamepad::{
            PadState,
            android::{self as pad, AndroidPads, Axes},
        },
        input_mode,
    };
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use winit::event::MouseButton;
    use winit::platform::android::activity::{
        InputStatus,
        input::{Axis, InputEvent, KeyAction, KeyEvent, MotionAction, MotionEvent},
    };

    /// `InputDevice.SOURCE_MOUSE` and `SOURCE_MOUSE_RELATIVE` (the captured pointer's) without
    /// their class bits.
    const SOURCE_MOUSE: u32 = 0x2000;
    const SOURCE_MOUSE_RELATIVE: u32 = 0x0002_0000;
    /// `MotionEvent.BUTTON_PRIMARY`, `_SECONDARY` and `_TERTIARY`. The back and forward buttons
    /// are left to Android, which makes the Back and Forward keys of them.
    const BUTTONS: [(u32, MouseButton); 3] = [
        (1, MouseButton::Left),
        (2, MouseButton::Right),
        (4, MouseButton::Middle),
    ];
    /// The most mouse events kept for a game that is not reading them (a save on quit).
    const MOUSE_QUEUE: usize = 512;

    /// What a mouse did, for the game's next loop iteration (game/input_mode.rs).
    pub enum MouseEvent {
        /// The pointer moved to this window position, in pixels.
        Moved(f64, f64),
        Button(MouseButton, bool),
        /// The wheel, in lines: winit's `LineDelta` (right and up positive).
        Wheel(f32, f32),
        /// How far the mouse went, in pixels, right and down positive: the view turns by it
        /// while the game holds the pointer.
        Motion(f64, f64),
    }

    struct Mouse {
        /// The game reads the mouse's events (from its first loop iteration): before, the loading
        /// screen gets them as winit makes them, as a finger.
        taken: bool,
        events: Vec<MouseEvent>,
        buttons: u32,
        at: Option<(f32, f32)>,
    }

    static PADS: Mutex<AndroidPads> = Mutex::new(AndroidPads::new());
    static MOUSE: Mutex<Mouse> = Mutex::new(Mouse {
        taken: false,
        events: Vec::new(),
        buttons: 0,
        at: None,
    });
    /// A physical keyboard's key went down since the game last asked.
    static KEYBOARD_USED: AtomicBool = AtomicBool::new(false);

    /// The hook: the events taken are answered as handled; the others go on to winit.
    pub(super) fn hook(event: &InputEvent<'_>) -> Option<InputStatus> {
        match event {
            InputEvent::KeyEvent(key) => key_input(key),
            InputEvent::MotionEvent(motion) => motion_input(motion),
            _ => None,
        }
    }

    fn key_input(key: &KeyEvent<'_>) -> Option<InputStatus> {
        let source: u32 = key.source().into();
        let code = u32::from(key.key_code()) as i32;
        let down = match key.action() {
            KeyAction::Down => true,
            KeyAction::Up => false,
            _ => return None,
        };
        if let Some(button) = pad::pad_key(source, code) {
            let mut pads = PADS.lock().unwrap_or_else(|e| e.into_inner());
            pads.key(key.device_id(), button, down);
            return Some(InputStatus::Handled);
        }
        // winit turns it into a key event as on a computer; this only notes the keyboard in use.
        if down && input_mode::android_keyboard(source, key.device_id(), key.scan_code(), code) {
            KEYBOARD_USED.store(true, Ordering::Relaxed);
        }
        None
    }

    fn motion_input(motion: &MotionEvent<'_>) -> Option<InputStatus> {
        let source: u32 = motion.source().into();
        if pad::joystick(source) {
            if motion.pointer_count() > 0 {
                let pointer = motion.pointer_at_index(0);
                let axis = |a| pointer.axis_value(a);
                let axes = Axes {
                    x: axis(Axis::X),
                    y: axis(Axis::Y),
                    z: axis(Axis::Z),
                    rz: axis(Axis::Rz),
                    hat_x: axis(Axis::HatX),
                    hat_y: axis(Axis::HatY),
                    ltrigger: axis(Axis::Ltrigger),
                    rtrigger: axis(Axis::Rtrigger),
                    brake: axis(Axis::Brake),
                    gas: axis(Axis::Gas),
                };
                let mut pads = PADS.lock().unwrap_or_else(|e| e.into_inner());
                pads.motion(motion.device_id(), axes);
            }
            return Some(InputStatus::Handled);
        }
        let relative = source & SOURCE_MOUSE_RELATIVE != 0;
        if source & SOURCE_MOUSE == 0 && !relative {
            return None;
        }
        let mut mouse = MOUSE.lock().unwrap_or_else(|e| e.into_inner());
        if !mouse.taken {
            return None;
        }
        if motion.pointer_count() == 0 {
            return Some(InputStatus::Handled);
        }
        let mut events = Vec::new();
        let pointer = motion.pointer_at_index(0);
        if !relative && motion.action() != MotionAction::HoverExit {
            let at = (pointer.x(), pointer.y());
            if mouse.at != Some(at) {
                mouse.at = Some(at);
                events.push(MouseEvent::Moved(f64::from(at.0), f64::from(at.1)));
            }
        }
        // How far the mouse went, the moves batched into the event before it too: captured, its
        // x and y; free, its RELATIVE_X and RELATIVE_Y (the pointer's own, accelerated), which
        // turn the view while the game holds a pointer Android has not captured (yet).
        let (x, y) = if relative {
            (Axis::X, Axis::Y)
        } else {
            (Axis::RelativeX, Axis::RelativeY)
        };
        let (mut dx, mut dy) = (pointer.axis_value(x), pointer.axis_value(y));
        for earlier in pointer.history() {
            dx += earlier.axis_value(x);
            dy += earlier.axis_value(y);
        }
        if dx != 0.0 || dy != 0.0 {
            events.push(MouseEvent::Motion(f64::from(dx), f64::from(dy)));
        }
        if motion.action() == MotionAction::Scroll {
            let right = pointer.axis_value(Axis::Hscroll);
            let up = pointer.axis_value(Axis::Vscroll);
            if right != 0.0 || up != 0.0 {
                // winit's wheel moves the content: right is negative.
                events.push(MouseEvent::Wheel(-right, up));
            }
        }
        let buttons = motion.button_state().0;
        for (bit, button) in BUTTONS {
            if (buttons ^ mouse.buttons) & bit != 0 {
                events.push(MouseEvent::Button(button, buttons & bit != 0));
            }
        }
        mouse.buttons = buttons;
        if mouse.events.len() + events.len() <= MOUSE_QUEUE {
            mouse.events.append(&mut events);
        }
        Some(InputStatus::Handled)
    }

    /// The active controller's device id and state, `None` before one was used.
    pub fn pad_state() -> Option<(i32, PadState)> {
        PADS.lock().unwrap_or_else(|e| e.into_inner()).active()
    }

    /// The mouse's events since the last call; the game reads the mouse from the first.
    pub fn take_mouse_events() -> Vec<MouseEvent> {
        let mut mouse = MOUSE.lock().unwrap_or_else(|e| e.into_inner());
        mouse.taken = true;
        std::mem::take(&mut mouse.events)
    }

    /// Whether a physical keyboard's key went down since the last call.
    pub fn keyboard_used() -> bool {
        KEYBOARD_USED.swap(false, Ordering::Relaxed)
    }
}

pub use input::{MouseEvent, keyboard_used, pad_state, take_mouse_events};

/// Android took the captured pointer back while the game held it (StarlingActivity).
static POINTER_LOST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `StarlingActivity.onPointerCaptureLost`, called on the UI thread.
pub fn note_pointer_capture_lost() {
    POINTER_LOST.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Whether Android took the captured pointer back since the last call (focus lost, the system's
/// own shortcut).
pub fn pointer_capture_lost() -> bool {
    POINTER_LOST.swap(false, std::sync::atomic::Ordering::Relaxed)
}

/// Capture the mouse for the game's mouse look, or let it go (StarlingActivity.capturePointer):
/// captured, its moves come as relative motion and its pointer hides.
pub fn capture_pointer(capture: bool) {
    with_activity(|env, activity| {
        env.call_method(
            activity,
            "capturePointer",
            "(Z)V",
            &[jni::objects::JValue::Bool(u8::from(capture))],
        )
        .map(drop)
    });
}

/// A controller's maker, its USB vendor id (StarlingActivity.padVendor), for its button pictures.
pub fn pad_vendor(device: i32) -> Option<u16> {
    with_activity(|env, activity| {
        env.call_method(
            activity,
            "padVendor",
            "(I)I",
            &[jni::objects::JValue::Int(device)],
        )?
        .i()
    })
    .and_then(|vendor| u16::try_from(vendor).ok())
    .filter(|vendor| *vendor != 0)
}

/// A controller's name (StarlingActivity.padName), for the log; `None` once it is gone.
pub fn pad_name(device: i32) -> Option<String> {
    with_activity(|env, activity| {
        let name = env
            .call_method(
                activity,
                "padName",
                "(I)Ljava/lang/String;",
                &[jni::objects::JValue::Int(device)],
            )?
            .l()?;
        if name.is_null() {
            return Ok(None);
        }
        Ok(Some(String::from(env.get_string(&name.into())?)))
    })
    .flatten()
}

/// Vibrate a controller for `millis` ms, its heavy motor at `strong` and its light one at `weak`
/// (0 to 255): StarlingActivity.vibratePad.
pub fn vibrate_pad(device: i32, millis: i32, strong: i32, weak: i32) {
    use jni::objects::JValue;
    with_activity(|env, activity| {
        env.call_method(
            activity,
            "vibratePad",
            "(IIII)V",
            &[
                JValue::Int(device),
                JValue::Int(millis),
                JValue::Int(strong),
                JValue::Int(weak),
            ],
        )
        .map(drop)
    });
}

/// The activity, for the event loop (`EventLoopBuilderExtAndroid::with_android_app`).
pub fn android_app() -> AndroidApp {
    APP.get().expect("android_main sets the app first").clone()
}

/// The command line the desktop binary would get, without the program name.
pub fn args() -> Vec<String> {
    ARGS.get().cloned().unwrap_or_default()
}

/// The game's command line and environment: `--resources` in the game folder, then
/// `horizon-args.txt` there (written over adb for tests), which may override it: one argument per
/// line, or `NAME=value` for an environment variable (`HORIZON_GPU_CULL=0`, `WGPU_BACKEND=...`).
fn command_line(home: &Path) -> (Vec<String>, Vec<(String, String)>) {
    let mut args = vec![
        "--resources".to_owned(),
        home.join("resources").display().to_string(),
    ];
    let mut env = Vec::new();
    let text = std::fs::read_to_string(home.join("horizon-args.txt")).unwrap_or_default();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line.split_once('=') {
            Some((name, value))
                if !name.is_empty()
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') =>
            {
                env.push((name.to_owned(), value.to_owned()))
            }
            _ => args.push(line.to_owned()),
        }
    }
    (args, env)
}

/// The game's console lines in logcat, under the tag `starling`.
fn logcat(level: log::Level, message: &str) {
    use std::ffi::CString;
    let priority = match level {
        log::Level::Error => 6,
        log::Level::Warn => 5,
        log::Level::Info => 4,
        log::Level::Debug => 3,
        log::Level::Trace => 2,
    };
    let tag = c"starling";
    for line in message.lines() {
        let Ok(text) = CString::new(line.replace('\0', " ")) else {
            continue;
        };
        // SAFETY: both pointers are NUL-terminated strings that outlive the call.
        unsafe { __android_log_write(priority, tag.as_ptr(), text.as_ptr()) };
    }
}

#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(
        priority: std::ffi::c_int,
        tag: *const std::ffi::c_char,
        text: *const std::ffi::c_char,
    ) -> std::ffi::c_int;
}

/// bionic's `struct mallinfo`: every field a `size_t`.
#[repr(C)]
struct Mallinfo {
    arena: usize,
    ordblks: usize,
    smblks: usize,
    hblks: usize,
    hblkhd: usize,
    usmblks: usize,
    fsmblks: usize,
    /// Bytes allocated and not freed.
    uordblks: usize,
    fordblks: usize,
    keepcost: usize,
}

unsafe extern "C" {
    fn mallinfo() -> Mallinfo;
    fn dlopen(filename: *const std::ffi::c_char, flags: std::ffi::c_int) -> *mut std::ffi::c_void;
    fn dlsym(
        handle: *mut std::ffi::c_void,
        symbol: *const std::ffi::c_char,
    ) -> *mut std::ffi::c_void;
}

/// The native heap in use (the Stats overlay's `heap`).
fn heap_bytes() -> Option<u64> {
    // SAFETY: mallinfo takes nothing and returns a plain struct.
    Some(unsafe { mallinfo() }.uordblks as u64)
}

/// The NDK's thermal API (AThermal, Android 11; the headroom Android 12), looked up at run time
/// so the library still loads on Android 8.
struct Thermal {
    manager: usize,
    status: Option<unsafe extern "C" fn(usize) -> std::ffi::c_int>,
    headroom: Option<unsafe extern "C" fn(usize, std::ffi::c_int) -> f32>,
}

fn thermal() -> Option<&'static Thermal> {
    static THERMAL: OnceLock<Option<Thermal>> = OnceLock::new();
    THERMAL
        .get_or_init(|| {
            // SAFETY: dlopen and dlsym get NUL-terminated names; each symbol found is cast to the
            // signature the NDK headers give it (thermal.h), and the manager it acquires lives
            // for the process.
            unsafe {
                let lib = dlopen(c"libandroid.so".as_ptr(), 2 /* RTLD_NOW */);
                if lib.is_null() {
                    return None;
                }
                let acquire = dlsym(lib, c"AThermal_acquireManager".as_ptr());
                if acquire.is_null() {
                    return None;
                }
                let acquire: unsafe extern "C" fn() -> usize = std::mem::transmute(acquire);
                let manager = acquire();
                if manager == 0 {
                    return None;
                }
                let symbol = |name: &std::ffi::CStr| {
                    let s = dlsym(lib, name.as_ptr());
                    (!s.is_null()).then_some(s)
                };
                Some(Thermal {
                    manager,
                    status: symbol(c"AThermal_getCurrentThermalStatus")
                        .map(|s| std::mem::transmute(s)),
                    headroom: symbol(c"AThermal_getThermalHeadroom")
                        .map(|s| std::mem::transmute(s)),
                })
            }
        })
        .as_ref()
}

fn thermal_status() -> Option<i32> {
    let t = thermal()?;
    // SAFETY: the function and the manager came from the NDK (`thermal`).
    t.status.map(|f| unsafe { f(t.manager) })
}

fn thermal_headroom() -> Option<f32> {
    let t = thermal()?;
    // SAFETY: as `thermal_status`. The forecast is for 10 s ahead; NaN when asked too often.
    t.headroom
        .map(|f| unsafe { f(t.manager, 10) })
        .filter(|h| h.is_finite())
}

/// Functions of libandroid.so newer than Android 8, looked up at run time like `thermal`.
struct FrameHints {
    /// ADPF's performance hint (Android 13): the manager's session for the render thread.
    create: Option<unsafe extern "C" fn(usize, *const i32, usize, i64) -> usize>,
    manager: usize,
    update: Option<unsafe extern "C" fn(usize, i64) -> std::ffi::c_int>,
    report: Option<unsafe extern "C" fn(usize, i64) -> std::ffi::c_int>,
    /// ANativeWindow_setFrameRate (Android 11).
    set_frame_rate: Option<unsafe extern "C" fn(*mut std::ffi::c_void, f32, i8) -> i32>,
}

fn frame_hints() -> &'static FrameHints {
    static HINTS: OnceLock<FrameHints> = OnceLock::new();
    HINTS.get_or_init(|| {
        // SAFETY: as `thermal`: NUL-terminated names, each symbol cast to its signature in the
        // NDK headers (performance_hint.h, native_window.h); the manager lives for the process.
        unsafe {
            let lib = dlopen(c"libandroid.so".as_ptr(), 2 /* RTLD_NOW */);
            let symbol = |name: &std::ffi::CStr| {
                let s = if lib.is_null() {
                    std::ptr::null_mut()
                } else {
                    dlsym(lib, name.as_ptr())
                };
                (!s.is_null()).then_some(s)
            };
            let manager = symbol(c"APerformanceHint_getManager")
                .map(|f| {
                    std::mem::transmute::<*mut std::ffi::c_void, unsafe extern "C" fn() -> usize>(
                        f,
                    )()
                })
                .unwrap_or(0);
            FrameHints {
                create: symbol(c"APerformanceHint_createSession").map(|s| std::mem::transmute(s)),
                manager,
                update: symbol(c"APerformanceHint_updateTargetWorkDuration")
                    .map(|s| std::mem::transmute(s)),
                report: symbol(c"APerformanceHint_reportActualWorkDuration")
                    .map(|s| std::mem::transmute(s)),
                set_frame_rate: symbol(c"ANativeWindow_setFrameRate")
                    .map(|s| std::mem::transmute(s)),
            }
        }
    })
}

unsafe extern "C" {
    fn gettid() -> i32;
    fn _exit(status: std::ffi::c_int) -> !;
}

/// The display's fastest refresh rate, asked of the activity once (0: unknown, no preference).
fn max_refresh_rate() -> f32 {
    static RATE: OnceLock<f32> = OnceLock::new();
    *RATE.get_or_init(|| {
        with_activity(|env, activity| env.call_method(activity, "maxRefreshRate", "()F", &[])?.f())
            .unwrap_or(0.0)
    })
}

/// The render thread's hint session, its target, and the window and rate last voted for.
#[derive(Default)]
struct HintState {
    session: usize,
    target_ns: i64,
    window: usize,
    voted: f32,
}

thread_local! {
    static HINT_STATE: std::cell::RefCell<HintState> = std::cell::RefCell::default();
}

/// Tell Android about a frame the render thread finished: its CPU time against the time the
/// frame cap allows (ADPF), so the scheduler raises the clock or moves the thread to a bigger
/// core when a tick makes a frame late, rather than following a load average; and the frame
/// cap as the display's refresh rate, so a 60 FPS cap gets a 60 Hz screen (every frame shown
/// as long) instead of a 120 Hz one alternating one and two refreshes. `fps_limit` 260 and up
/// is no cap.
pub fn frame_hint(cpu: std::time::Duration, fps_limit: i32) {
    let hints = frame_hints();
    let cap = (fps_limit < 260).then(|| fps_limit.clamp(10, 240));
    let target_ns = 1_000_000_000 / i64::from(cap.unwrap_or(120));
    HINT_STATE.with_borrow_mut(|state| {
        if state.session == 0
            && hints.manager != 0
            && let Some(create) = hints.create
        {
            // SAFETY: the manager came from APerformanceHint_getManager; one thread id, this one.
            state.session = unsafe {
                let tid = gettid();
                create(hints.manager, &tid, 1, target_ns)
            };
            state.target_ns = target_ns;
        }
        if state.session != 0 {
            // SAFETY: a session this thread created, never closed.
            unsafe {
                if state.target_ns != target_ns
                    && let Some(update) = hints.update
                {
                    update(state.session, target_ns);
                    state.target_ns = target_ns;
                }
                if let Some(report) = hints.report {
                    report(state.session, (cpu.as_nanos() as i64).max(1));
                }
            }
        }
        let Some(set_frame_rate) = hints.set_frame_rate else {
            return;
        };
        let Some(window) = android_app().native_window() else {
            return;
        };
        // No cap: the screen's fastest rate (Android's own choice may stay at 60 Hz).
        let rate = cap.map_or_else(max_refresh_rate, |c| c as f32);
        let ptr = window.ptr().as_ptr() as usize;
        if (state.window, state.voted) != (ptr, rate) {
            // SAFETY: the window is held for the call; 0 is
            // ANATIVEWINDOW_FRAME_RATE_COMPATIBILITY_DEFAULT, and a rate of 0 is no preference.
            unsafe { set_frame_rate(ptr as *mut std::ffi::c_void, rate, 0) };
            state.window = ptr;
            state.voted = rate;
        }
    });
}

/// Whether `android_main` ran in this process already: a second run ends the process
/// (see [`main`]).
static STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The process's start-up, from the cdylib's `android_main`: the game folder, the environment
/// (`HORIZON_HOME`, `HORIZON_PRIVATE`, `horizon-args.txt`'s), the platform hooks, then the game as
/// the desktop binary starts it (`entry::start`), and the activity's end.
pub fn main(app: AndroidApp) {
    // winit's event loop is made once a process. Android can start a new activity in a process
    // the game is ending (a restart into a world with other mods did: "EventLoop can't be
    // recreated", reported as a crash): end it at once, and the activity's next start is fresh.
    if STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        logcat(
            log::Level::Info,
            "a second start in a process the game ran in: ending it",
        );
        // SAFETY: ends the process at once, as the end of the first run does.
        unsafe { _exit(0) }
    }
    crate::telemetry::mark_start();
    // The app's own files: resources, caches, the import inbox, test arguments.
    let private: PathBuf = app
        .internal_data_path()
        .unwrap_or_else(|| PathBuf::from("/data/local/tmp/horizon"));
    let _ = std::fs::create_dir_all(&private);
    let mut notes = Vec::new();
    let home = game_folder(&private, &mut notes);
    let (args, env) = command_line(&private);
    // SAFETY: no other thread of ours runs yet; the game reads these from its threads afterwards.
    unsafe {
        std::env::set_var("HORIZON_HOME", &home);
        std::env::set_var("HORIZON_PRIVATE", &private);
        for (name, value) in &env {
            std::env::set_var(name, value);
        }
        // Starling is not Horizon: no live count ping to Horizon's telemetry server (and no
        // other sending) unless the test arguments ask; a phone has no Discord to show to.
        for (name, value) in [
            ("HORIZON_TELEMETRY_URL", "off"),
            ("HORIZON_NO_DISCORD", "1"),
        ] {
            if std::env::var_os(name).is_none() {
                std::env::set_var(name, value);
            }
        }
    }
    if let Err(e) = std::env::set_current_dir(&home) {
        logcat(
            log::Level::Error,
            &format!("Horizon: cannot enter {}: {e}", home.display()),
        );
    }
    logging::set_console(logcat);
    crate::system_stats::set_probes(crate::system_stats::Probes {
        heap_bytes,
        thermal_status,
        thermal_headroom,
    });
    crate::platform::set(crate::platform::Hooks {
        open_folder,
        pick_file,
        clipboard_get,
        clipboard_set,
        open_link,
        seal: seal_secret,
        open: open_secret,
        library_dir,
        asset,
        keep_working,
    });
    horizon_protocol::set_name_servers(dns_servers);
    let _ = ARGS.set(args.clone());
    let _ = APP.set(app);
    crate::entry::start(Default::default(), || {
        log::info!(
            "Horizon: Android, game folder {}, private files {}, arguments {args:?}, environment {env:?}",
            home.display(),
            private.display()
        );
        for note in notes {
            log::info!("{note}");
        }
        // Read from the APK's assets, which the app above opens.
        crate::preparer::put_shader_pack(&home);
        // The saved Microsoft sign-in renews while the game loads.
        crate::accounts::resume();
        winit::platform::android::set_input_hook(input::hook);
    });
    finish_and_exit()
}

/// The end of the game's run, normal or not. Quit leaves the activity alive, and Android starts
/// again an app whose process ends under its top activity, as after a crash: finish it, and its
/// onDestroy ends the process.
pub fn finish_and_exit() -> ! {
    with_activity(|env, activity| env.call_method(activity, "finish", "()V", &[]).map(drop));
    std::thread::sleep(std::time::Duration::from_secs(3));
    // Else, or once the activity is gone: Android may keep the process and start a new activity
    // in it, but winit's event loop cannot be made twice in one process, so end it and the next
    // launch is a fresh one. `_exit`, not `exit`: the C library's exit handlers tear down mutexes
    // that the app's Java threads (hwui) still use, and the process would abort instead.
    // SAFETY: ends the process at once; the log was flushed before.
    unsafe { _exit(0) }
}
