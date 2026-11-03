//! Gameplay and startup, above the reusable client library.
// The Android hooks (android.rs) call into the activity through JNI and the C library.
#![cfg_attr(not(target_os = "android"), forbid(unsafe_code))]
extern crate self as horizon_client;
pub use horizon_client_lib::*;
#[cfg(target_os = "android")]
pub mod android;
pub mod entry;
pub mod game;
pub mod mod_api;
