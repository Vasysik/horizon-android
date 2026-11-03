//! Publish F2 PNGs through MediaStore; no broad storage access on Android 10+.
use horizon_client::Result;
use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JObject, JValue},
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};
static RESULTS: OnceLock<Mutex<Vec<std::result::Result<String, String>>>> = OnceLock::new();
static PERMISSION_PENDING: AtomicBool = AtomicBool::new(false);
struct Request {
    vm: JavaVM,
    activity: GlobalRef,
    resolver: GlobalRef,
    sdk: i32,
    legacy_dir: Option<PathBuf>,
    file: PathBuf,
    report: bool,
}
fn permission(env: &mut JNIEnv<'_>, activity: &JObject<'_>) -> jni::errors::Result<bool> {
    let name = JObject::from(env.new_string("android.permission.WRITE_EXTERNAL_STORAGE")?);
    Ok(env
        .call_method(
            activity,
            "checkSelfPermission",
            "(Ljava/lang/String;)I",
            &[JValue::Object(&name)],
        )?
        .i()?
        == 0)
}
pub fn save(file: &Path) -> std::result::Result<(), String> {
    save_file(file, false)
}
fn save_file(file: &Path, report: bool) -> std::result::Result<(), String> {
    let request = crate::platform::with_context(|env, activity| {
        let sdk = env
            .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
            .i()?;
        if sdk <= 28
            && !permission(env, activity)?
            && !PERMISSION_PENDING.swap(true, Ordering::Relaxed)
        {
            let permissions = env.new_object_array(1, "java/lang/String", JObject::null())?;
            env.set_object_array_element(
                &permissions,
                0,
                env.new_string("android.permission.WRITE_EXTERNAL_STORAGE")?,
            )?;
            env.call_method(
                activity,
                "requestPermissions",
                "([Ljava/lang/String;I)V",
                &[JValue::Object(permissions.as_ref()), JValue::Int(202)],
            )?;
        }
        let resolver = env
            .call_method(
                activity,
                "getContentResolver",
                "()Landroid/content/ContentResolver;",
                &[],
            )?
            .l()?;
        let legacy_dir = if sdk <= 28 {
            let pictures =
                JObject::from(env.new_string(if report { "Download" } else { "Pictures" })?);
            let dir = env
                .call_static_method(
                    "android/os/Environment",
                    "getExternalStoragePublicDirectory",
                    "(Ljava/lang/String;)Ljava/io/File;",
                    &[JValue::Object(&pictures)],
                )?
                .l()?;
            let path = env
                .call_method(&dir, "getAbsolutePath", "()Ljava/lang/String;", &[])?
                .l()?;
            let value: String = env.get_string((&path).into())?.into();
            Some(PathBuf::from(value).join("HorizonAndroid"))
        } else {
            None
        };
        Ok(Request {
            vm: env.get_java_vm()?,
            activity: env.new_global_ref(activity)?,
            resolver: env.new_global_ref(resolver)?,
            sdk,
            legacy_dir,
            file: file.to_owned(),
            report,
        })
    });
    let Some(request) = request else {
        PERMISSION_PENDING.store(false, Ordering::Relaxed);
        return Err("Android could not open file export".into());
    };
    std::thread::Builder::new()
        .name("Screenshot gallery".into())
        .spawn(move || {
            let result = write(request).map_err(|e| e.to_string());
            PERMISSION_PENDING.store(false, Ordering::Relaxed);
            RESULTS
                .get_or_init(Mutex::default)
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(result);
        })
        .map_err(|e| {
            PERMISSION_PENDING.store(false, Ordering::Relaxed);
            e.to_string()
        })?;
    Ok(())
}
pub fn poll() -> Vec<std::result::Result<String, String>> {
    std::mem::take(
        &mut *RESULTS
            .get_or_init(Mutex::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner()),
    )
}
fn put(
    env: &mut JNIEnv<'_>,
    values: &JObject<'_>,
    key: &str,
    text: &str,
) -> jni::errors::Result<()> {
    let key = JObject::from(env.new_string(key)?);
    let text = JObject::from(env.new_string(text)?);
    env.call_method(
        values,
        "put",
        "(Ljava/lang/String;Ljava/lang/String;)V",
        &[JValue::Object(&key), JValue::Object(&text)],
    )?;
    Ok(())
}
fn pending(env: &mut JNIEnv<'_>, values: &JObject<'_>, value: i32) -> jni::errors::Result<()> {
    let key = JObject::from(env.new_string("is_pending")?);
    let number = env
        .call_static_method(
            "java/lang/Integer",
            "valueOf",
            "(I)Ljava/lang/Integer;",
            &[JValue::Int(value)],
        )?
        .l()?;
    env.call_method(
        values,
        "put",
        "(Ljava/lang/String;Ljava/lang/Integer;)V",
        &[JValue::Object(&key), JValue::Object(&number)],
    )?;
    Ok(())
}
fn write(request: Request) -> Result<String> {
    use std::os::fd::FromRawFd;
    let mut env = request.vm.attach_current_thread()?;
    if request.sdk <= 28 {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !permission(&mut env, request.activity.as_obj())? {
            if std::time::Instant::now() >= deadline {
                PERMISSION_PENDING.store(false, Ordering::Relaxed);
                return Err(
                    "Allow storage access to export this file; the original remains in Horizon"
                        .into(),
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        PERMISSION_PENDING.store(false, Ordering::Relaxed);
    }
    let mut input = std::fs::File::open(&request.file)?;
    let name = request
        .file
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid screenshot name")?;
    let mut row = JObject::null();
    let mut reserved_legacy: Option<PathBuf> = None;
    let result = (|| -> Result<String> {
        let values = env.new_object("android/content/ContentValues", "()V", &[])?;
        put(&mut env, &values, "_display_name", name)?;
        put(
            &mut env,
            &values,
            "mime_type",
            if request.report {
                "application/x-ndjson"
            } else {
                "image/png"
            },
        )?;
        if request.sdk >= 29 {
            put(
                &mut env,
                &values,
                "relative_path",
                if request.report {
                    "Download/HorizonAndroid"
                } else {
                    "Pictures/HorizonAndroid"
                },
            )?;
            pending(&mut env, &values, 1)?;
        } else if let Some(dir) = &request.legacy_dir {
            std::fs::create_dir_all(dir)?;
            let mut output = dir.join(name);
            for n in 1.. {
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&output)
                {
                    Ok(_) => {
                        reserved_legacy = Some(output.clone());
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(e) => return Err(e.into()),
                }
                output = dir.join(format!(
                    "{}_{n}.{}",
                    request
                        .file
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy(),
                    request
                        .file
                        .extension()
                        .unwrap_or_default()
                        .to_string_lossy()
                ));
            }
            put(&mut env, &values, "_data", &output.to_string_lossy())?;
        }
        let images = if request.report && request.sdk >= 29 {
            env.get_static_field(
                "android/provider/MediaStore$Downloads",
                "EXTERNAL_CONTENT_URI",
                "Landroid/net/Uri;",
            )?
            .l()?
        } else if request.report {
            let volume = JObject::from(env.new_string("external")?);
            env.call_static_method(
                "android/provider/MediaStore$Files",
                "getContentUri",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&volume)],
            )?
            .l()?
        } else {
            env.get_static_field(
                "android/provider/MediaStore$Images$Media",
                "EXTERNAL_CONTENT_URI",
                "Landroid/net/Uri;",
            )?
            .l()?
        };
        row = env
            .call_method(
                request.resolver.as_obj(),
                "insert",
                "(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;",
                &[JValue::Object(&images), JValue::Object(&values)],
            )?
            .l()?;
        if row.is_null() {
            return Err("MediaStore did not create an image".into());
        }
        let mode = JObject::from(env.new_string("w")?);
        let fd = env
            .call_method(
                request.resolver.as_obj(),
                "openFileDescriptor",
                "(Landroid/net/Uri;Ljava/lang/String;)Landroid/os/ParcelFileDescriptor;",
                &[JValue::Object(&row), JValue::Object(&mode)],
            )?
            .l()?;
        let raw = env.call_method(&fd, "detachFd", "()I", &[])?.i()?;
        if raw < 0 {
            return Err("MediaStore did not provide a writable file".into());
        }
        // detachFd transfers ownership to this File, including on copy failure.
        let mut output = unsafe { std::fs::File::from_raw_fd(raw) };
        std::io::copy(&mut input, &mut output)?;
        output.sync_all()?;
        drop(output);
        if request.sdk >= 29 {
            let published = env.new_object("android/content/ContentValues", "()V", &[])?;
            pending(&mut env, &published, 0)?;
            env.call_method(request.resolver.as_obj(),"update","(Landroid/net/Uri;Landroid/content/ContentValues;Ljava/lang/String;[Ljava/lang/String;)I",&[JValue::Object(&row),JValue::Object(&published),JValue::Object(&JObject::null()),JValue::Object(&JObject::null())])?;
        }
        Ok(format!(
            "{} saved to {}/HorizonAndroid/{name}",
            if request.report {
                "Performance report"
            } else {
                "Screenshot"
            },
            if request.report {
                "Download"
            } else {
                "Pictures"
            }
        ))
    })();
    if result.is_err() && !row.is_null() {
        let _ = env.exception_clear();
        let _ = env.call_method(
            request.resolver.as_obj(),
            "delete",
            "(Landroid/net/Uri;Ljava/lang/String;[Ljava/lang/String;)I",
            &[
                JValue::Object(&row),
                JValue::Object(&JObject::null()),
                JValue::Object(&JObject::null()),
            ],
        );
        let _ = env.exception_clear();
    }
    if result.is_err()
        && let Some(path) = reserved_legacy
    {
        let _ = std::fs::remove_file(path);
    }
    result
}
