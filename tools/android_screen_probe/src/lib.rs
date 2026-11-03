//! Isolated window/input probe. No game, resources, accounts or player data.
#![allow(dead_code)]
#[path = "../../../crates/horizon-client/src/android/native_ime.rs"]
mod ime;
// SAF dispatch is outside this probe; the production screen/IME code is shared.
mod document_bridge {
    pub fn register(
        _: &mut jni::JNIEnv<'_>,
        _: &jni::objects::JObject<'_>,
        _: super::documents::Callback,
    ) -> jni::errors::Result<()> {
        Ok(())
    }
}
mod documents {
    pub type Callback = extern "system" fn(
        jni::JNIEnv<'_>,
        jni::objects::JObject<'_>,
        i32,
        i32,
        jni::objects::JString<'_>,
    );
    pub extern "system" fn document_result(
        _: jni::JNIEnv<'_>,
        _: jni::objects::JObject<'_>,
        _: i32,
        _: i32,
        _: jni::objects::JString<'_>,
    ) {
    }
    pub fn picker_failed(_: &str) {}
}
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    platform::android::{
        EventLoopBuilderExtAndroid,
        activity::{AndroidApp, OnCreateState},
    },
    window::{Window, WindowId},
};
struct Probe {
    app: AndroidApp,
    window: Option<Window>,
    start: Instant,
    phase: u32,
    drawn: Instant,
}
impl ApplicationHandler for Probe {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        self.window = Some(events.create_window(Window::default_attributes()).unwrap());
    }
    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Touch(t) => log::info!("TOUCH {:?} logical={:?}", t.phase, t.location),
            WindowEvent::Resized(s) => log::info!("RESIZE {s:?}"),
            WindowEvent::CloseRequested => events.exit(),
            _ => {}
        }
    }
    fn about_to_wait(&mut self, events: &ActiveEventLoop) {
        events.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(100),
        ));
        let phase = ((self.start.elapsed().as_secs() / 8) % 8) as u32;
        if self.phase != phase {
            self.phase = phase;
            ime::configure_screen(phase != 0, phase >= 4);
            let aspect = match phase % 4 {
                0 => [0, 0],
                1 => [16, 9],
                2 => [4, 3],
                _ => [1, 1],
            };
            winit::platform::android::set_native_content_aspect(aspect);
            winit::platform::android::set_native_input_scale(if phase >= 6 {
                [0.5, 0.5]
            } else {
                [1.0, 1.0]
            });
            log::info!(
                "PHASE {phase} aspect={aspect:?} immersive={} avoid={} scale={}",
                phase != 0,
                phase >= 4,
                if phase >= 6 { 0.5 } else { 1.0 }
            );
        }
        ime::refresh_bounds();
        if self.drawn.elapsed() < Duration::from_millis(250) {
            return;
        }
        self.drawn = Instant::now();
        if let Some(window) = self.app.native_window() {
            if let Ok(mut buffer) = window.lock(None) {
                let (w, h, stride) = (buffer.width(), buffer.height(), buffer.stride());
                let [x, y, rw, rh] =
                    winit::platform::android::native_content_rect([w as u32, h as u32]);
                if let Some(bytes) = buffer.bytes() {
                    for py in 0..h {
                        for px in 0..w {
                            let inside = px >= x as usize
                                && py >= y as usize
                                && px < (x + rw) as usize
                                && py < (y + rh) as usize;
                            let border = inside
                                && (px < (x + 4) as usize
                                    || py < (y + 4) as usize
                                    || px >= (x + rw - 4.min(rw)) as usize
                                    || py >= (y + rh - 4.min(rh)) as usize);
                            let rgba = if border {
                                [255, 255, 255, 255]
                            } else if inside {
                                [32, 96, 160, 255]
                            } else {
                                [0, 0, 0, 255]
                            };
                            for (c, v) in rgba.into_iter().enumerate() {
                                bytes[(py * stride + px) * 4 + c].write(v);
                            }
                        }
                    }
                }
            }
            if let Some(w) = &self.window {
                log::info!(
                    "RECT {:?} logical={:?}",
                    winit::platform::android::native_content_rect([
                        window.width() as u32,
                        window.height() as u32
                    ]),
                    w.inner_size()
                );
            }
        }
    }
}
#[unsafe(no_mangle)]
fn android_on_create(s: &OnCreateState<'_>) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("HorizonScreenProbe")
            .with_max_level(log::LevelFilter::Info),
    );
    if let Err(e) = unsafe { ime::initialize(s.vm_as_ptr().cast(), s.activity_as_ptr().cast()) } {
        log::error!("INIT {e}");
    }
}
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    let events = EventLoop::builder()
        .with_android_app(app.clone())
        .build()
        .unwrap();
    events
        .run_app(&mut Probe {
            app,
            window: None,
            start: Instant::now(),
            phase: u32::MAX,
            drawn: Instant::now() - Duration::from_secs(1),
        })
        .unwrap();
}
