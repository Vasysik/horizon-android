//! Isolated developer probe: no Minecraft assets, accounts or player data.
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    platform::android::{EventLoopBuilderExtAndroid, activity::AndroidApp},
    window::{Window, WindowId},
};
struct Probe {
    app: AndroidApp,
    window: Option<Window>,
}
impl Probe {
    fn capture(&self, capture: bool) {
        let result = (|| -> jni::errors::Result<bool> {
            let vm = unsafe { jni::JavaVM::from_raw(self.app.vm_as_ptr().cast()) }?;
            let mut env = vm.attach_current_thread()?;
            let activity =
                unsafe { jni::objects::JObject::from_raw(self.app.activity_as_ptr().cast()) };
            let window = env
                .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?
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
            env.call_method(&view, "hasPointerCapture", "()Z", &[])?.z()
        })();
        log::info!("capture request {capture}: {result:?}");
    }
}
impl ApplicationHandler for Probe {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        self.window = Some(events.create_window(Window::default_attributes()).unwrap());
        // Submit one frame: Android otherwise has no visible surface/touch region.
        if let Some(window) = self.app.native_window() {
            if let Ok(mut buffer) = window.lock(None) {
                if let Some(bytes) = buffer.bytes() {
                    for byte in bytes {
                        byte.write(255);
                    }
                }
            }
        }
    }
    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        log::info!("window {event:?}");
        match event {
            WindowEvent::Focused(focused) => self.capture(focused),
            WindowEvent::CloseRequested => events.exit(),
            WindowEvent::KeyboardInput { event, .. } if event.state.is_pressed() => {
                if event.logical_key
                    == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
                {
                    self.capture(false);
                } else {
                    self.capture(true);
                }
            }
            _ => {}
        }
    }
    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        log::info!("device {event:?}");
    }
}
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("HorizonInputProbe")
            .with_max_level(log::LevelFilter::Info),
    );
    let events = EventLoop::builder()
        .with_android_app(app.clone())
        .build()
        .unwrap();
    events.run_app(&mut Probe { app, window: None }).unwrap();
    std::process::exit(0);
}
