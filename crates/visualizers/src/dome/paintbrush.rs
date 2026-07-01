use domers_core::Rgb;

use crate::{
    color_util::{hsv_to_rgb, light_paint},
    geometry::{build_dome_led_points, distance3, hemisphere_point, DOME_LED_POINTS},
    input::VisualizerInput,
    math::{paintbrush_frame_in_cycle, paintbrush_twinkle, spectrum_nudge},
    quaternion::Quaternion,
    rng::DotNetRandom,
};

pub(crate) fn quaternion_paintbrush_frame(input: VisualizerInput) -> Vec<Rgb> {
    let orientation = input.orientation_override.map_or_else(
        || idle_paintbrush_orientation(input),
        |orientation| {
            Quaternion::from_yaw_pitch_roll(orientation.yaw, orientation.pitch, orientation.roll)
        },
    );
    let frame_in_cycle = u64::from(paintbrush_frame_in_cycle(input));
    let trail_orientations = paintbrush_trail_orientations(input, frame_in_cycle);
    let ripple_counter = paintbrush_ripple_counter(frame_in_cycle);
    let stamp_frame = paintbrush_stamp_frame(frame_in_cycle);
    let threshold_factor = 0.25 + f64::from(input.volume.clamp(0.0, 1.0)) + 0.01;
    let threshold = 2.0 / threshold_factor;
    let saturation = (1.3 / f64::from(input.volume.max(0.01)) - 1.0).clamp(0.2, 1.0);

    DOME_LED_POINTS
        .get_or_init(build_dome_led_points)
        .iter()
        .map(|point| {
            let (x, y, z) = hemisphere_point(point.x, point.y);
            let (rx, ry, rz) = orientation.transform_vector(x, y, z);
            let distance = distance3(rx, ry, rz, -1.0, 0.0, 0.0).max(0.001);
            let neg_distance = distance3(rx, ry, rz, 1.0, 0.0, 0.0).max(0.001);
            let potential = 1.0 / (distance * neg_distance);
            let strength = potential - threshold;
            let hue = (1.0 + orientation.w) / 2.0;

            let mut color = paintbrush_twinkle(input, point.index, z);
            if strength > 0.0 {
                color = light_paint(color, hsv_to_rgb(hue, saturation, 1.0));
            }

            for (trail_orientation, fade) in &trail_orientations {
                let (tx, ty, tz) = trail_orientation.transform_vector(x, y, z);
                let trail_distance = distance3(tx, ty, tz, -1.0, 0.0, 0.0).max(0.001);
                let trail_neg_distance = distance3(tx, ty, tz, 1.0, 0.0, 0.0).max(0.001);
                let trail_potential = 1.0 / (trail_distance * trail_neg_distance);
                if trail_potential > threshold {
                    let trail_hue = (1.0 + trail_orientation.w) / 2.0;
                    color = light_paint(color, hsv_to_rgb(trail_hue, saturation, *fade));
                }
            }

            if ripple_counter > 0.0 {
                let ripple_radius = ripple_counter / 300.0;
                let distance_to_spot = distance3(rx, ry, rz, -1.0, 0.0, 0.0);
                if (distance_to_spot - ripple_radius).abs() < 0.012 {
                    let ripple_saturation = (1.0 - ripple_counter / 600.0).clamp(0.0, 1.0);
                    let ripple_value = (1.0 - ripple_counter / 800.0).clamp(0.0, 1.0);
                    color = light_paint(color, hsv_to_rgb(hue, ripple_saturation, ripple_value));
                }
            }

            if let Some(stamp_frame) = stamp_frame {
                let distance_to_spot = distance3(rx, ry, rz, -1.0, 0.0, 0.0);
                if paintbrush_stamp_ring(distance_to_spot, stamp_frame) {
                    color = hsv_to_rgb(hue, 0.2, 1.0);
                }
            }

            color
        })
        .collect()
}
pub(crate) fn idle_paintbrush_orientation(input: VisualizerInput) -> Quaternion {
    idle_paintbrush_orientation_at(input.volume, paintbrush_frame_in_cycle(input))
}

pub(crate) fn idle_paintbrush_orientation_at(volume: f32, frame_in_cycle: u32) -> Quaternion {
    let level = f64::from(volume.clamp(0.0, 1.0));
    let mut random = DotNetRandom::new(0);
    let mut yaw = 0.0;
    let mut pitch = -0.25;
    let mut roll = 0.0;
    let mut yaw_momentum = 0.0;
    let mut pitch_momentum = 0.0005;
    let mut roll_momentum = 0.0;

    for _ in 0..=frame_in_cycle {
        yaw_momentum = (yaw_momentum + spectrum_nudge(&mut random, 0.0001)).clamp(-0.001, 0.001);
        roll_momentum = (roll_momentum + spectrum_nudge(&mut random, 0.0001)).clamp(-0.001, 0.001);
        pitch_momentum =
            (pitch_momentum + spectrum_nudge(&mut random, 0.0001)).clamp(-0.001, 0.001);

        let motion_scale = 4.0 * (level + 0.25);
        yaw += motion_scale * yaw_momentum;
        pitch += motion_scale * pitch_momentum;
        roll += motion_scale * roll_momentum;
    }

    let yaw = std::f64::consts::TAU * yaw;
    let pitch = std::f64::consts::TAU * pitch;
    let roll = std::f64::consts::TAU * roll;
    Quaternion::from_yaw_pitch_roll(yaw, pitch, roll)
}

pub(crate) fn paintbrush_trail_orientations(
    input: VisualizerInput,
    frame_in_cycle: u64,
) -> Vec<(Quaternion, f64)> {
    if input.orientation_override.is_some() || frame_in_cycle == 0 {
        return Vec::new();
    }

    [8_u64, 18, 32, 56, 88, 128]
        .into_iter()
        .filter(|offset| frame_in_cycle >= *offset)
        .map(|offset| {
            let frame = (frame_in_cycle - offset)
                .try_into()
                .expect("paintbrush trail frame fits in u32");
            let offset_f64 = f64::from(u32::try_from(offset).expect("trail offset fits in u32"));
            let fade = (1.0 - offset_f64 / 150.0).clamp(0.12, 0.75);
            (idle_paintbrush_orientation_at(input.volume, frame), fade)
        })
        .collect()
}

pub(crate) fn paintbrush_ripple_counter(frame_in_cycle: u64) -> f64 {
    pub(crate) const RIPPLE_COOLDOWN_FRAMES: u64 = 100;
    if frame_in_cycle <= RIPPLE_COOLDOWN_FRAMES {
        return 0.0;
    }
    let frame = frame_in_cycle - RIPPLE_COOLDOWN_FRAMES;
    if frame >= 1_000 {
        0.0
    } else {
        f64::from(u32::try_from(frame).expect("ripple frame fits in u32"))
    }
}

pub(crate) fn paintbrush_stamp_frame(frame_in_cycle: u64) -> Option<u64> {
    pub(crate) const STAMP_START_FRAMES: u64 = 1_001;
    if frame_in_cycle < STAMP_START_FRAMES {
        return None;
    }
    let frame = frame_in_cycle - STAMP_START_FRAMES;
    (frame < 90).then_some(frame)
}

pub(crate) fn paintbrush_stamp_ring(distance_to_spot: f64, stamp_frame: u64) -> bool {
    if stamp_frame < 45 {
        distance_to_spot.rem_euclid(0.4) < 0.05
    } else {
        let cooldown = 10.0
            - f64::from(u32::try_from(stamp_frame - 45).expect("stamp frame fits in u32")) / 4.5;
        let ring_distance = 2.4 - (1.8 / (4.0 - cooldown / 2.0)).clamp(0.0, 2.4);
        let half_width = 0.003 * cooldown * cooldown;
        (ring_distance - half_width..=ring_distance + half_width).contains(&distance_to_spot)
    }
}
