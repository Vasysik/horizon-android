//! Sensor delivery owns its looper: it never wakes or logs through NativeActivity.
use std::{
    collections::VecDeque,
    ffi::{c_int, c_void},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

#[link(name = "android")]
unsafe extern "C" {
    fn ALooper_prepare(flags: c_int) -> *mut c_void;
    fn ALooper_pollOnce(
        ms: c_int,
        fd: *mut c_int,
        events: *mut c_int,
        data: *mut *mut c_void,
    ) -> c_int;
    fn ASensorManager_getInstance() -> *mut c_void;
    fn ASensorManager_getDefaultSensor(m: *mut c_void, t: c_int) -> *const c_void;
    fn ASensorManager_createEventQueue(
        m: *mut c_void,
        l: *mut c_void,
        id: c_int,
        cb: *mut c_void,
        data: *mut c_void,
    ) -> *mut c_void;
    fn ASensorManager_destroyEventQueue(m: *mut c_void, q: *mut c_void) -> c_int;
    fn ASensorEventQueue_enableSensor(q: *mut c_void, s: *const c_void) -> c_int;
    fn ASensorEventQueue_disableSensor(q: *mut c_void, s: *const c_void) -> c_int;
    fn ASensorEventQueue_setEventRate(q: *mut c_void, s: *const c_void, us: c_int) -> c_int;
    fn ASensorEventQueue_getEvents(q: *mut c_void, e: *mut SensorEvent, n: usize) -> isize;
}
#[repr(C)]
struct SensorEvent {
    version: i32,
    sensor: i32,
    kind: i32,
    reserved: i32,
    timestamp: i64,
    data: [f32; 16],
    flags: u32,
    tail: [i32; 3],
}
struct Sample {
    timestamp: i64,
    rate: [f32; 3],
}
#[derive(Default)]
struct Shared {
    enabled: AtomicBool,
    stop: AtomicBool,
    available: AtomicBool,
    active: AtomicBool,
    pending: Mutex<VecDeque<Sample>>,
}
pub(super) struct Gyroscope {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
    enabled: bool,
    last: i64,
    pub samples: u64,
}
impl Gyroscope {
    pub fn spawn() -> Option<Self> {
        let shared = Arc::new(Shared::default());
        let state = shared.clone();
        let worker = thread::Builder::new()
            .name("Android gyroscope".into())
            .spawn(move || run(&state))
            .ok()?;
        Some(Self {
            shared,
            worker: Some(worker),
            enabled: false,
            last: 0,
            samples: 0,
        })
    }
    pub fn available(&self) -> bool {
        self.shared.available.load(Ordering::Acquire)
    }
    pub fn active(&self) -> bool {
        self.shared.active.load(Ordering::Acquire)
    }
    pub fn read(&mut self, enabled: bool, sensitivity: f32, rotation: i32) -> [f32; 2] {
        if self.enabled != enabled {
            self.enabled = enabled;
            self.last = 0;
            self.shared
                .pending
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
            self.shared.enabled.store(enabled, Ordering::Release);
            if let Some(w) = &self.worker {
                w.thread().unpark();
            }
        }
        let mut delta = [0.; 2];
        let mut pending = self
            .shared
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        for sample in pending.drain(..) {
            if !enabled {
                continue;
            }
            let dt = (sample.timestamp - self.last) as f32 * 1e-9;
            let valid = self.last != 0 && dt > 0. && dt < 0.1;
            self.last = sample.timestamp;
            if valid {
                let d = super::math::gyro_delta(sample.rate, dt, rotation, sensitivity);
                delta[0] += d[0];
                delta[1] += d[1];
                self.samples += 1;
            }
        }
        delta.map(|v| v.clamp(-20., 20.))
    }
}
impl Drop for Gyroscope {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        if let Some(w) = self.worker.take() {
            w.thread().unpark();
            let _ = w.join();
        }
    }
}
fn run(state: &Shared) {
    // All opaque Android pointers stay on this thread. At most 128 recent
    // samples are retained; the render thread does not allocate per sample.
    unsafe {
        let manager = ASensorManager_getInstance();
        if manager.is_null() {
            return;
        }
        let sensor = ASensorManager_getDefaultSensor(manager, 4);
        if sensor.is_null() {
            return;
        }
        let looper = ALooper_prepare(1);
        if looper.is_null() {
            return;
        }
        let queue = ASensorManager_createEventQueue(
            manager,
            looper,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        if queue.is_null() {
            return;
        }
        state.available.store(true, Ordering::Release);
        let mut active = false;
        while !state.stop.load(Ordering::Acquire) {
            let enabled = state.enabled.load(Ordering::Acquire);
            if active != enabled {
                if enabled {
                    active = ASensorEventQueue_enableSensor(queue, sensor) >= 0;
                    if active {
                        ASensorEventQueue_setEventRate(queue, sensor, 10_000);
                    }
                } else {
                    ASensorEventQueue_disableSensor(queue, sensor);
                    active = false;
                }
                state.active.store(active, Ordering::Release);
            }
            if !active {
                thread::park();
                continue;
            }
            // Bounded wait also bounds focus-loss shutdown and Drop/join to 20ms.
            ALooper_pollOnce(
                20,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            for _ in 0..256 {
                let mut event: SensorEvent = std::mem::zeroed();
                if ASensorEventQueue_getEvents(queue, &mut event, 1) <= 0 {
                    break;
                }
                if event.kind != 4 {
                    continue;
                }
                let mut pending = state.pending.lock().unwrap_or_else(|e| e.into_inner());
                if pending.len() == 128 {
                    pending.pop_front();
                }
                pending.push_back(Sample {
                    timestamp: event.timestamp,
                    rate: [event.data[0], event.data[1], event.data[2]],
                });
            }
        }
        if active {
            ASensorEventQueue_disableSensor(queue, sensor);
        }
        state.active.store(false, Ordering::Release);
        ASensorManager_destroyEventQueue(manager, queue);
    }
}
