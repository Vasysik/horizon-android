//! Android system input and platform services, called from the Rust client.
use horizon_client::osk::Output;
use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JObject, JValue},
};
use std::cell::RefCell;
thread_local! {
    static EDIT: RefCell<Option<String>> = const { RefCell::new(None) };
    static MULTILINE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static VIBRATOR: RefCell<Option<Option<GlobalRef>>> = const { RefCell::new(None) };
    static VIBRATING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static PAD_VIBRATOR: RefCell<Option<GlobalRef>> = const { RefCell::new(None) };
    static PAD_HAPTIC_DEVICE: std::cell::Cell<Option<i32>> = const { std::cell::Cell::new(None) };
}

fn with_vibrator(f: impl FnOnce(&mut JNIEnv<'_>, &JObject<'_>) -> jni::errors::Result<()>) {
    crate::APP.with(|slot| {
        let app = slot.borrow();
        let Some(app) = app.as_ref() else {
            return;
        };
        // AndroidApp keeps the VM and activity alive for this callback.
        let Ok(vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
            return;
        };
        let Ok(mut env) = vm.attach_current_thread() else {
            return;
        };
        let result = env.with_local_frame(16, |env| -> jni::errors::Result<()> {
            VIBRATOR.with(|cache| {
                let mut cache = cache.borrow_mut();
                if cache.is_none() {
                    let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
                    let name = JObject::from(env.new_string("vibrator")?);
                    let service = env
                        .call_method(
                            &activity,
                            "getSystemService",
                            "(Ljava/lang/String;)Ljava/lang/Object;",
                            &[JValue::Object(&name)],
                        )?
                        .l()?;
                    let supported = !service.is_null()
                        && env.call_method(&service, "hasVibrator", "()Z", &[])?.z()?;
                    *cache = Some(if supported {
                        Some(env.new_global_ref(service)?)
                    } else {
                        None
                    });
                }
                if let Some(service) = cache.as_ref().and_then(|s| s.as_ref()) {
                    f(env, service.as_obj())?;
                }
                Ok(())
            })
        });
        if let Err(error) = result {
            // A device without a vibrator must not leave a pending Java exception.
            let _ = env.exception_clear();
            log::warn!("Android haptics: {error}");
        }
    });
}
pub fn vibrate(pulse: horizon_client::gamepad::Pulse) {
    let amplitude = (pulse.strong.max(pulse.weak).clamp(0.0, 1.0) * 255.0).round() as i32;
    if amplitude == 0 || pulse.millis == 0 {
        return;
    }
    if vibrate_gamepad(pulse, amplitude) {
        return;
    }
    with_vibrator(|env, service| {
        let effect = env
            .call_static_method(
                "android/os/VibrationEffect",
                "createOneShot",
                "(JI)Landroid/os/VibrationEffect;",
                &[
                    JValue::Long(i64::from(pulse.millis.min(1000))),
                    JValue::Int(amplitude),
                ],
            )?
            .l()?;
        env.call_method(
            service,
            "vibrate",
            "(Landroid/os/VibrationEffect;)V",
            &[JValue::Object(&effect)],
        )?;
        VIBRATING.with(|v| v.set(true));
        Ok(())
    });
}
pub fn cancel_vibration() {
    cancel_gamepad_vibration();
    if !VIBRATING.with(|v| v.replace(false)) {
        return;
    }
    with_vibrator(|env, service| {
        env.call_method(service, "cancel", "()V", &[])?;
        Ok(())
    });
}

fn cancel_gamepad_vibration() {
    let vibrator = PAD_VIBRATOR.with(|v| v.borrow_mut().take());
    if let Some(vibrator) = vibrator {
        with_context(|env, _| {
            env.call_method(vibrator.as_obj(), "cancel", "()V", &[])?;
            Ok(())
        });
    }
}

/// Use the vibrator advertised by the active InputDevice, never an unrelated pad.
/// Unsupported USB/Bluetooth firmware retains the existing phone feedback.
fn vibrate_gamepad(pulse: horizon_client::gamepad::Pulse, amplitude: i32) -> bool {
    let Some(id) = horizon_client::android_runtime::active_gamepad_id() else {
        return false;
    };
    with_context(|env, _| {
        let device = env
            .call_static_method(
                "android/view/InputDevice",
                "getDevice",
                "(I)Landroid/view/InputDevice;",
                &[JValue::Int(id)],
            )?
            .l()?;
        if device.is_null() {
            return Ok(false);
        }
        let sdk = env
            .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
            .i()?;
        let first_probe = PAD_HAPTIC_DEVICE.with(|last| last.replace(Some(id)) != Some(id));
        if sdk >= 31 {
            let manager = env.call_method(
                &device, "getVibratorManager", "()Landroid/os/VibratorManager;", &[],
            )?.l()?;
            let ids = jni::objects::JIntArray::from(
                env.call_method(&manager, "getVibratorIds", "()[I", &[])?.l()?,
            );
            let count = env.get_array_length(&ids)?;
            if first_probe {
                log::info!("Android controller haptics: device {id}, {count} vibrator(s)");
            }
            if count > 0 {
                let effect = env.call_static_method(
                    "android/os/VibrationEffect", "createOneShot", "(JI)Landroid/os/VibrationEffect;",
                    &[JValue::Long(i64::from(pulse.millis.min(1000))), JValue::Int(amplitude)],
                )?.l()?;
                // The default vibrator addresses only one motor. Send the mono
                // pulse through the InputDevice's manager to every advertised motor.
                let combined = env.call_static_method(
                    "android/os/CombinedVibration", "createParallel",
                    "(Landroid/os/VibrationEffect;)Landroid/os/CombinedVibration;",
                    &[JValue::Object(&effect)],
                )?.l()?;
                env.call_method(&manager, "vibrate", "(Landroid/os/CombinedVibration;)V",
                    &[JValue::Object(&combined)])?;
                let reference = env.new_global_ref(&manager)?;
                PAD_VIBRATOR.with(|v| *v.borrow_mut() = Some(reference));
                return Ok(true);
            }
        }
        // Some vendor drivers expose only the legacy device vibrator, even
        // on Android versions which have VibratorManager.
        let vibrator = env.call_method(&device, "getVibrator", "()Landroid/os/Vibrator;", &[])?.l()?;
        if !env.call_method(&vibrator, "hasVibrator", "()Z", &[])?.z()? {
            if first_probe {
                log::warn!("Android controller haptics: device {id} exposes no vibrator; using phone feedback");
            }
            return Ok(false);
        }
        let effect = env
            .call_static_method(
                "android/os/VibrationEffect",
                "createOneShot",
                "(JI)Landroid/os/VibrationEffect;",
                &[
                    JValue::Long(i64::from(pulse.millis.min(1000))),
                    JValue::Int(amplitude),
                ],
            )?
            .l()?;
        env.call_method(
            &vibrator,
            "vibrate",
            "(Landroid/os/VibrationEffect;)V",
            &[JValue::Object(&effect)],
        )?;
        let reference = env.new_global_ref(&vibrator)?;
        PAD_VIBRATOR.with(|v| *v.borrow_mut() = Some(reference));
        Ok(true)
    })
    .unwrap_or(false)
}

pub(crate) fn with_context<R>(
    f: impl FnOnce(&mut JNIEnv<'_>, &JObject<'_>) -> jni::errors::Result<R>,
) -> Option<R> {
    crate::APP.with(|slot| {
        let app = slot.borrow();
        let app = app.as_ref()?;
        let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        let result = env.with_local_frame(16, |env| {
            let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
            f(env, &activity)
        });
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                let _ = env.exception_clear();
                log::warn!("Android platform: {error}");
                None
            }
        }
    })
}
pub fn clipboard_get() -> Option<String> {
    with_context(|env, activity| {
        let name = JObject::from(env.new_string("clipboard")?);
        let service = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&name)],
            )?
            .l()?;
        let clip = env
            .call_method(
                &service,
                "getPrimaryClip",
                "()Landroid/content/ClipData;",
                &[],
            )?
            .l()?;
        if clip.is_null() || env.call_method(&clip, "getItemCount", "()I", &[])?.i()? == 0 {
            return Ok(String::new());
        }
        let item = env
            .call_method(
                &clip,
                "getItemAt",
                "(I)Landroid/content/ClipData$Item;",
                &[JValue::Int(0)],
            )?
            .l()?;
        // Only paste text. Do not resolve content URIs or read clipboard files.
        let text = env
            .call_method(&item, "getText", "()Ljava/lang/CharSequence;", &[])?
            .l()?;
        if text.is_null() {
            return Ok(String::new());
        }
        let text = jni::objects::JString::from(
            env.call_method(text, "toString", "()Ljava/lang/String;", &[])?
                .l()?,
        );
        let value = env.get_string(&text)?.into();
        Ok(value)
    })
}

pub fn set_orientation(orientation: crate::catalog::Orientation) {
    with_context(|env, activity| {
        env.call_method(
            activity,
            "setRequestedOrientation",
            "(I)V",
            &[JValue::Int(orientation.android_value())],
        )?;
        Ok(())
    });
}

pub fn cache_dir() -> Option<std::path::PathBuf> {
    with_context(|env, activity| {
        let file = env
            .call_method(activity, "getCacheDir", "()Ljava/io/File;", &[])?
            .l()?;
        let path = jni::objects::JString::from(
            env.call_method(&file, "getAbsolutePath", "()Ljava/lang/String;", &[])?
                .l()?,
        );
        let text: String = env.get_string(&path)?.into();
        Ok(std::path::PathBuf::from(text))
    })
}

pub fn open_repository() -> bool {
    open_url("https://github.com/Vasysik/horizon-android")
}
/// Browse our SAF directory through the system file manager, never a private path.
pub fn open_documents(id: &str) -> bool {
    with_context(|env, activity| {
        let package = env
            .call_method(activity, "getPackageName", "()Ljava/lang/String;", &[])?
            .l()?;
        let package: String = env.get_string((&package).into())?.into();
        let authority = JObject::from(env.new_string(format!("{package}.documents"))?);
        let document = JObject::from(env.new_string(id)?);
        let uri = env
            .call_static_method(
                "android/provider/DocumentsContract",
                "buildDocumentUri",
                "(Ljava/lang/String;Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&authority), JValue::Object(&document)],
            )?
            .l()?;
        let action = JObject::from(env.new_string("android.intent.action.VIEW")?);
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )?;
        let mime = JObject::from(env.new_string("vnd.android.document/directory")?);
        env.call_method(
            &intent,
            "setDataAndType",
            "(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&uri), JValue::Object(&mime)],
        )?;
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::Int(3)],
        )?;
        env.call_method(
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(())
    })
    .is_some()
}
pub fn open_url(url: &str) -> bool {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return false;
    }
    with_context(|env, activity| {
        let address = JObject::from(env.new_string(url)?);
        let uri = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&address)],
            )?
            .l()?;
        let action = JObject::from(env.new_string("android.intent.action.VIEW")?);
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[JValue::Object(&action), JValue::Object(&uri)],
        )?;
        env.call_method(
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(())
    })
    .is_some()
}
/// Public Android power/thermal status; no device identifiers or sysfs access.
pub fn performance_power() -> serde_json::Value {
    with_context(|env, activity| {
        let name = JObject::from(env.new_string("power")?);
        let power = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&name)],
            )?
            .l()?;
        let sdk = env
            .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
            .i()?;
        let saver = env
            .call_method(&power, "isPowerSaveMode", "()Z", &[])?
            .z()?;
        let thermal = if sdk >= 29 {
            Some(
                env.call_method(&power, "getCurrentThermalStatus", "()I", &[])?
                    .i()?,
            )
        } else {
            None
        };
        Ok(serde_json::json!({"android_sdk":sdk,"power_save":saver,"thermal_status":thermal}))
    })
    .unwrap_or(serde_json::Value::Null)
}
pub fn clipboard_set(text: &str) {
    let _ = clipboard_write(text, false);
}
pub fn copy_login_code(text: &str) -> bool {
    clipboard_write(text, true)
}
fn clipboard_write(text: &str, sensitive: bool) -> bool {
    with_context(|env, activity| {
        let name = JObject::from(env.new_string("clipboard")?);
        let service = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&name)],
            )?
            .l()?;
        let label = JObject::from(env.new_string("Horizon")?);
        let value = JObject::from(env.new_string(text)?);
        let clip = env
            .call_static_method(
                "android/content/ClipData",
                "newPlainText",
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                &[JValue::Object(&label), JValue::Object(&value)],
            )?
            .l()?;
        if sensitive {
            let extras = env.new_object("android/os/PersistableBundle", "()V", &[])?;
            let key = JObject::from(env.new_string("android.content.extra.IS_SENSITIVE")?);
            env.call_method(
                &extras,
                "putBoolean",
                "(Ljava/lang/String;Z)V",
                &[JValue::Object(&key), JValue::Bool(1)],
            )?;
            let description = env
                .call_method(
                    &clip,
                    "getDescription",
                    "()Landroid/content/ClipDescription;",
                    &[],
                )?
                .l()?;
            env.call_method(
                &description,
                "setExtras",
                "(Landroid/os/PersistableBundle;)V",
                &[JValue::Object(&extras)],
            )?;
        }
        env.call_method(
            service,
            "setPrimaryClip",
            "(Landroid/content/ClipData;)V",
            &[JValue::Object(&clip)],
        )?;
        Ok(())
    })
    .is_some()
}

pub fn active() -> bool {
    EDIT.with(|e| e.borrow().is_some())
}

pub fn finish() {
    with_context(|env, activity| {
        env.call_method(activity, "finish", "()V", &[])?;
        Ok(())
    });
}

pub fn capture_mouse(capture: bool) -> bool {
    if crate::native_ime::capture_mouse(capture) {
        return true;
    }
    with_context(|env, activity| {
        let window = env
            .call_method(activity, "getWindow", "()Landroid/view/Window;", &[])?
            .l()?;
        let view = env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?
            .l()?;
        env.call_method(
            &view,
            if capture {
                "requestPointerCapture"
            } else {
                "releasePointerCapture"
            },
            "()V",
            &[],
        )?;
        Ok(())
    })
    .is_some()
}

pub fn show(text: &str) {
    show_editor(text, false);
}
pub fn show_multiline(text: &str) {
    show_editor(text, true);
}
fn show_editor(text: &str, multiline: bool) {
    MULTILINE.with(|mode| mode.set(multiline));
    if crate::native_ime::show(text) {
        EDIT.with(|e| *e.borrow_mut() = Some(text.into()));
    } else {
        log::warn!("Android system keyboard is unavailable");
    }
}
pub fn hide() {
    EDIT.with(|e| *e.borrow_mut() = None);
    crate::native_ime::hide();
}

pub fn poll() -> Vec<Output> {
    EDIT.with(|edit| {
        let mut edit = edit.borrow_mut();
        let Some(old) = edit.as_mut() else {
            return vec![];
        };
        let Some(new) = crate::native_ime::text() else {
            return vec![];
        };
        if new == *old {
            return vec![];
        }
        let out =
            horizon_client::ime_edit::editor_changes(old, &new, MULTILINE.with(|mode| mode.get()));
        *old = new;
        out
    })
}

/// Field hit tests use the same GUI coordinates as the touch menus.
pub fn field_at(p: [f32; 2]) -> bool {
    horizon_client::widgets::last_targets().iter().any(|t| {
        let [x, y, w, h] = t.rect;
        t.kind == horizon_client::widgets::Kind::Field
            && (x..x + w).contains(&p[0])
            && (y..y + h).contains(&p[1])
    })
}

/// Only Microsoft's device-code verification pages can be opened by the login panel.
pub fn open_login_url(url: &str) -> Result<(), String> {
    let fixed = [
        "https://www.microsoft.com/link",
        "https://microsoft.com/link",
        "https://microsoft.com/devicelogin",
        "https://www.microsoft.com/devicelogin",
        "https://aka.ms/mce-reviewappid",
    ]
    .contains(&url);
    let verification = url::Url::parse(url).is_ok_and(|u| {
        let pairs = u.query_pairs().collect::<Vec<_>>();
        u.scheme() == "https"
            && matches!(u.host_str(), Some("microsoft.com" | "www.microsoft.com"))
            && matches!(u.path(), "/link" | "/devicelogin")
            && u.username().is_empty()
            && u.password().is_none()
            && u.port_or_known_default() == Some(443)
            && u.fragment().is_none()
            && pairs.len() == 1
            && pairs[0].0 == "otc"
            && pairs[0].1.len() <= 32
            && pairs[0]
                .1
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
    });
    let authorize = url::Url::parse(url).is_ok_and(|u| {
        u.scheme() == "https"
            && u.host_str() == Some("login.microsoftonline.com")
            && u.port_or_known_default() == Some(443)
            && u.username().is_empty()
            && u.password().is_none()
            && u.path() == "/consumers/oauth2/v2.0/authorize"
            && u.fragment().is_none()
    });
    if !fixed && !authorize && !verification {
        return Err("Unexpected Microsoft sign-in URL.".into());
    }
    crate::APP.with(|slot| {
        let app = slot.borrow();
        let app = app.as_ref().ok_or("Android activity unavailable")?;
        let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.map_err(|e| e.to_string())?;
        let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
        env.with_local_frame(16, |env| -> jni::errors::Result<()> {
            let url = JObject::from(env.new_string(url)?);
            let uri = env
                .call_static_method(
                    "android/net/Uri",
                    "parse",
                    "(Ljava/lang/String;)Landroid/net/Uri;",
                    &[JValue::Object(&url)],
                )?
                .l()?;
            let action = JObject::from(env.new_string("android.intent.action.VIEW")?);
            let intent = env.new_object(
                "android/content/Intent",
                "(Ljava/lang/String;Landroid/net/Uri;)V",
                &[JValue::Object(&action), JValue::Object(&uri)],
            )?;
            // The documented Custom Tabs intent protocol uses a nullable session
            // binder. Unsupported browsers fall back to ordinary ACTION_VIEW.
            let extras = env.new_object("android/os/Bundle", "()V", &[])?;
            let session =
                JObject::from(env.new_string("android.support.customtabs.extra.SESSION")?);
            let null = JObject::null();
            env.call_method(
                &extras,
                "putBinder",
                "(Ljava/lang/String;Landroid/os/IBinder;)V",
                &[JValue::Object(&session), JValue::Object(&null)],
            )?;
            env.call_method(
                &intent,
                "putExtras",
                "(Landroid/os/Bundle;)Landroid/content/Intent;",
                &[JValue::Object(&extras)],
            )?;
            let title =
                JObject::from(env.new_string("android.support.customtabs.extra.TITLE_VISIBILITY")?);
            env.call_method(
                &intent,
                "putExtra",
                "(Ljava/lang/String;I)Landroid/content/Intent;",
                &[JValue::Object(&title), JValue::Int(1)],
            )?;
            let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
            env.call_method(
                &activity,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            )?;
            Ok(())
        })
        .map_err(|e| {
            let _ = env.exception_clear();
            e.to_string()
        })
    })
}

/// Poll only at a low rate; Android sends no NativeActivity disconnect callback.
pub fn poll_gamepad_connections() {
    thread_local! { static LAST: std::cell::Cell<Option<std::time::Instant>> = const { std::cell::Cell::new(None) }; }
    if !LAST.with(|last| {
        if last
            .get()
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(500))
        {
            false
        } else {
            last.set(Some(std::time::Instant::now()));
            true
        }
    }) {
        return;
    }
    let ids = with_context(|env, _| {
        let ids = env
            .call_static_method("android/view/InputDevice", "getDeviceIds", "()[I", &[])?
            .l()?;
        let ids = jni::objects::JIntArray::from(ids);
        let mut values = vec![0; env.get_array_length(&ids)? as usize];
        env.get_int_array_region(&ids, 0, &mut values)?;
        let mut controllers = Vec::new();
        for id in &values {
            let device = env
                .call_static_method(
                    "android/view/InputDevice",
                    "getDevice",
                    "(I)Landroid/view/InputDevice;",
                    &[JValue::Int(*id)],
                )?
                .l()?;
            if !device.is_null() {
                let sources = env.call_method(&device, "getSources", "()I", &[])?.i()?;
                if sources & 0x401 == 0x401 || sources & 0x01000010 == 0x01000010 {
                    controllers.push(*id);
                }
                env.delete_local_ref(device)?;
            }
        }
        Ok((values, controllers))
    });
    if let Some((ids, controllers)) = ids {
        horizon_client::android_runtime::retain_gamepads(&ids);
        if horizon_client::android_runtime::active_gamepad_id().is_none() {
            cancel_gamepad_vibration();
        }
        winit::platform::android::set_native_gamepads(&controllers);
    }
}

/// Display rotation is queried only while gyro aiming is enabled.
pub fn display_rotation() -> i32 {
    with_context(|env, activity| {
        let manager = env
            .call_method(
                activity,
                "getWindowManager",
                "()Landroid/view/WindowManager;",
                &[],
            )?
            .l()?;
        let display = env
            .call_method(
                manager,
                "getDefaultDisplay",
                "()Landroid/view/Display;",
                &[],
            )?
            .l()?;
        env.call_method(display, "getRotation", "()I", &[])?.i()
    })
    .unwrap_or(0)
}
