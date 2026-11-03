//! Isolated diagnostic recovery probe. No game, GPU, accounts or player data.
#![allow(dead_code)]
#[path = "../../../crates/horizon-client/src/android/diagnostics.rs"]
mod diagnostics;
#[path = "../../../crates/horizon-client/src/android/exit_reports.rs"]
mod exit_reports;
mod backend { pub const VERSION: &str = "diagnostics-probe"; }
use android_activity::{AndroidApp, MainEvent, PollEvent};
use std::{cell::RefCell, time::Duration};
thread_local! {static APP:RefCell<Option<AndroidApp>>=const {RefCell::new(None)};}
mod platform {
    pub(crate) fn with_context<R>(f:impl FnOnce(&mut jni::JNIEnv<'_>,&jni::objects::JObject<'_>)->jni::errors::Result<R>)->Option<R>{
        super::APP.with(|slot|{
            let app=slot.borrow();let app=app.as_ref()?;
            let vm=unsafe {jni::JavaVM::from_raw(app.vm_as_ptr().cast())}.ok()?;
            let mut env=vm.attach_current_thread().ok()?;
            env.with_local_frame(32,|env|{
                let activity=unsafe {jni::objects::JObject::from_raw(app.activity_as_ptr().cast())};
                f(env,&activity)
            }).ok()
        })
    }
}
#[unsafe(no_mangle)]
fn android_main(app:AndroidApp){
    let home=app.internal_data_path().unwrap().join("diagnostic-probe");
    APP.with(|slot|*slot.borrow_mut()=Some(app.clone()));
    diagnostics::initialize(&home,serde_json::json!({"purpose":"isolated recovery test"})).unwrap();
    exit_reports::install(&home);
    diagnostics::heartbeat("probe_ready",||serde_json::json!({}));
    diagnostics::log_line(log::Level::Info,"probe","Rust diagnostic writer ready");
    diagnostics::flush();
    std::fs::write(home.join("ready"),b"ready").unwrap();
    loop {
        let mut destroyed=false;
        app.poll_events(Some(Duration::from_millis(100)),|event|{
            if matches!(event,PollEvent::Main(MainEvent::Destroy)){destroyed=true;}
        });
        if destroyed {diagnostics::finish();return;}
        let command=std::fs::read_to_string(home.join("command")).unwrap_or_default();
        if command.is_empty(){continue;}
        let _=std::fs::remove_file(home.join("command"));
        match command.trim(){
            "abort"=>{diagnostics::heartbeat("intentional_native_abort",||serde_json::json!({}));diagnostics::flush();std::process::abort();}
            "panic"=>panic!("intentional diagnostic test panic"),
            "export"=>{let result=diagnostics::export(&home).map(|p|p.to_string_lossy().into_owned());let _=std::fs::write(home.join("export-result"),format!("{result:?}"));}
            "finish"=>{diagnostics::finish();std::process::exit(0);}
            _=>{}
        }
    }
}
