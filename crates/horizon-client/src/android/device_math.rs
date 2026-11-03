//! Pure sensor coordinate conversion, also tested without Android hardware.
pub fn gyro_delta(rate: [f32; 3], seconds: f32, rotation: i32, sensitivity: f32) -> [f32; 2] {
    if !seconds.is_finite()
        || seconds <= 0.0
        || seconds >= 0.1
        || !sensitivity.is_finite()
        || !rate.iter().all(|v| v.is_finite())
    {
        return [0.0; 2];
    }
    let [x, y, _] = rate;
    let (pitch, yaw) = match rotation.rem_euclid(4) {
        0 => (x, y),
        1 => (-y, x),
        2 => (-x, -y),
        _ => (y, -x),
    };
    let gain = seconds * (180.0 / std::f32::consts::PI) * sensitivity.clamp(0.1, 3.0);
    [-yaw * gain, -pitch * gain]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn orientation_rotates_axes_without_changing_magnitude() {
        let first = gyro_delta([1.0, 2.0, 0.0], 0.01, 0, 1.0);
        assert_eq!(
            gyro_delta([1.0, 2.0, 0.0], 0.01, 1, 1.0),
            [first[1], -first[0]]
        );
        assert_eq!(
            gyro_delta([1.0, 2.0, 0.0], 0.01, 2, 1.0),
            [-first[0], -first[1]]
        );
        assert_eq!(
            gyro_delta([1.0, 2.0, 0.0], 0.01, 3, 1.0),
            [-first[1], first[0]]
        );
    }
    #[test]
    fn time_and_sensitivity_are_linear() {
        let d = gyro_delta([0.0, 1.0, 0.0], 0.01, 0, 1.0);
        assert_eq!(
            gyro_delta([0.0, 1.0, 0.0], 0.02, 0, 1.0),
            d.map(|v| v * 2.0)
        );
        assert_eq!(
            gyro_delta([0.0, 1.0, 0.0], 0.01, 0, 2.0),
            d.map(|v| v * 2.0)
        );
    }
    #[test]
    fn stale_and_invalid_samples_do_not_spin_camera() {
        for t in [0.0, -1.0, 0.1, 1.0, f32::NAN] {
            assert_eq!(gyro_delta([1.0; 3], t, 0, 1.0), [0.0; 2]);
        }
        assert_eq!(gyro_delta([f32::INFINITY; 3], 0.01, 0, 1.0), [0.0; 2]);
        assert_eq!(gyro_delta([1.0; 3], 0.01, 0, f32::NAN), [0.0; 2]);
    }
}
