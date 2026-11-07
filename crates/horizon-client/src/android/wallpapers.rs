//! User-selected looping live wallpapers for the native Android launcher.
//!
//! Two formats are supported: animated GIF/APNG, decoded by the `image` crate,
//! and MP4, decoded by Android's own media stack through `HorizonMedia`
//! (`wallpaper_video.rs`). Both end up as the same bounded vector of RGBA
//! frames, so the launcher has one drawing path.
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
/// Shared by both formats: the decoded wallpaper lives in RAM.
pub(super) const MAX_FRAMES: usize = 48;
pub const FRAME_WIDTH: u32 = 384;
pub const FRAME_HEIGHT: u32 = 216;
const WALLPAPER_FILE: &str = "launcher-wallpaper.gif";
const PART_FILE: &str = "launcher-wallpaper.partial";
/// Where an MP4 is staged before decoding. Android's media stack takes a file
/// path, and a `content://` URI has none.
const VIDEO_FILE: &str = "launcher-wallpaper.mp4";

/// Which decoder owns the saved wallpaper file.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Format {
    /// `IMAGE`: GIF or APNG.
    Image,
    /// `VIDEO`: MP4 (H.264/HEVC/AV1 as far as the device's media stack decodes).
    Video,
}
impl Format {
    fn of(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".gif") || lower.ends_with(".png") || lower.ends_with(".apng") {
            Some(Self::Image)
        } else if lower.ends_with(".mp4") || lower.ends_with(".m4v") || lower.ends_with(".mov") {
            Some(Self::Video)
        } else {
            None
        }
    }
    /// The file name this format is saved under.
    fn file(self) -> &'static str {
        match self {
            Self::Image => WALLPAPER_FILE,
            Self::Video => VIDEO_FILE,
        }
    }
}

/// Sniff the format from the selected file's own bytes.
///
/// The file name is a hint, not a promise: Android content providers return
/// whatever display name they like, and a GIF renamed `.mp4` must still work.
fn sniff(file: &Path) -> Option<Format> {
    let mut header = [0u8; 12];
    let mut opened = File::open(file).ok()?;
    let read = opened.read(&mut header).ok()?;
    let header = &header[..read];
    if header.starts_with(b"GIF87a") || header.starts_with(b"GIF89a") {
        return Some(Format::Image);
    }
    if header.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(Format::Image);
    }
    // ISO base media (MP4/MOV): a `ftyp` box at offset 4.
    if header.len() >= 8 && &header[4..8] == b"ftyp" {
        return Some(Format::Video);
    }
    None
}

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
    // Whichever format was saved last: only one wallpaper file is kept.
    let saved = if home.join(VIDEO_FILE).exists() {
        home.join(VIDEO_FILE)
    } else {
        home.join(WALLPAPER_FILE)
    };
    let name = clean_name(name);
    let spawned = std::thread::Builder::new()
        .name("Saved wallpaper decode".into())
        .spawn(move || {
            let format = Format::of(&name).or_else(|| sniff(&saved));
            let decoded = match format {
                Some(Format::Video) => decode_video(&saved, name, saved_vm()),
                Some(Format::Image) => decode_gif(&saved, name),
                None => Err("The saved wallpaper is not a GIF, APNG or MP4".into()),
            };
            publish(decoded, true);
        });
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
    for name in [WALLPAPER_FILE, VIDEO_FILE, PART_FILE] {
        let path = home.join(name);
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
/// The JVM kept for the decode threads that Android's media stack needs.
static VM: OnceLock<jni::JavaVM> = OnceLock::new();

/// Remember the JVM once, at startup, so a wallpaper loaded from the saved file
/// long after the picker closed can still reach Android's media stack.
pub fn initialize_vm(vm: jni::JavaVM) {
    let _ = VM.set(vm);
}

fn saved_vm() -> Option<jni::JavaVM> {
    VM.get().cloned()
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
                // The VM the JNI call arrived on decodes MP4 on this thread.
                import(&home, input, &name, Some(vm.clone()))
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

/// The display name, kept only for the Settings row and the status line.
/// Anything with a recognised extension passes; the decoder is chosen by the
/// file's own bytes, not by this name.
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
    if Format::of(&clean).is_some() {
        clean
    } else {
        "wallpaper.gif".into()
    }
}

/// Copy the selected document into `home`, decode it and make it the saved
/// wallpaper. GIF and MP4 take the same path; only the decoder differs.
/// Copy the selected document into `home`, decode it and make it the saved
/// wallpaper. GIF, APNG and MP4 take the same path; only the decoder differs.
///
/// The decoder is chosen by the file's bytes, so a provider that reports a
/// generic or missing display name cannot steer the import wrong.
pub(super) fn import(
    home: &Path,
    mut input: File,
    display_name: &str,
    vm: Option<jni::JavaVM>,
) -> std::result::Result<LiveWallpaper, String> {
    let name = clean_name(display_name);
    fs::create_dir_all(home).map_err(|e| e.to_string())?;
    let temporary = home.join(PART_FILE);
    // An MP4 is staged on disk (Android's media stack wants a path); a GIF is
    // held in the same file and decoded from it, so both formats copy alike.
    // Read the header first: the copy limit depends on what the file is.
    let mut header = [0u8; 12];
    let sniffed = read_header(&mut input, &mut header);
    let staged = (|| -> std::io::Result<()> {
        let mut output = File::create(&temporary)?;
        if let Some(count) = sniffed {
            output.write_all(&header[..count])?;
        }
        let mut buffer = [0u8; 32 * 1024];
        let mut total = sniffed.unwrap_or(0) as u64;
        loop {
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            total = total.saturating_add(count as u64);
            if total > super::wallpaper_video::MAX_VIDEO_BYTES {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "File exceeds the {} MiB import limit",
                        super::wallpaper_video::MAX_VIDEO_BYTES >> 20
                    ),
                ));
            }
            output.write_all(&buffer[..count])?;
        }
        if total < 14 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "File is too small",
            ));
        }
        output.sync_all()
    })();
    if let Err(error) = staged {
        let _ = fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    let format = Format::of(display_name).or_else(|| sniff(&temporary));
    let Some(format) = format else {
        let _ = fs::remove_file(&temporary);
        return Err("Choose an animated GIF, APNG or MP4 clip for a live wallpaper".into());
    };
    // A GIF has a smaller budget than a clip; check it now that the format is
    // known rather than trusting the name.
    if format == Format::Image
        && fs::metadata(&temporary).is_ok_and(|m| m.len() > MAX_GIF_BYTES)
    {
        let _ = fs::remove_file(&temporary);
        return Err("GIF exceeds the 16 MiB import limit".into());
    }
    let decoded = match format {
        Format::Image => decode_gif(&temporary, name),
        Format::Video => decode_video(&temporary, name, vm),
    };
    let wallpaper = match decoded {
        Ok(wallpaper) => wallpaper,
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
    };
    // Keep only one saved wallpaper: the other format's file goes away.
    let other = if format == Format::Image {
        VIDEO_FILE
    } else {
        WALLPAPER_FILE
    };
    let _ = fs::remove_file(home.join(other));
    if let Err(error) = fs::rename(&temporary, home.join(format.file())) {
        let _ = fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    Ok(wallpaper)
}

/// Read up to the buffer's length from `input`, tolerating a short file.
fn read_header(input: &mut File, buffer: &mut [u8]) -> Option<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match input.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(_) => break,
        }
    }
    (filled > 0).then_some(filled)
}

/// Android's media stack decodes the staged clip through `HorizonMedia`.
fn decode_video(
    path: &Path,
    name: String,
    vm: Option<jni::JavaVM>,
) -> std::result::Result<LiveWallpaper, String> {
    let vm = vm.ok_or("Video wallpapers need the Android media stack, which is unavailable")?;
    let clip = super::wallpaper_video::decode(&vm, path)?;
    let frames = clip
        .frames
        .into_iter()
        .filter_map(|(rgba, delay_ms)| {
            cover_bytes(rgba, clip.width, clip.height).map(|rgba| WallpaperFrame { rgba, delay_ms })
        })
        .collect::<Vec<_>>();
    if frames.len() < 2 {
        return Err("Choose a clip with at least two decodable frames".into());
    }
    Ok(LiveWallpaper::new(
        name,
        frames,
        FRAME_WIDTH,
        FRAME_HEIGHT,
    ))
}

/// [`cover`] for an already-decoded RGBA buffer. `None` when the buffer does
/// not match the stated size, so a malformed frame is dropped rather than
/// uploaded at the wrong dimensions.
fn cover_bytes(rgba: Vec<u8>, width: u32, height: u32) -> Option<Vec<u8>> {
    image::RgbaImage::from_raw(width, height, rgba).map(|image| cover(&image))
}

/// Decode an animated GIF or APNG. The decoder is picked from the header, so a
/// provider's display name cannot send an animation to the video path.
fn decode_gif(path: &Path, name: String) -> std::result::Result<LiveWallpaper, String> {
    let mut header_file = File::open(path).map_err(|e| e.to_string())?;
    if header_file.metadata().map_err(|e| e.to_string())?.len() > MAX_GIF_BYTES {
        return Err("GIF exceeds the 16 MiB import limit".into());
    }
    let mut header = [0u8; 10];
    header_file
        .read_exact(&mut header)
        .map_err(|_| "GIF header is incomplete")?;
    let frames = if header.starts_with(b"\x89PNG\r\n\x1a\n") {
        decode_apng(path)?
    } else if &header[..6] == b"GIF87a" || &header[..6] == b"GIF89a" {
        let source_width = u16::from_le_bytes([header[6], header[7]]);
        let source_height = u16::from_le_bytes([header[8], header[9]]);
        if source_width == 0
            || source_height == 0
            || source_width > MAX_SOURCE_SIDE
            || source_height > MAX_SOURCE_SIDE
        {
            return Err("GIF dimensions must be between 1 and 2048 pixels".into());
        }
        decode_gif_frames(path)?
    } else {
        return Err("Selected file is not a GIF or APNG".into());
    };
    if frames.len() < 2 {
        return Err("Choose an animation with at least two frames".into());
    }
    if frames.len() > MAX_FRAMES {
        return Err(format!(
            "Animation has more than {MAX_FRAMES} frames; trim it and try again"
        ));
    }
    Ok(LiveWallpaper::new(
        name,
        frames,
        FRAME_WIDTH,
        FRAME_HEIGHT,
    ))
}

fn decode_gif_frames(path: &Path) -> std::result::Result<Vec<WallpaperFrame>, String> {
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
    Ok(frames)
}

/// APNG: the same `image` decoder chain the GIF path uses, one frame at a time.
fn decode_apng(path: &Path) -> std::result::Result<Vec<WallpaperFrame>, String> {
    let decoder = image::codecs::png::PngDecoder::new(BufReader::new(
        File::open(path).map_err(|e| e.to_string())?,
    ))
    .map_err(|e| format!("Could not decode APNG: {e}"))?;
    let decoder = image::AnimationDecoder::into_frames(decoder);
    let mut frames = Vec::new();
    for decoded in decoder.take(MAX_FRAMES + 1) {
        let frame = decoded.map_err(|e| format!("Could not decode APNG frame: {e}"))?;
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
    Ok(frames)
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

    #[test]
    fn a_format_is_chosen_by_the_file_not_only_its_name() {
        assert_eq!(Format::of("holiday.gif"), Some(Format::Image));
        assert_eq!(Format::of("holiday.PNG"), Some(Format::Image));
        assert_eq!(Format::of("clip.apng"), Some(Format::Image));
        assert_eq!(Format::of("clip.mp4"), Some(Format::Video));
        assert_eq!(Format::of("clip.M4V"), Some(Format::Video));
        assert_eq!(Format::of("notes.txt"), None);
        // The provider's display name survives cleaning when it names a format.
        assert_eq!(clean_name("clip.mp4"), "clip.mp4");
        assert_eq!(clean_name("../escape/clip.mp4"), "clip.mp4");
        // An unusable name falls back to the image default, and the sniffed
        // header decides what the file really is.
        assert_eq!(clean_name("payload.bin"), "wallpaper.gif");
    }

    #[test]
    fn video_and_image_wallpapers_save_under_different_files() {
        assert_ne!(Format::Image.file(), Format::Video.file());
        assert!(Format::Image.file().ends_with(".gif"));
        assert!(Format::Video.file().ends_with(".mp4"));
    }

    #[test]
    fn the_sniffer_reads_the_header_not_the_extension() {
        let dir = std::env::temp_dir().join(format!("oxide-wallpaper-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let write = |name: &str, bytes: &[u8]| {
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            path
        };
        // A GIF renamed .mp4 must still be recognised as an animation.
        assert_eq!(
            sniff(&write("renamed.mp4", b"GIF89a\x01\x00\x01\x00\x00\x00")),
            Some(Format::Image)
        );
        // An MP4's `ftyp` box sits at offset 4.
        assert_eq!(
            sniff(&write("clip.bin", b"\x00\x00\x00\x18ftypisom\x00\x00\x00\x00")),
            Some(Format::Video)
        );
        assert_eq!(sniff(&write("junk.mp4", b"not a media file")), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
