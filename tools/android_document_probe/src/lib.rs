//! Isolated SAF regression probe; no game, accounts or user worlds.
#![allow(dead_code)]
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[path = "../../../crates/horizon-client/src/content_files.rs"]
mod content_files;
#[path = "../../../crates/horizon-client/src/android/document_bridge.rs"]
mod document_bridge;
#[path = "../../../crates/horizon-client/src/android/native_ime.rs"]
mod ime;
use android_activity::{AndroidApp, MainEvent, OnCreateState, PollEvent};
use jni::{
    JNIEnv,
    objects::{JObject, JString, JValue},
};
mod documents {
    use super::*;
    pub fn picker_failed(e: &str) {
        log::error!("PICKER ERROR {e}")
    }
    pub extern "system" fn document_result(
        mut env: JNIEnv<'_>,
        activity: JObject<'_>,
        request: i32,
        result: i32,
        uri: JString<'_>,
    ) {
        if request != 4101 {
            return;
        }
        if result != -1 || uri.is_null() {
            log::info!("PASS cancel callback");
            return;
        }
        let setup = (|| -> jni::errors::Result<_> {
            let text: String = env.get_string(&uri)?.into();
            Ok((env.get_java_vm()?, env.new_global_ref(activity)?, text))
        })();
        if let Ok((vm, activity, uri)) = setup {
            std::thread::spawn(move || {
                let result = (|| -> Result<()> {
                    let mut env = vm.attach_current_thread()?;
                    let opened =
                        document_bridge::open(&mut env, activity.as_obj(), &uri, "import.zip");
                    if opened.is_err() {
                        let _ = env.exception_clear();
                    }
                    let (mut file, name) = opened?;
                    let cache = env
                        .call_method(activity.as_obj(), "getCacheDir", "()Ljava/io/File;", &[])?
                        .l()?;
                    let path = env
                        .call_method(&cache, "getAbsolutePath", "()Ljava/lang/String;", &[])?
                        .l()?;
                    let path: String = env.get_string((&path).into())?.into();
                    let stage = std::path::PathBuf::from(path).join("probe.part");
                    let mut output = std::fs::File::create(&stage)?;
                    let copied = content_files::copy_bounded(&mut file, &mut output, |_| true)?;
                    drop(output);
                    let kind = if name.ends_with(".hmod") {
                        content_files::Kind::Mods
                    } else if name.ends_with(".hshader") {
                        content_files::Kind::Shaders
                    } else {
                        content_files::Kind::Packs
                    };
                    let result = content_files::validate(&stage, kind, &name, || false);
                    let _ = std::fs::remove_file(stage);
                    result?;
                    log::info!("PASS provider read + copy + validation: {copied} bytes ({kind:?})");
                    Ok(())
                })();
                if let Err(e) = result {
                    log::error!("PROVIDER RESULT {e}")
                }
            });
        } else {
            let _ = env.exception_clear();
            log::error!("CALLBACK ERROR")
        }
    }
}
#[unsafe(no_mangle)]
fn android_on_create(s: &OnCreateState<'_>) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("HorizonDocumentProbe")
            .with_max_level(log::LevelFilter::Info),
    );
    if let Err(e) = unsafe { ime::initialize(s.vm_as_ptr().cast(), s.activity_as_ptr().cast()) } {
        log::error!("INIT ERROR {e}");
        return;
    }
    let result = (|| -> jni::errors::Result<()> {
        let vm = unsafe { jni::JavaVM::from_raw(s.vm_as_ptr().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { JObject::from_raw(s.activity_as_ptr().cast()) };
        env.call_method(
            &activity,
            "nativeDocumentResult",
            "(IILjava/lang/String;)V",
            &[
                JValue::Int(4101),
                JValue::Int(0),
                JValue::Object(&JObject::null()),
            ],
        )?;
        let text = JObject::from(env.new_string("content://missing.probe/document/test")?);
        env.call_method(
            &activity,
            "nativeDocumentResult",
            "(IILjava/lang/String;)V",
            &[JValue::Int(4101), JValue::Int(-1), JValue::Object(&text)],
        )?;
        log::info!("PASS Java dispatch of cancel + success callbacks");
        Ok(())
    })();
    if let Err(e) = result {
        log::error!("DISPATCH ERROR {e}")
    }
}
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    let mut done = false;
    let mut picked = false;
    while !done {
        app.poll_events(
            Some(std::time::Duration::from_millis(100)),
            |event| match event {
                PollEvent::Main(MainEvent::InitWindow { .. }) => {
                    if let Some(w) = app.native_window() {
                        if let Ok(mut b) = w.lock(None) {
                            if let Some(bytes) = b.bytes() {
                                for v in bytes {
                                    v.write(150);
                                }
                            }
                        }
                    }
                    if !picked {
                        picked = true;
                        ime::pick_document();
                    }
                }
                PollEvent::Main(MainEvent::InputAvailable) => {
                    if let Ok(mut input) = app.input_events_iter() {
                        while input.next(|_| android_activity::InputStatus::Unhandled) {}
                    }
                }
                PollEvent::Main(MainEvent::Destroy) => done = true,
                _ => {}
            },
        );
    }
    std::process::exit(0)
}
