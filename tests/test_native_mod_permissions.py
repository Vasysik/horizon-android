"""packaging/android/native_mod_permissions.py: the permissions a native mod needs, from its source."""

import importlib.util
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "native_mod_permissions",
    Path(__file__).resolve().parent.parent / "packaging/android/native_mod_permissions.py",
)
permissions = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(permissions)


class NeedsTests(unittest.TestCase):
    def mod(self, files):
        folder = Path(tempfile.mkdtemp(prefix="native-mod-"))
        for name, text in files.items():
            path = folder / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
        return folder

    def test_capturing_sound_needs_the_microphone(self):
        for source in [
            ("src/mic.rs", "let stream = device.build_input_stream(&config, on_data, on_err, None)?;"),
            ("src/mic.rs", "let device = host.default_input_device().unwrap();"),
            ("native/capture.c", "AAudioStreamBuilder_setDirection(b, AAUDIO_DIRECTION_INPUT);"),
            ("src/aaudio.rs", "builder.direction(AudioDirection::Input)"),
            ("native/sles.c", "(*engine)->CreateAudioRecorder(engine, &recorder, &src, &sink, 1, ids, req);"),
            ("src/jni.rs", "env.find_class(\"android/media/AudioRecord\")?"),
        ]:
            with self.subTest(source=source[1]):
                self.assertEqual(permissions.needs(self.mod(dict([source]))), ["microphone"])

    def test_playing_sound_or_coding_it_needs_nothing(self):
        folder = self.mod({
            "src/speaker.rs": "let stream = device.build_output_stream(&config, play, err, None)?;",
            "opus/src/opus_encoder.c": "int opus_encode(OpusEncoder *st, const opus_int16 *pcm);",
            "assets/voicechat/textures/mic.png": "build_input_stream",
            "horizon-mixin.toml": "# build_input_stream in a manifest is not a call",
        })
        self.assertEqual(permissions.needs(folder), [])


if __name__ == "__main__":
    unittest.main()
