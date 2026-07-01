use domers_core::Rgb;
use domers_outputs::topology::DOME_PIXELS;

use crate::{
    color_util::light_paint,
    geometry::{build_dome_led_points, distance2, DOME_LED_POINTS},
    input::VisualizerInput,
    math::runtime_visualizer_progress,
};

pub(crate) fn splat_frame(input: VisualizerInput) -> Vec<Rgb> {
    if input.animation_frame == 0 {
        return vec![Rgb::BLACK; DOME_PIXELS];
    }

    let adjusted_level = f64::from(input.volume.clamp(0.0, 1.0))
        .sqrt()
        .clamp(0.1, 1.0);
    let points = DOME_LED_POINTS.get_or_init(build_dome_led_points);
    let splats = [
        Splat {
            center_x: 0.22,
            center_y: 0.34,
            phase_offset: 0.00,
            color_index: 0,
        },
        Splat {
            center_x: 0.74,
            center_y: 0.42,
            phase_offset: 0.31,
            color_index: 3,
        },
        Splat {
            center_x: 0.46,
            center_y: 0.76,
            phase_offset: 0.63,
            color_index: 6,
        },
    ];

    let progress = runtime_visualizer_progress(input, 240);
    points
        .iter()
        .map(|point| {
            let mut color = Rgb::BLACK;
            for splat in splats {
                let age = (progress + splat.phase_offset).rem_euclid(1.0);
                let radius = adjusted_level * (0.06 + 0.24 * age);
                let distance = distance2(point.x, point.y, splat.center_x, splat.center_y);
                if distance < radius {
                    let falloff = 1.0 - distance / radius;
                    let fade = (1.0 - age).powi(2);
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "Splat preview clamps normalized brightness before RGB scaling"
                    )]
                    let strength = (falloff * fade).clamp(0.0, 1.0) as f32;
                    color = light_paint(
                        color,
                        input.palette[splat.color_index % input.palette.len()].scale(strength),
                    );
                }
            }
            color
        })
        .collect()
}

#[derive(Clone, Copy)]
pub(crate) struct Splat {
    center_x: f64,
    center_y: f64,
    phase_offset: f64,
    color_index: usize,
}
