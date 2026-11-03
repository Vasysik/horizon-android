//! Android's controllers (Starling's): the pads android.rs keeps from the system's key and
//! motion events (horizon_client::gamepad::android), their makers for the button pictures, and
//! their motors through the activity (StarlingActivity.vibratePad). The desktop's are gilrs'
//! (pad/hardware.rs).
use super::{super::Game, Pads};
use horizon_client::gamepad::{GlyphStyle, PadState, Pulse, Rumble};
use serde_json::{Value, json};

pub(in crate::game) use super::no_router::{PadId, PadRouter};

/// The controllers used so far and the one playing.
#[derive(Default)]
pub(super) struct Devices {
    /// Android's device ids, with their maker (USB vendor id), asked of Android once each.
    seen: Vec<(i32, Option<u16>)>,
    active: Option<i32>,
}

impl Pads {
    /// Splitscreen gives pads away; there are no gilrs pads here to close.
    pub(in crate::game) fn release_hardware(&mut self) {}

    /// The active pad's state; its maker picks the button pictures.
    pub(super) fn read(&mut self) -> PadState {
        let Some((device, state)) = crate::android::pad_state() else {
            return PadState::default();
        };
        let vendor = match self.devices.seen.iter().find(|(d, _)| *d == device) {
            Some((_, vendor)) => *vendor,
            None => {
                let vendor = crate::android::pad_vendor(device);
                log::info!(
                    "Controller in use: {} (vendor {:04x})",
                    crate::android::pad_name(device).unwrap_or_default(),
                    vendor.unwrap_or(0)
                );
                self.devices.seen.push((device, vendor));
                vendor
            }
        };
        self.devices.active = Some(device);
        self.style = GlyphStyle::of_vendor(vendor);
        state
    }
}

impl Game<'_> {
    pub(super) fn pad_output_off(&mut self) {}
    pub(super) fn pad_output_frame(&mut self) {}
    pub(in crate::game) fn pad_close_output(&mut self) {}

    /// One pulse on the active pad's motors, the heavy and the light one (the shaped steps are
    /// the DualSense's own output's); a pad without motors stays still.
    pub(super) fn pad_vibrate(&mut self, _: Rumble, pulse: Pulse) {
        let Some(device) = self.pad.devices.active else {
            return;
        };
        let level = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as i32;
        crate::android::vibrate_pad(
            device,
            i32::try_from(pulse.millis).unwrap_or(i32::MAX),
            level(pulse.strong),
            level(pulse.weak),
        );
    }

    /// The hardware part of `pad status`.
    pub(super) fn pad_devices_status(&self) -> Value {
        let devices = &self.pad.devices;
        let pads: Vec<Value> = devices
            .seen
            .iter()
            .map(|(device, vendor)| {
                json!({
                    "device": device,
                    "name": crate::android::pad_name(*device),
                    "vendor": vendor,
                    "active": devices.active == Some(*device),
                })
            })
            .collect();
        json!({ "backend": true, "controllers": pads })
    }
}
