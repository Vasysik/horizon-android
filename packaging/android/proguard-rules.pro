# Rust reaches these by JNI name: the native bridge, the NativeActivity and the
# documents provider. Keep them and every native method as written.
-keep class dev.horizon.android.** { *; }
-keepclasseswithmembernames,includedescriptorclasses class * {
    native <methods>;
}
# Provided by the device's window manager at run time (Jetpack WindowManager).
-dontwarn androidx.window.extensions.**
-dontwarn androidx.window.sidecar.**
