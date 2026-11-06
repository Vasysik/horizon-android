//! Device snapshots from Android's InputDevice API; NativeActivity has no removal callback.
#[derive(Default)]
pub(super) struct Devices {
    connected: Option<Vec<i32>>,
    removed: Vec<i32>,
}
impl Devices {
    pub(super) fn update(&mut self, ids: &[i32]) {
        if let Some(previous) = &self.connected {
            for id in previous {
                if !ids.contains(id) && !self.removed.contains(id) {
                    self.removed.push(*id);
                }
            }
        }
        self.connected = Some(ids.to_vec());
    }
    pub(super) fn take_removed(&mut self) -> Vec<i32> {
        std::mem::take(&mut self.removed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hot_unplug_reports_each_removed_device_once() {
        let mut devices = Devices::default();
        devices.update(&[1, 2, 3]);
        assert!(devices.take_removed().is_empty());
        devices.update(&[1]);
        devices.update(&[1]);
        assert_eq!(devices.take_removed(), [2, 3]);
        assert!(devices.take_removed().is_empty());
        devices.update(&[1, 2]);
        devices.update(&[1]);
        assert_eq!(devices.take_removed(), [2]);
    }
}
