//! SAF selection, bounded background imports and explicit collision choices.
use horizon_client::{
    Result,
    content_files::{self, Kind},
    hud::{HudAtlas, HudBatch, rgba},
    ui_theme::{Face, Ui, color},
};
use jni::{
    JNIEnv,
    objects::{JObject, JString},
};
use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

struct Request {
    kind: Kind,
    home: PathBuf,
    cache: PathBuf,
}
struct Conflict {
    kind: Kind,
    staged: PathBuf,
    target: PathBuf,
}
#[derive(Clone)]
pub struct Notice {
    pub text: String,
    pub changed: bool,
    pub kind: Option<Kind>,
}
#[derive(Default)]
struct State {
    request: Option<Request>,
    working: bool,
    conflict: Option<Conflict>,
    notice: Option<Notice>,
    hits: Vec<([f32; 4], Choice)>,
    toast: Option<(String, std::time::Instant)>,
    cursor: (f32, f32),
}
#[derive(Clone, Copy)]
pub enum Choice {
    KeepBoth,
    Replace,
    Cancel,
}
static STATE: Mutex<State> = Mutex::new(State {
    request: None,
    working: false,
    conflict: None,
    notice: None,
    hits: Vec::new(),
    toast: None,
    cursor: (0.0, 0.0),
});
static CANCEL: AtomicBool = AtomicBool::new(false);
pub fn busy() -> bool {
    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.request.is_some() || s.working || s.conflict.is_some()
}
pub fn conflict() -> bool {
    STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .conflict
        .is_some()
}
fn notice(text: impl Into<String>, changed: bool) {
    let text = text.into();
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.toast = Some((text.clone(), std::time::Instant::now()));
    s.notice = Some(Notice {
        text,
        changed,
        kind: None,
    });
}
fn imported(text: impl Into<String>, kind: Kind) {
    let text = text.into();
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.toast = Some((text.clone(), std::time::Instant::now()));
    s.notice = Some(Notice {
        text,
        changed: true,
        kind: Some(kind),
    });
}
pub fn poll() -> Option<Notice> {
    if crate::document_provider::take_changes() {
        return Some(Notice {
            text: "Content files changed. Reload packs to apply.".into(),
            changed: true,
            kind: None,
        });
    }
    STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .notice
        .take()
}
pub fn begin(kind: Kind, home: PathBuf) -> std::result::Result<(), String> {
    if busy() {
        return Err("Finish or cancel the current import first".into());
    }
    CANCEL.store(false, Ordering::Relaxed);
    crate::platform::hide();
    let cache = crate::platform::cache_dir()
        .ok_or("Could not access the Android import cache")?
        .join("content-imports");
    STATE.lock().unwrap_or_else(|e| e.into_inner()).request = Some(Request { kind, home, cache });
    if !crate::native_ime::pick_document() {
        picker_failed("Android document picker is unavailable");
        return Err("Android document picker is unavailable".into());
    }
    notice(format!("Choose a {} file", kind.label()), false);
    Ok(())
}
pub fn picker_failed(error: &str) {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).request = None;
    notice(error, false);
}
/// Called by the approved Activity result bridge, never from a game tick.
/// Keep a conventional JNI export as well as explicit registration: Android
/// may restore a picker result before the IME bridge has been initialized.
#[unsafe(export_name = "Java_dev_horizon_android_HorizonActivity_nativeDocumentResult")]
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
    let pending = STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .request
        .take();
    let Some(pending) = pending else {
        notice("Import interrupted. Choose the file again.", false);
        return;
    };
    if result != -1 || uri.is_null() {
        notice("Import cancelled", false);
        return;
    }
    let setup = (|| -> jni::errors::Result<_> {
        let text: String = env.get_string(&uri)?.into();
        let activity = env.new_global_ref(activity)?;
        Ok((env.get_java_vm()?, activity, text))
    })();
    let Ok((vm, activity, uri)) = setup else {
        let _ = env.exception_clear();
        notice("Could not read the selected document", false);
        return;
    };
    if !uri.starts_with("content://") || uri.len() > 8192 {
        notice("Unsupported document URI", false);
        return;
    }
    STATE.lock().unwrap_or_else(|e| e.into_inner()).working = true;
    let spawned = std::thread::Builder::new()
        .name("Content import".into())
        .spawn(move || {
            let result = (|| -> Result<()> {
                let mut env = vm.attach_current_thread()?;
                let fallback = if pending.kind == Kind::Mods {
                    "import.hmod"
                } else {
                    "import.zip"
                };
                let opened =
                    crate::document_bridge::open(&mut env, activity.as_obj(), &uri, fallback);
                if opened.is_err() {
                    let _ = env.exception_clear();
                }
                let (input, name) = opened.map_err(|e| e.to_string())?;
                stage(pending, input, name)
            })();
            STATE.lock().unwrap_or_else(|e| e.into_inner()).working = false;
            if let Err(error) = result {
                notice(format!("Import failed: {error}"), false);
            }
        });
    if let Err(error) = spawned {
        STATE.lock().unwrap_or_else(|e| e.into_inner()).working = false;
        notice(format!("Import failed: {error}"), false);
    }
}
fn stage(request: Request, mut input: std::fs::File, name: String) -> Result<()> {
    if !content_files::plain_name(&name) || !request.kind.accepts(&name) {
        return Err("Selected file has the wrong extension for this folder".into());
    }
    let dir = request.home.join(request.kind.folder());
    std::fs::create_dir_all(&dir)?;
    if std::fs::symlink_metadata(&dir)?.file_type().is_symlink() {
        return Err("Refusing a redirected content directory".into());
    }
    std::fs::create_dir_all(&request.cache)?;
    if std::fs::symlink_metadata(&request.cache)?
        .file_type()
        .is_symlink()
    {
        return Err("Refusing a redirected import cache".into());
    }
    for entry in std::fs::read_dir(&request.cache)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(".import-") && name.ends_with(".part") && entry.file_type()?.is_file() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let staged = request.cache.join(format!(".import-{stamp}.part"));
    let result = (|| -> Result<()> {
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)?;
        content_files::copy_bounded(&mut input, &mut output, |bytes| {
            notice(
                format!("Importing {}: {} KiB", request.kind.label(), bytes / 1024),
                false,
            );
            !CANCEL.load(Ordering::Relaxed)
        })?;
        output.sync_all()?;
        drop(output);
        notice("Checking archive...", false);
        let native_shader = content_files::validate(&staged, request.kind, &name, || {
            CANCEL.load(Ordering::Relaxed)
        })?;
        let name = if request.kind == Kind::Mods {
            format!("{}.hmod", horizon_mods::Package::read(&staged)?.manifest.id)
        } else if native_shader || name.to_ascii_lowercase().ends_with(".hshader") {
            let pack = horizon_shader_api::Pack::read(&staged)?;
            horizon_shader_api::check(&pack).map_err(|e| format!("Invalid shader pack: {e:?}"))?;
            let mut path = PathBuf::from(name);
            path.set_extension("hshader");
            path.to_string_lossy().into_owned()
        } else {
            name
        };
        if !content_files::plain_name(&name) {
            return Err("Package has an invalid ID or filename".into());
        }
        if CANCEL.load(Ordering::Relaxed) {
            return Err("Import cancelled".into());
        }
        let target = dir.join(name);
        if target.exists() {
            STATE.lock().unwrap_or_else(|e| e.into_inner()).conflict = Some(Conflict {
                kind: request.kind,
                staged: staged.clone(),
                target,
            });
            notice(
                "File already exists: choose Keep both, Replace or Cancel",
                false,
            );
        } else {
            std::fs::rename(&staged, &target)?;
            imported(
                format!(
                    "Imported {} into {}. Enable it in game settings.",
                    target.file_name().unwrap_or_default().to_string_lossy(),
                    request.kind.label()
                ),
                request.kind,
            );
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(staged);
    }
    result
}
pub fn resolve(choice: Choice) {
    let c = STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .conflict
        .take();
    let Some(mut c) = c else {
        return;
    };
    if matches!(choice, Choice::Cancel) {
        let _ = std::fs::remove_file(c.staged);
        notice("Import cancelled", false);
        return;
    }
    let result = (|| -> Result<()> {
        if matches!(choice, Choice::Replace)
            && std::fs::symlink_metadata(&c.target)
                .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            return Err("Cannot replace a folder or symbolic link".into());
        }
        if matches!(choice, Choice::KeepBoth) {
            if c.kind == Kind::Mods {
                return Err(
                    "A mod ID cannot be installed twice; choose Replace to update it".into(),
                );
            }
            let mut stem = c
                .target
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            while stem.len() > 150 {
                stem.pop();
            }
            let ext = c
                .target
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let dir = c
                .target
                .parent()
                .ok_or("Invalid content folder")?
                .to_owned();
            for i in 1..=10000 {
                let candidate = dir.join(format!("{stem} ({i}).{ext}"));
                if !candidate.exists() {
                    c.target = candidate;
                    break;
                }
                if i == 10000 {
                    return Err("Too many filename conflicts".into());
                }
            }
        }
        std::fs::rename(&c.staged, &c.target)?;
        Ok(())
    })();
    match result {
        Ok(()) => imported(
            "Import completed. Enable it in Installed or game settings.",
            c.kind,
        ),
        Err(e) => {
            let _ = std::fs::remove_file(c.staged);
            notice(format!("Import failed: {e}"), false);
        }
    }
}
pub fn cancel() {
    CANCEL.store(true, Ordering::Relaxed);
    STATE.lock().unwrap_or_else(|e| e.into_inner()).request = None;
    if conflict() {
        resolve(Choice::Cancel);
    } else {
        notice("Import cancelled", false);
    }
}
pub fn tap(x: f32, y: f32) -> bool {
    if !busy() {
        return false;
    }
    let choice = STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .hits
        .iter()
        .find(|(r, _)| x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3])
        .map(|(_, c)| *c);
    if let Some(choice) = choice {
        if matches!(choice, Choice::Cancel) {
            cancel();
        } else {
            resolve(choice);
        }
    }
    true
}
pub fn draw(batch: &mut HudBatch, atlas: &HudAtlas) {
    let mut ui = Ui::mobile(atlas, batch, (0.0, 0.0));
    draw_ui(&mut ui);
    ui.finish();
}
/// Launcher overlays must use its bundled font, including before game fonts exist.
pub fn draw_ui(ui: &mut Ui<'_>) {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.hits.clear();
    if s.conflict.is_none() && !s.working {
        // The launcher already shows the result in its status line. Keep only
        // actionable overlays (progress/cancel and a name-conflict choice).
        return;
    }
    let (name, choices) = if let Some(c) = &s.conflict {
        (
            c.target
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            if c.kind == Kind::Mods {
                vec![("Replace", Choice::Replace), ("Cancel", Choice::Cancel)]
            } else {
                vec![
                    ("Keep both", Choice::KeepBoth),
                    ("Replace", Choice::Replace),
                    ("Cancel", Choice::Cancel),
                ]
            },
        )
    } else {
        (
            s.toast.as_ref().map(|v| v.0.clone()).unwrap_or_default(),
            vec![("Cancel", Choice::Cancel)],
        )
    };
    let w = 440.0f32.min(ui.w - 24.0);
    let vertical = w < 320.0;
    let height = if vertical {
        92.0 + choices.len() as f32 * 44.0
    } else {
        144.0
    };
    let x = (ui.w - w) / 2.0;
    let y = ((ui.h - height) / 2.0).max(0.0);
    ui.rounded(0.0, 0.0, ui.w, ui.h, 0.0, rgba(0, 0, 0, 180));
    ui.rounded(x, y, w, height, 2.0, color::BG);
    ui.text(
        Face::Medium,
        20.0,
        x + 12.0,
        y + 12.0,
        color::TEXT,
        if s.conflict.is_some() {
            "File exists"
        } else {
            "Importing file"
        },
    );
    let name = ui.fit_text(Face::Regular, 16.0, &name, w - 24.0);
    ui.text(Face::Regular, 16.0, x + 12.0, y + 46.0, color::MUTED, &name);
    let width = (w - 24.0) / choices.len() as f32;
    for (i, (label, choice)) in choices.into_iter().enumerate() {
        let rect = if vertical {
            [x + 12.0, y + 86.0 + i as f32 * 44.0, w - 24.0, 38.0]
        } else {
            [x + 12.0 + i as f32 * width, y + 92.0, width - 4.0, 38.0]
        };
        ui.rounded(rect[0], rect[1], rect[2], rect[3], 2.0, color::RAISED);
        ui.text(
            Face::Medium,
            16.0,
            rect[0] + 8.0,
            rect[1] + 7.0,
            color::TEXT,
            label,
        );
        s.hits.push((
            [
                rect[0] * ui.density,
                rect[1] * ui.density,
                rect[2] * ui.density,
                rect[3] * ui.density,
            ],
            choice,
        ));
    }
}
pub fn handle_event(event: &winit::event::WindowEvent) -> bool {
    use winit::{
        event::{ElementState, MouseButton, TouchPhase, WindowEvent},
        keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    };
    if !busy() {
        return false;
    }
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            STATE.lock().unwrap_or_else(|e| e.into_inner()).cursor =
                (position.x as f32, position.y as f32);
            true
        }
        WindowEvent::MouseInput {
            button: MouseButton::Left,
            state,
            ..
        } => {
            if *state == ElementState::Released {
                let (x, y) = STATE.lock().unwrap_or_else(|e| e.into_inner()).cursor;
                tap(x, y);
            }
            true
        }
        WindowEvent::Touch(t) => {
            if t.phase == TouchPhase::Ended {
                tap(t.location.x as f32, t.location.y as f32);
            }
            true
        }
        WindowEvent::KeyboardInput { event, .. } => {
            if event.state == ElementState::Pressed {
                if event.physical_key == PhysicalKey::Code(KeyCode::Escape)
                    || event.logical_key == Key::Named(NamedKey::BrowserBack)
                {
                    cancel();
                } else if event.physical_key == PhysicalKey::Code(KeyCode::Enter) && conflict() {
                    let native = STATE
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .conflict
                        .as_ref()
                        .is_some_and(|c| c.kind == Kind::Mods);
                    resolve(if native {
                        Choice::Cancel
                    } else {
                        Choice::KeepBoth
                    });
                }
            }
            true
        }
        _ => false,
    }
}
