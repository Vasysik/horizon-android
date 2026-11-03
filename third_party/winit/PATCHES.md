# Winit Android input patch

Upstream: winit 0.30.13 from crates.io, original Apache-2.0 license retained.

Android-only mouse hover/buttons/wheel/relative events and modifier snapshots
are forwarded as normal winit events. Android touch and absolute mouse coordinates use the same safe-area/aspect
rectangle as presentation, including resolution scaling. New touches outside
that rectangle are ignored; existing gestures still receive their releases.
Desktop backends retain their upstream behaviour. A stock Android NativeActivity host is used for full
mouse input; no project Java/Kotlin host is included.
