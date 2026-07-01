use domers_core::Rgb;

use crate::{
    color_util::{hsv_to_rgb, light_paint},
    geometry::{build_dome_led_points, distance3, DOME_LED_POINTS},
    input::{OrientationDeviceInput, VisualizerInput},
    math::{max_axis_by_abs, spectrum_quaternion_test_point},
    quaternion::Quaternion,
};

fn identity_orientation() -> Quaternion {
    Quaternion {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    }
}

fn orientation_from_override(input: &VisualizerInput) -> Option<Quaternion> {
    input.orientation_override.map(|orientation| {
        Quaternion::from_yaw_pitch_roll(orientation.yaw, orientation.pitch, orientation.roll)
    })
}

/// Spectrum `OrientationInput.deviceRotation(deviceId)` — missing device → identity.
fn device_rotation(input: &VisualizerInput, device_id: i32) -> Quaternion {
    input
        .orientation_devices
        .iter()
        .filter_map(|device| *device)
        .find(|device| i32::from(device.device_id) == device_id)
        .map_or_else(identity_orientation, |device| device.rotation)
}

fn live_devices(input: &VisualizerInput) -> Vec<OrientationDeviceInput> {
    let mut devices: Vec<_> = input
        .orientation_devices
        .iter()
        .filter_map(|device| *device)
        .collect();
    devices.sort_by_key(|device| device.device_id);
    devices
}

pub(crate) fn quaternion_test_frame(input: VisualizerInput) -> Vec<Rgb> {
    let orientation = orientation_from_override(&input)
        .unwrap_or_else(|| device_rotation(&input, input.orientation_device_spotlight));
    DOME_LED_POINTS
        .get_or_init(build_dome_led_points)
        .iter()
        .map(|point| {
            let (x, y, z) = spectrum_quaternion_test_point(point.x, point.y);
            let (x, y, z) = orientation.transform_vector(x, y, z);
            match max_axis_by_abs(x, y, z) {
                0 => Rgb::from_u24(0xff_00_00),
                1 => Rgb::from_u24(0x00_ff_00),
                _ => Rgb::from_u24(0x00_00_ff),
            }
        })
        .collect()
}

pub(crate) fn quaternion_multi_test_frame(input: VisualizerInput) -> Vec<Rgb> {
    DOME_LED_POINTS
        .get_or_init(build_dome_led_points)
        .iter()
        .enumerate()
        .map(|(point_index, _point)| quaternion_multi_test_color_at(&input, point_index))
        .collect()
}

/// Per-pixel multi-test color for one hemisphere sample (spot at `(0, 1, 0)`).
pub(crate) fn quaternion_multi_test_color_at(input: &VisualizerInput, point_index: usize) -> Rgb {
    let point = &DOME_LED_POINTS.get_or_init(build_dome_led_points)[point_index];
    let (x, y, z) = spectrum_quaternion_multi_point(point.x, point.y);
    let pixel = (x, y, z);

    let devices = live_devices(input);
    if devices.is_empty() {
        if let Some(orientation) = orientation_from_override(input) {
            return quaternion_multi_spot_at(orientation, pixel, 0.0, 0.2, 1.0);
        }
        return Rgb::BLACK;
    }

    let device_count = devices.len();
    let mut color = Rgb::BLACK;
    for (index, device) in devices.iter().enumerate() {
        let (rx, ry, rz) = device.rotation.transform_vector(pixel.0, pixel.1, pixel.2);
        let distance = distance3(rx, ry, rz, 0.0, 1.0, 0.0);
        let (radius, saturation) = if device.action_flag == 1 {
            (0.4, 0.0)
        } else {
            (0.2, 1.0)
        };
        if distance < radius {
            let value = ((radius - distance) / radius).clamp(0.0, 1.0);
            let hue = f64::from(u32::try_from(index).expect("device index fits in u32"))
                / f64::from(u32::try_from(device_count).expect("device count fits in u32"));
            color = light_paint(color, hsv_to_rgb(hue, saturation, value));
        }
    }
    color
}

fn spectrum_quaternion_multi_point(x: f64, y: f64) -> (f64, f64, f64) {
    let px = 2.0 * x - 1.0;
    let py = 1.0 - 2.0 * y;
    let z = if px * px + py * py > 1.0 {
        0.0
    } else {
        (1.0 - px * px - py * py).sqrt()
    };
    (px, py, z)
}

fn quaternion_multi_spot_at(
    orientation: Quaternion,
    pixel: (f64, f64, f64),
    hue: f64,
    radius: f64,
    saturation: f64,
) -> Rgb {
    let (rx, ry, rz) = orientation.transform_vector(pixel.0, pixel.1, pixel.2);
    let distance = distance3(rx, ry, rz, 0.0, 1.0, 0.0);
    if distance < radius {
        hsv_to_rgb(
            hue,
            saturation,
            ((radius - distance) / radius).clamp(0.0, 1.0),
        )
    } else {
        Rgb::BLACK
    }
}
