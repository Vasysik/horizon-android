# Android input probe (developer tool)

This Rust-only NativeActivity probe has no Minecraft or account dependencies.
It creates a plain white native surface, logs winit window/device events with tag
`HorizonInputProbe`, requests pointer capture on focus or a key press, and releases
it on Escape/focus loss. `hasPointerCapture` is asynchronous; the immediate result
can still describe the previous state.

Build its standalone Cargo manifest for `x86_64-linux-android` with the NDK linker
and a throwaway target directory. Package the library in a developer-only APK
whose launcher is `android.app.NativeActivity`, `android:hasCode="false"`, and
`android.app.lib_name="horizon_android_input_probe"`. No DEX or Java host is needed.
Use a separate package such as `dev.horizon.inputprobe` on an isolated emulator.
The surface must submit a first frame or Android will not route pointer events.

API 35 smoke commands, after pressing Escape to release capture:

```sh
adb shell input tap 200 200
adb shell input mouse tap 300 300
adb shell input mouse scroll 300 300 --axis VSCROLL,2 --axis HSCROLL,-1
adb shell input keycombination 59 51
adb logcat -d -s HorizonInputProbe:I '*:S'
```

Android shell `mouse tap` leaves the button-state mask zero; it verifies cursor
position but cannot verify real mouse button transitions. Those transitions,
relative motion, cancellation and repeated snapshots have standalone Rust tests:

```sh
rustc --edition 2021 --test third_party/winit/src/platform_impl/android/mouse_state.rs -o mouse-tests
./mouse-tests
```

The probe does not establish gameplay performance or physical device support.
