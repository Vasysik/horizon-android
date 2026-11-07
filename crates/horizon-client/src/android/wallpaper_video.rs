//! MP4 live wallpapers: Android's own media stack decodes the clip.
//!
//! No video codec is bundled with the APK. `HorizonMedia` (Java) samples the
//! selected file at a bounded rate and size and hands back RGBA frames, and this
//! module turns them into the same frame vector the GIF path produces, so the
//! launcher draws a clip and a GIF through one code path.

use jni::{
    JavaVM,
    objects::{JByteArray, JObject, JValue},
};
use std::path::Path;

/// Longest clip the launcher will hold in RAM.
pub(super) const MAX_VIDEO_MS: i64 = 30_000;
/// A clip may be bigger than a GIF, since only the decoded frames are kept.
pub(super) const MAX_VIDEO_BYTES: u64 = 128 << 20;
/// Frames are decoded at this size and scaled to the atlas slot afterwards.
pub(super) const MAX_CLIP_EDGE: u32 = 640;

/// One decoded clip: RGBA frames with their per-frame delay in milliseconds.
pub(super) struct DecodedClip {
    pub width: u32,
    pub height: u32,
    pub frames: Vec<(Vec<u8>, u32)>,
}

/// Decode `path` (an MP4 already staged in app storage) into RGBA frames.
///
/// Returns an error string fit for the Settings status line on every failure,
/// including a device whose media stack has no decoder for the file's codec.
pub(super) fn decode(vm: &JavaVM, path: &Path) -> Result<DecodedClip, String> {
    let path = path.to_str().ok_or("the wallpaper path is not valid UTF-8")?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("could not attach the media thread: {e}"))?;
    let decoded = (|| -> jni::errors::Result<DecodedClip> {
        let text = JObject::from(env.new_string(path)?);
        let clip = env
            .call_static_method(
                "dev/horizon/android/HorizonMedia",
                "decode",
                "(Ljava/lang/String;J)Ldev/horizon/android/HorizonMedia$Clip;",
                &[JValue::Object(&text), JValue::Long(MAX_VIDEO_MS)],
            )?
            .l()?;
        if clip.is_null() {
            return Err(jni::errors::Error::NullPtr("HorizonMedia.Clip"));
        }
        let width = env.get_field(&clip, "width", "I")?.i()?.clamp(2, 8192) as u32;
        let height = env.get_field(&clip, "height", "I")?.i()?.clamp(2, 8192) as u32;
        let times = long_array(&mut env, &clip, "times")?;
        let pixels = env.convert_byte_array(JByteArray::from(
            env.get_field(&clip, "rgba", "[B")?.l()?,
        ))?;
        let stride = width as usize * height as usize * 4;
        if stride == 0 || pixels.len() % stride != 0 {
            return Err(jni::errors::Error::NullPtr("HorizonMedia.Clip.rgba"));
        }
        let count = pixels.len() / stride;
        if count < 2 {
            return Err(jni::errors::Error::NullPtr("HorizonMedia.Clip.frames"));
        }
        if count > super::wallpapers::MAX_FRAMES {
            return Err(jni::errors::Error::NullPtr("HorizonMedia.Clip.tooManyFrames"));
        }
        // One presentation time per frame, so the clip keeps its own pacing
        // instead of a guessed frame rate.
        let deadline = |index: usize| -> i64 { times.get(index).copied().unwrap_or(0) };
        let frames = (0..count)
            .map(|index| {
                let start = deadline(index).max(0);
                let end = deadline(index + 1).max(start + 1);
                // Vanilla GIF pacing: clamp out of nonsense before the frame
                // timing maths sees it.
                let delay = (end - start).clamp(20, 1000) as u32;
                let from = index * stride;
                (pixels[from..from + stride].to_vec(), delay)
            })
            .collect();
        Ok(DecodedClip {
            width,
            height,
            frames,
        })
    })();
    if decoded.is_err() {
        let _ = env.exception_clear();
    }
    decoded.map_err(|error| format!("Could not decode the MP4: {error}"))
}

/// A `long[]` field as Rust values.
fn long_array(
    env: &mut jni::JNIEnv<'_>,
    object: &JObject<'_>,
    field: &str,
) -> jni::errors::Result<Vec<i64>> {
    // Same shape as `platform.rs`'s int arrays: build the reference wrapper with
    // `From<JObject>` and read the region straight out.
    let array = jni::objects::JLongArray::from(env.get_field(object, field, "[J")?.l()?);
    let length = env.get_array_length(&array)? as usize;
    let mut values = vec![0i64; length];
    env.get_long_array_region(&array, 0, &mut values)?;
    Ok(values)
}
