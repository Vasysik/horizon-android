//! Client-wide display/performance preferences, independent of touch layouts.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DISPLAY_RATES: [u16; 7] = [0, 30, 60, 90, 120, 144, 165];

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub display_hz: u16,
    pub performance_hints: bool,
    pub gpu_profile: bool,
}

impl Settings {
    pub fn sanitize(&mut self) {
        if !DISPLAY_RATES.contains(&self.display_hz) {
            self.display_hz = 0;
        }
    }

    /// Upgrade the old touch-file preferences only when the client has not
    /// saved the corresponding choice. Explicit Off/System default wins.
    pub fn migrate_missing(&mut self, client: &Value, touch: &Value) {
        if client.get("display_hz").is_none() {
            self.display_hz = touch["display_hz"]
                .as_u64()
                .and_then(|v| u16::try_from(v).ok())
                .unwrap_or(0);
        }
        if client.get("performance_hints").is_none() {
            self.performance_hints = touch["performance_hints"].as_bool().unwrap_or(false);
        }
        if client.get("gpu_profile").is_none() {
            self.gpu_profile = touch["gpu_profile"].as_bool().unwrap_or(false);
        }
        self.sanitize();
    }

    pub fn display_label(self) -> String {
        if self.display_hz == 0 {
            "System default".into()
        } else {
            format!("{} Hz (request)", self.display_hz)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn old_touch_preferences_migrate_without_overriding_explicit_off() {
        let old = json!({"display_hz":120,"performance_hints":true,"gpu_profile":true});
        let mut settings = Settings::default();
        settings.migrate_missing(&json!({}), &old);
        assert_eq!(settings.display_hz, 120);
        assert_eq!(settings.display_label(), "120 Hz (request)");
        assert!(settings.performance_hints && settings.gpu_profile);
        let saved = json!({"display_hz":0,"performance_hints":false,"gpu_profile":false});
        let mut settings: Settings = serde_json::from_value(saved.clone()).unwrap();
        settings.migrate_missing(&saved, &old);
        assert_eq!(settings.display_hz, 0);
        assert_eq!(settings.display_label(), "System default");
        assert!(!settings.performance_hints && !settings.gpu_profile);
    }

    #[test]
    fn corrupt_old_values_and_unknown_frequencies_use_safe_defaults() {
        for hz in [json!(999), json!(u64::MAX), json!("120"), Value::Null] {
            let mut settings = Settings::default();
            settings.migrate_missing(&json!({}), &json!({"display_hz":hz}));
            assert_eq!(settings.display_hz, 0);
            assert!(!settings.performance_hints && !settings.gpu_profile);
        }
    }
}
