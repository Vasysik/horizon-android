//! User-selected looping GIF wallpapers for the native Android launcher.
use image::{AnimationDecoder, codecs::gif::GifDecoder, imageops::FilterType};
use jni::{
    JNIEnv,
    objects::{JObject, JString},
};
use std::{
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const MAX_GIF_BYTES: u64 = 16 << 20;
const MAX_SOURCE_SIDE: u16 = 2048;
const MAX_FRAMES: usize = 48;
pub const FRAME_WIDTH: u32 = 384;
pub const FRAME_HEIGHT: u32 = 216;
const WALLPAPER_FILE: &str = "launcher-wallpaper.gif";
const PART_FILE: &str = "launcher-wallpaper.partial";

#[derive(Clone)]
pub struct WallpaperFrame {
    pub rgba: Vec<u8>,
    pub delay_ms: u32,
}

#[derive(Clone)]
pub struct LiveWallpaper {
    name: String,
    frames: Vec<WallpaperFrame>,
    width: u32,
    height: u32,
    total_ms: u64,
    started: Instant,
}
impl LiveWallpaper {
    fn new(name: String, frames: Vec<WallpaperFrame>, width: u32, height: u32) -> Self {
        let total_ms = frames.iter().map(|f| u64::from(f.delay_ms)).sum::<u64>().max(1);
        Self {
            name,
            frames,
            width,
            height,
            total_ms,
            started: Instant::now(),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn is_animated(&self) -> bool {
        self.frames.len() > 1
    }
    pub fn frame(&self, now: Instant) -> (usize, &WallpaperFrame) {
        let mut elapsed = now.saturating_duration_since(self.started).as_millis() as u64
            % self.total_ms;
        for (index, frame) in self.frames.iter().enumerate() {
            if elapsed < u64::from(frame.delay_ms) {
                return (index, frame);
            }
            elapsed -= u64::from(frame.delay_ms);
        }
        let last = self.frames.len().saturating_sub(1);
        (last, &self.frames[last])
    }
    pub fn until_next_frame(&self, now: Instant) -> Duration {
        let mut elapsed = now.saturating_duration_since(self.started).as_millis() as u64
            % self.total_ms;
        for frame in &self.frames {
            let duration = u64::from(frame.delay_ms);
            if elapsed < duration {
                return Duration::from_millis((duration - elapsed).clamp(16, 100));
            }
            elapsed -= duration;
        }
        Duration::from_millis(16)
    }
    /// Scale every decoded frame once to the actual atlas slot size.
    pub fn resize_to(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 || (width == self.width && height == self.height) {
            return;
        }
        let expected = self.width as usize * self.height as usize * 4;
        for frame in &mut self.frames {
            if frame.rgba.len() != expected {
                continue;
            }
            let pixels = std::mem::take(&mut frame.rgba);
            if let Some(image) = image::RgbaImage::from_raw(self.width, self.height, pixels) {
                frame.rgba = image::imageops::resize(&image, width, height, FilterType::Triangle)
                    .into_raw();
            }
        }
        self.width = width;
        self.height = height;
    }
}

pub struct Completion {
    pub wallpaper: std::result::Result<LiveWallpaper, String>,
    pub from_saved_file: bool,
}
#[derive(Default)]
struct State {
    request_home: Option<PathBuf>,
    busy: bool,
    result: Option<Completion>,
}
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
fn state() -> &'static Mutex<State> {
    STATE.get_or_init(|| Mutex::new(State::default()))
}

pub fn busy() -> bool {
    let state = state().lock().unwrap_or_else(|e| e.into_inner());
    state.busy || state.result.is_some()
}
pub fn begin(home: &Path) -> std::result::Result<(), String> {
    {
        let mut state = state().lock().unwrap_or_else(|e| e.into_inner());
        if state.busy {
            return Err("Finish selecting the current wallpaper first".into());
        }
        state.busy = true;
        state.request_home = Some(home.to_owned());
        state.result = None;
    }
    crate::platform::hide();
    if !crate::native_ime::pick_wallpaper() {
        picker_failed("Android file picker is unavailable");
        return Err("Android file picker is unavailable".into());
    }
    Ok(())
}
pub fn load_async(home: &Path, name: &str) -> std::result::Result<(), String> {
    {
        let mut state = state().lock().unwrap_or_else(|e| e.into_inner());
        if state.busy {
            return Err("Another wallpaper operation is already running".into());
        }
        state.busy = true;
        state.request_home = None;
        state.result = None;
    }
    let path = home.join(WALLPAPER_FILE);
    let name = clean_name(name);
    let spawned = std::thread::Builder::new()
        .name("Saved wallpaper decode".into())
        .spawn(move || publish(decode_gif(&path, name), true));
    if let Err(error) = spawned {
        publish(Err(format!("Could not start wallpaper loading: {error}")), true);
        return Err(format!("Could not start wallpaper loading: {error}"));
    }
    Ok(())
}
pub fn picker_failed(message: &str) {
    let mut state = state().lock().unwrap_or_else(|e| e.into_inner());
    state.request_home = None;
    state.busy = false;
    state.result = Some(Completion {
        wallpaper: Err(message.into()),
        from_saved_file: false,
    });
}
pub fn poll() -> Option<Completion> {
    state()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .result
        .take()
}
pub fn remove(home: &Path) -> std::io::Result<()> {
    for name in [WALLPAPER_FILE, PART_FILE] {
        let path = home.join(name);
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
/// The native Activity result callback routes request 4102 here.
pub fn document_result(
    mut env: JNIEnv<'_>,
    activity: JObject<'_>,
    result: i32,
    uri: JString<'_>,
) {
    let home = state()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .request_home
        .take();
    let Some(home) = home else {
        return;
    };
    if result != -1 || uri.is_null() {
        publish(Err("Wallpaper selection cancelled".into()), false);
        return;
    }
    let setup = (|| -> jni::errors::Result<_> {
        let uri_text: String = env.get_string(&uri)?.into();
        Ok((env.get_java_vm()?, env.new_global_ref(activity)?, uri_text))
    })();
    let Ok((vm, activity, uri)) = setup else {
        let _ = env.exception_clear();
        publish(Err("Could not read the selected wallpaper document".into()), false);
        return;
    };
    if !uri.starts_with("content://") || uri.len() > 8192 {
        publish(Err("Unsupported wallpaper document".into()), false);
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("Live wallpaper import".into())
        .spawn(move || {
            let result = (|| -> std::result::Result<LiveWallpaper, String> {
                let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
                let opened = crate::document_bridge::open(
                    &mut env,
                    activity.as_obj(),
                    &uri,
                    WALLPAPER_FILE,
                );
                if opened.is_err() {
                    let _ = env.exception_clear();
                }
                let (input, name) = opened.map_err(|e| e.to_string())?;
                import_gif(&home, input, &name)
            })();
            publish(result, false);
        });
    if let Err(error) = spawned {
        publish(Err(format!("Could not start wallpaper import: {error}")), false);
    }
}

fn publish(result: std::result::Result<LiveWallpaper, String>, from_saved_file: bool) {
    let mut state = state().lock().unwrap_or_else(|e| e.into_inner());
    state.request_home = None;
    state.busy = false;
    state.result = Some(Completion {
        wallpaper: result,
        from_saved_file,
    });
}

fn clean_name(value: &str) -> String {
    let name = Path::new(value)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("wallpaper.gif");
    let clean: String = name
        .chars()
        .filter(|c| !c.is_control() && *c != '/' && *c != '\\')
        .take(72)
        .collect();
    if clean.to_ascii_lowercase().ends_with(".gif") {
        clean
    } else {
        "wallpaper.gif".into()
    }
}

fn import_gif(
    home: &Path,
    mut input: File,
    display_name: &str,
) -> std::result::Result<LiveWallpaper, String> {
    let name = clean_name(display_name);
    if !display_name.to_ascii_lowercase().ends_with(".gif") {
        return Err("Choose an animated GIF file for a live wallpaper".into());
    }
    fs::create_dir_all(home).map_err(|e| e.to_string())?;
    let temporary = home.join(PART_FILE);
    let copied = (|| -> std::io::Result<()> {
        let mut output = File::create(&temporary)?;
        let mut buffer = [0u8; 32 * 1024];
        let mut total = 0u64;
        loop {
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            total = total.saturating_add(count as u64);
            if total > MAX_GIF_BYTES {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "GIF exceeds the 16 MiB import limit",
                ));
            }
            output.write_all(&buffer[..count])?;
        }
        if total < 14 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "GIF file is too small",
            ));
        }
        output.sync_all()
    })();
    if let Err(error) = copied {
        let _ = fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    let wallpaper = match decode_gif(&temporary, name) {
        Ok(wallpaper) => wallpaper,
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
    };
    if let Err(error) = fs::rename(&temporary, home.join(WALLPAPER_FILE)) {
        let _ = fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    Ok(wallpaper)
}

fn decode_gif(path: &Path, name: String) -> std::result::Result<LiveWallpaper, String> {
    let mut header_file = File::open(path).map_err(|e| e.to_string())?;
    if header_file.metadata().map_err(|e| e.to_string())?.len() > MAX_GIF_BYTES {
        return Err("GIF exceeds the 16 MiB import limit".into());
    }
    let mut header = [0u8; 10];
    header_file
        .read_exact(&mut header)
        .map_err(|_| "GIF header is incomplete")?;
    if &header[..6] != b"GIF87a" && &header[..6] != b"GIF89a" {
        return Err("Selected file is not a GIF".into());
    }
    let source_width = u16::from_le_bytes([header[6], header[7]]);
    let source_height = u16::from_le_bytes([header[8], header[9]]);
    if source_width == 0
        || source_height == 0
        || source_width > MAX_SOURCE_SIDE
        || source_height > MAX_SOURCE_SIDE
    {
        return Err("GIF dimensions must be between 1 and 2048 pixels".into());
    }
    let decoder = GifDecoder::new(BufReader::new(File::open(path).map_err(|e| e.to_string())?))
        .map_err(|e| format!("Could not decode GIF: {e}"))?;
    let mut frames = Vec::new();
    for decoded in decoder.into_frames().take(MAX_FRAMES + 1) {
        let frame = decoded.map_err(|e| format!("Could not decode GIF frame: {e}"))?;
        let (numerator, denominator) = frame.delay().numer_denom_ms();
        let delay_ms = if denominator == 0 {
            100
        } else {
            (numerator / denominator).clamp(20, 1000)
        };
        let image = frame.into_buffer();
        frames.push(WallpaperFrame {
            rgba: cover(&image),
            delay_ms,
        });
    }
    if frames.len() < 2 {
        return Err("Choose an animated GIF with at least two frames".into());
    }
    if frames.len() > MAX_FRAMES {
        return Err(format!(
            "GIF has more than {MAX_FRAMES} frames; trim it and try again"
        ));
    }
    Ok(LiveWallpaper::new(
        name,
        frames,
        FRAME_WIDTH,
        FRAME_HEIGHT,
    ))
}

fn cover(image: &image::RgbaImage) -> Vec<u8> {
    let (source_width, source_height) = image.dimensions();
    // Crop to 16:9 before resizing; resizing a very wide or tall source first
    // could allocate hundreds of megabytes for pixels that are then discarded.
    let (crop_width, crop_height) = if u64::from(source_width) * 9
        > u64::from(source_height) * 16
    {
        (
            ((u64::from(source_height) * 16 / 9) as u32).max(1),
            source_height,
        )
    } else {
        (
            source_width,
            ((u64::from(source_width) * 9 / 16) as u32).max(1),
        )
    };
    let x = source_width.saturating_sub(crop_width) / 2;
    let y = source_height.saturating_sub(crop_height) / 2;
    let cropped = image::imageops::crop_imm(image, x, y, crop_width, crop_height).to_image();
    image::imageops::resize(&cropped, FRAME_WIDTH, FRAME_HEIGHT, FilterType::Triangle).into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallpaper_frames_loop_by_their_individual_delays() {
        let wallpaper = LiveWallpaper::new(
            "test.gif".into(),
            vec![
                WallpaperFrame {
                    rgba: vec![0; 4],
                    delay_ms: 40,
                },
                WallpaperFrame {
                    rgba: vec![0; 4],
                    delay_ms: 60,
                },
            ],
            1,
            1,
        );
        let start = wallpaper.started;
        assert_eq!(wallpaper.frame(start).0, 0);
        assert_eq!(wallpaper.frame(start + Duration::from_millis(39)).0, 0);
        assert_eq!(wallpaper.frame(start + Duration::from_millis(40)).0, 1);
        assert_eq!(wallpaper.frame(start + Duration::from_millis(100)).0, 0);
    }

    #[test]
    fn wallpaper_size_is_limited_to_a_phone_background() {
        let pixels = vec![255; 1920 * 1080 * 4];
        let image = image::RgbaImage::from_raw(1920, 1080, pixels).unwrap();
        assert_eq!(cover(&image).len(), (FRAME_WIDTH * FRAME_HEIGHT * 4) as usize);
    }
}
