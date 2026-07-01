use domers_core::Rgb;
use domers_outputs::topology::DOME_PIXELS;

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

fn orientation_from_devices(input: &VisualizerInput) -> Option<Quaternion> {
    input
        .orientation_devices
        .iter()
        .find_map(|device| *device)
        .map(|device| device.rotation)
}

pub(crate) fn quaternion_test_frame(input: VisualizerInput) -> Vec<Rgb> {
    let orientation = orientation_from_override(&input)
        .or_else(|| orientation_from_devices(&input))
        .unwrap_or_else(identity_orientation);
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
    let devices: Vec<OrientationDeviceInput> = input
        .orientation_devices
        .iter()
        .filter_map(|device| *device)
        .collect();
    if devices.is_empty() {
        return orientation_from_override(&input).map_or_else(
            || vec![Rgb::BLACK; DOME_PIXELS],
            |orientation| quaternion_multi_spot_frame(orientation, 0.0, 0.2, 1.0),
        );
    }

    let device_count = devices.len();
    DOME_LED_POINTS
        .get_or_init(build_dome_led_points)
        .iter()
        .map(|point| {
            let (x, y, z) = spectrum_quaternion_test_point(point.x, point.y);
            let pixel = (x, y, z);
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
        })
        .collect()
}

fn quaternion_multi_spot_frame(
    orientation: Quaternion,
    hue: f64,
    radius: f64,
    saturation: f64,
) -> Vec<Rgb> {
    DOME_LED_POINTS
        .get_or_init(build_dome_led_points)
        .iter()
        .map(|point| {
            let (x, y, z) = spectrum_quaternion_test_point(point.x, point.y);
            let (rx, ry, rz) = orientation.transform_vector(x, y, z);
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
        })
        .collect()
}
