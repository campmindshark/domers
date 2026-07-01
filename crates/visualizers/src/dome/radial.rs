use domers_core::Rgb;

use crate::{
    dome::{VOLUME_GRADIENT_SPEED, VOLUME_ROTATION_SPEED},
    geometry::{build_dome_led_points, DOME_LED_POINTS},
    input::VisualizerInput,
    math::{map_wrap, runtime_visualizer_progress_unwrapped, wrap},
};

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Spectrum chooses the radial gradient by truncating normalized volume times 8"
)]
pub(crate) fn radial_frame(input: VisualizerInput) -> Vec<Rgb> {
    let adjusted_level = f64::from(input.volume.clamp(0.0, 1.0))
        .sqrt()
        .clamp(0.1, 1.0);
    let progress = if input.animation_frame == 0 {
        0.0
    } else {
        runtime_visualizer_progress_unwrapped(input, 200)
    };
    let current_angle = wrap(progress * VOLUME_ROTATION_SPEED * 0.25, 0.0, 1.0);
    let current_gradient = wrap(progress * VOLUME_GRADIENT_SPEED, 0.0, 1.0);
    let which_gradient = (f64::from(input.volume.clamp(0.0, 1.0)) * 8.0) as usize;
    let size_limit = adjusted_level;

    DOME_LED_POINTS
        .get_or_init(build_dome_led_points)
        .iter()
        .map(|point| {
            let px = point.x * 2.0 - 1.0;
            let py = point.y * 2.0 - 1.0;
            let angle = map_wrap(
                py.atan2(px),
                -std::f64::consts::PI,
                std::f64::consts::PI,
                0.0,
                1.0,
            );
            let dist = (px * px + py * py).sqrt();
            let mut val = map_wrap(angle, current_angle, 1.0 + current_angle, 0.0, 1.0);
            val = wrap(val, 0.0, 1.0);
            val = (val * 2.0 - 1.0).abs();
            if val <= size_limit {
                input.palette_entries[which_gradient % input.palette_entries.len()].gradient_color(
                    dist,
                    current_gradient,
                    true,
                )
            } else {
                Rgb::BLACK
            }
        })
        .collect()
}
