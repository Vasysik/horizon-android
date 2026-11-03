//! DocumentsProvider JNI entry points also work when no Activity/game is running.
use super::document_store;
use jni::{
    JNIEnv,
    objects::{JClass, JString},
    sys::{jint, jstring},
};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::{os::fd::IntoRawFd, path::PathBuf};
static OPERATIONS: Mutex<()> = Mutex::new(());
static CHANGED: AtomicBool = AtomicBool::new(false);
pub fn take_changes() -> bool {
    CHANGED.swap(false, Ordering::Relaxed)
}

fn text(env: &mut JNIEnv<'_>, value: &JString<'_>) -> std::result::Result<String, String> {
    env.get_string(value)
        .map(Into::into)
        .map_err(|e| e.to_string())
}
fn failure(env: &mut JNIEnv<'_>, message: &str) {
    if env.exception_check().unwrap_or(true) {
        let _ = env.exception_clear();
    }
    let _ = env.throw_new("java/io/FileNotFoundException", message);
}

#[unsafe(no_mangle)]
extern "system" fn Java_dev_horizon_android_HorizonDocumentsProvider_nativeCall(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    home: JString<'_>,
    operation: JString<'_>,
    id: JString<'_>,
    arg: JString<'_>,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = OPERATIONS.lock().unwrap_or_else(|e| e.into_inner());
        let home = PathBuf::from(text(&mut env, &home)?);
        let op = text(&mut env, &operation)?;
        let id = text(&mut env, &id)?;
        let arg = text(&mut env, &arg)?;
        let value = document_store::call(&home, &op, &id, &arg).map_err(|e| e.to_string())?;
        if matches!(op.as_str(), "create" | "mkdir" | "rename" | "delete") {
            CHANGED.store(true, Ordering::Relaxed);
        }
        env.new_string(value.to_string())
            .map(|s| s.into_raw())
            .map_err(|e| e.to_string())
    }))
    .unwrap_or_else(|_| Err("Horizon document operation failed".into()));
    match result {
        Ok(value) => value,
        Err(error) => {
            failure(&mut env, &error);
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
extern "system" fn Java_dev_horizon_android_HorizonDocumentsProvider_nativeOpen(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    home: JString<'_>,
    id: JString<'_>,
    mode: JString<'_>,
) -> jint {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = OPERATIONS.lock().unwrap_or_else(|e| e.into_inner());
        let home = PathBuf::from(text(&mut env, &home)?);
        let id = text(&mut env, &id)?;
        let mode = text(&mut env, &mode)?;
        let file = document_store::open(&home, &id, &mode).map_err(|e| e.to_string())?;
        if mode != "r" {
            CHANGED.store(true, Ordering::Relaxed);
        }
        Ok::<_, String>(file.into_raw_fd())
    }))
    .unwrap_or_else(|_| Err("Horizon document open failed".into()));
    match result {
        Ok(fd) => fd,
        Err(error) => {
            failure(&mut env, &error);
            -1
        }
    }
}
