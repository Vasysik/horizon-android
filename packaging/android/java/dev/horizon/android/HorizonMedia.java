package dev.horizon.android;

import android.graphics.Bitmap;
import android.media.MediaMetadataRetriever;

/**
 * MP4 support for the launcher's live wallpapers. Only Android's own media
 * stack ships in this APK; no video codec is bundled.
 *
 * <p>Every method is called from Rust on the wallpaper worker thread, never from
 * a game tick. Frames are sampled at a bounded rate and size so a long or
 * high-resolution clip can never hold more than {@link #MAX_FRAMES} frames in
 * memory, and pixels come back as RGBA bytes ready to upload.
 */
public final class HorizonMedia {
    /** Matches the GIF path's budget: the decoded clip lives in RAM. */
    public static final int MAX_FRAMES = 48;
    /** Frames are scaled so their longer edge is at most this, before the crop. */
    public static final int MAX_EDGE = 640;

    private HorizonMedia() {}

    /** A decoded clip: RGBA frames plus the real timing and size they came from. */
    public static final class Clip {
        public final int width;
        public final int height;
        /** Total clip duration, for reporting; 0 when the container had none. */
        public final long durationMs;
        /** Presentation time of frame {@code i} in milliseconds. */
        public final long[] times;
        /** Frames packed back to back, 4 bytes of RGBA per pixel. */
        public final byte[] rgba;

        Clip(int width, int height, long durationMs, long[] times, byte[] rgba) {
            this.width = width;
            this.height = height;
            this.durationMs = durationMs;
            this.times = times;
            this.rgba = rgba;
        }
    }

    /**
     * How many frames to sample: bounded by {@link #MAX_FRAMES} and by a 20 fps
     * floor, so a short clip still reads as motion and a long one stays cheap.
     */
    public static int frameCount(long durationMs) {
        if (durationMs <= 0L) {
            return 1;
        }
        long frames = (durationMs * 20L + 999L) / 1000L;
        return (int) Math.max(2L, Math.min(MAX_FRAMES, frames));
    }

    /**
     * Decodes a clip into RAM.
     *
     * @param path         absolute path; a content:// URI has no filesystem path,
     *                     so the caller stages the selected file in app storage first
     * @param maxDurationMs reject longer clips before decoding anything
     * @return the clip, never null
     * @throws IllegalArgumentException when the file has no video track or is too long
     * @throws IllegalStateException    when decoding fails
     */
    public static Clip decode(String path, long maxDurationMs) {
        MediaMetadataRetriever retriever = new MediaMetadataRetriever();
        try {
            retriever.setDataSource(path);
            long durationMs = parseLong(retriever.extractMetadata(
                    MediaMetadataRetriever.METADATA_KEY_DURATION));
            if (maxDurationMs > 0L && durationMs > maxDurationMs) {
                throw new IllegalArgumentException("clip is "
                        + Math.ceil(durationMs / 1000.0) + " s; the limit is "
                        + (maxDurationMs / 1000) + " s");
            }
            String hasVideo = retriever.extractMetadata(
                    MediaMetadataRetriever.METADATA_KEY_HAS_VIDEO);
            if (!"yes".equals(hasVideo)) {
                throw new IllegalArgumentException("the file has no video track");
            }
            int frames = frameCount(durationMs);
            long intervalMs = durationMs > 0L ? durationMs / frames : 50L;
            if (intervalMs < 1L) {
                intervalMs = 1L;
            }
            long[] times = new long[frames];
            byte[] rgba = null;
            int width = 0;
            int height = 0;
            int written = 0;
            for (int i = 0; i < frames; i++) {
                long atMs = i * intervalMs;
                // CLOSEST decodes forward to the exact time; a wallpaper import
                // is one-off work on a worker thread, so accuracy wins over speed.
                Bitmap frame = retriever.getFrameAtTime(atMs * 1000L,
                        MediaMetadataRetriever.OPTION_CLOSEST);
                if (frame == null) {
                    break;
                }
                if (rgba == null) {
                    Bitmap scaled = scale(frame);
                    if (scaled != frame) {
                        frame.recycle();
                    }
                    frame = scaled;
                    width = frame.getWidth();
                    height = frame.getHeight();
                    rgba = new byte[width * height * 4 * frames];
                } else if (frame.getWidth() != width || frame.getHeight() != height) {
                    // A mid-stream size change: normalise it to the first frame.
                    Bitmap scaled = scale(frame);
                    if (scaled != frame) {
                        frame.recycle();
                    }
                    frame = scaled;
                }
                written = read(frame, rgba, width, height, i);
                times[i] = atMs;
                frame.recycle();
            }
            if (rgba == null || written < 2) {
                throw new IllegalStateException(
                        "the clip decoded to " + Math.max(0, written) + " frames");
            }
            if (written < frames) {
                // The stream ended early; keep what decoded.
                byte[] trimmed = new byte[width * height * 4 * written];
                System.arraycopy(rgba, 0, trimmed, 0, trimmed.length);
                rgba = trimmed;
                times = java.util.Arrays.copyOf(times, written);
            }
            return new Clip(width, height, durationMs, times, rgba);
        } catch (IllegalArgumentException | IllegalStateException error) {
            throw error;
        } catch (RuntimeException error) {
            throw new IllegalStateException("could not read the video: " + message(error));
        } finally {
            try {
                retriever.release();
            } catch (RuntimeException ignored) {
                // A failed setDataSource leaves nothing to release.
            }
        }
    }

    /** Copies one bitmap into {@code out} at frame {@code index}, returning the frames written. */
    private static int read(Bitmap frame, byte[] out, int width, int height, int index) {
        int[] pixels = new int[width * height];
        frame.getPixels(pixels, 0, width, 0, 0, width, height);
        int base = index * width * height * 4;
        for (int i = 0; i < pixels.length; i++) {
            int pixel = pixels[i];
            int at = base + i * 4;
            // Bitmap.getPixels returns ARGB_8888: alpha in the high byte, then
            // red, green, blue. The launcher uploads RGBA, so the red and blue
            // bytes swap here.
            out[at] = (byte) ((pixel >> 16) & 0xff);      // R
            out[at + 1] = (byte) ((pixel >> 8) & 0xff);   // G
            out[at + 2] = (byte) (pixel & 0xff);          // B
            out[at + 3] = (byte) ((pixel >>> 24) & 0xff); // A
        }
        return index + 1;
    }

    /** Scales a frame down to {@link #MAX_EDGE}, even dimensions, keeping its aspect. */
    private static Bitmap scale(Bitmap frame) {
        int source = Math.max(frame.getWidth(), frame.getHeight());
        if (source <= MAX_EDGE) {
            return frame;
        }
        float ratio = (float) MAX_EDGE / source;
        int width = Math.max(2, ((int) (frame.getWidth() * ratio)) & ~1);
        int height = Math.max(2, ((int) (frame.getHeight() * ratio)) & ~1);
        return Bitmap.createScaledBitmap(frame, width, height, true);
    }

    private static String message(Throwable error) {
        String text = error.getMessage();
        return text == null || text.isEmpty() ? error.getClass().getSimpleName() : text;
    }

    private static long parseLong(String value) {
        if (value == null) {
            return 0L;
        }
        try {
            return Long.parseLong(value.trim());
        } catch (NumberFormatException error) {
            return 0L;
        }
    }
}
