use domers_core::Rgb;

use crate::{input::VisualizerInput, rng::DotNetRandom};

pub(crate) const DOME_GLOBAL_FADE_SPEED: f64 = 0.0;
pub(crate) const DOME_GLOBAL_HUE_SPEED: f64 = 1.0;
pub(crate) const DOME_RADIAL_CENTER_SPEED: f64 = 0.0;
pub(crate) const DOME_RADIAL_CENTER_ANGLE: f64 = 0.0;
pub(crate) const DOME_RADIAL_CENTER_DISTANCE: f64 = 0.0;
pub(crate) const DOME_RADIAL_EFFECT: i32 = 0;
pub(crate) const DOME_RADIAL_FREQUENCY: f64 = 1.0;
/// Spectrum `domeRadialSize` from `spectrum_default_config.xml` used for goldens.
pub(crate) const DOME_RADIAL_SIZE: f64 = 1.0;
pub(crate) const SPLAT_FADE: f64 = 0.96;

pub(crate) fn polar_to_cartesian(angle: f64, distance: f64) -> (f64, f64) {
    (angle.cos() * distance, angle.sin() * distance)
}

/// Compute `(val, gradient_val)` for a radial effect, porting the C# switch.
pub(crate) fn radial_effect(effect: i32, angle: f64, dist: f64, current_angle: f64) -> (f64, f64) {
    let freq = DOME_RADIAL_FREQUENCY;
    match effect {
        1 => {
            let mut val = map_wrap(dist, current_angle, 1.0 + current_angle, 0.0, 1.0);
            val = wrap(val * freq, 0.0, 1.0);
            val = map_value(val, 0.0, 1.0, -1.0, 1.0).abs();
            let gradient_val = map_value(angle, 0.0, 1.0, -1.0, 1.0).abs();
            (val, gradient_val)
        }
        2 => {
            let mut val = map_wrap(
                angle + dist / freq,
                current_angle,
                1.0 + current_angle,
                0.0,
                1.0,
            );
            val = wrap(val * freq, 0.0, 1.0);
            val = map_value(val, 0.0, 1.0, -1.0, 1.0).abs();
            (val, dist)
        }
        3 => {
            let mut a = map_wrap(angle, current_angle, 1.0 + current_angle, 0.0, 1.0);
            a = wrap(a * freq, 0.0, 1.0);
            a = map_value(a, 0.0, 1.0, -1.0, 1.0).abs();
            ((dist - a).clamp(0.0, 1.0), dist)
        }
        _ => {
            let mut val = map_wrap(angle, current_angle, 1.0 + current_angle, 0.0, 1.0);
            val = wrap(val * freq, 0.0, 1.0);
            val = map_value(val, 0.0, 1.0, -1.0, 1.0).abs();
            (val, dist)
        }
    }
}
#[allow(
    clippy::cast_precision_loss,
    reason = "Runtime visualizer animation periods are small preview-frame counters"
)]
pub(crate) fn runtime_visualizer_progress(input: VisualizerInput, period_frames: u64) -> f64 {
    if input.animation_frame == 0 || period_frames == 0 {
        return 0.0;
    }
    let frame = input.animation_frame % period_frames;
    (input.beat_progress + frame as f64 / period_frames as f64).rem_euclid(1.0)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Runtime visualizer animation uses small preview-frame counters"
)]
pub(crate) fn runtime_visualizer_progress_unwrapped(
    input: VisualizerInput,
    frames_per_cycle: u64,
) -> f64 {
    if input.animation_frame == 0 || frames_per_cycle == 0 {
        return 0.0;
    }
    input.animation_frame as f64 / frames_per_cycle as f64
}

pub(crate) fn spectrum_nudge(random: &mut DotNetRandom, scale: f64) -> f64 {
    (random.next_double() - 0.5) * 2.0 * scale
}

pub(crate) fn paintbrush_frame_in_cycle(input: VisualizerInput) -> u32 {
    input
        .animation_frame
        .min(u64::from(u32::MAX))
        .try_into()
        .expect("paintbrush animation frame fits in u32")
}

pub(crate) fn paintbrush_twinkle(input: VisualizerInput, point_index: usize, z: f64) -> Rgb {
    if input.animation_frame == 0 || z <= 0.2 {
        return Rgb::BLACK;
    }
    let frame_bucket = input.animation_frame / 3;
    let seed = i32::try_from(
        ((frame_bucket.wrapping_mul(1_103_515_245))
            ^ u64::try_from(point_index).expect("point index fits in u64"))
            % i32::MAX as u64,
    )
    .expect("twinkle seed fits in i32");
    let mut random = DotNetRandom::new(seed);
    if random.next_double() < 0.001 {
        Rgb::from_u24(0xff_ff_ff)
    } else {
        Rgb::BLACK
    }
}

pub(crate) fn spectrum_quaternion_test_point(
    normalized_x: f64,
    normalized_y: f64,
) -> (f64, f64, f64) {
    let x = 2.0 * normalized_x - 1.0;
    let y = 1.0 - 2.0 * normalized_y;
    let z = (1.0 - x * x - y * y).sqrt();
    (x, y, z)
}

pub(crate) fn max_axis_by_abs(x: f64, y: f64, z: f64) -> u8 {
    if x.abs() > y.abs() {
        if x.abs() > z.abs() {
            0
        } else {
            2
        }
    } else if y.abs() > z.abs() {
        1
    } else {
        2
    }
}

pub(crate) fn map_value(
    value: f64,
    source_start: f64,
    source_end: f64,
    target_start: f64,
    target_end: f64,
) -> f64 {
    (value - source_start) * (target_end - target_start) / (source_end - source_start)
        + target_start
}

pub(crate) fn map_wrap(
    value: f64,
    source_start: f64,
    source_end: f64,
    target_start: f64,
    target_end: f64,
) -> f64 {
    wrap(
        map_value(value, source_start, source_end, target_start, target_end),
        target_start,
        target_end,
    )
}

pub(crate) fn wrap(mut value: f64, start: f64, end: f64) -> f64 {
    let range = end - start;
    while value < start {
        value += range;
    }
    while value > end {
        value -= range;
    }
    value
}
