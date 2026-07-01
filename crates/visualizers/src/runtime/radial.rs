use domers_outputs::DomeCommand;

use crate::{
    buffer::DomeBuffer,
    dome::{VOLUME_GRADIENT_SPEED, VOLUME_ROTATION_SPEED},
    input::VisualizerInput,
    math::{
        map_wrap, polar_to_cartesian, radial_effect, wrap, DOME_GLOBAL_FADE_SPEED,
        DOME_GLOBAL_HUE_SPEED, DOME_RADIAL_CENTER_ANGLE, DOME_RADIAL_CENTER_DISTANCE,
        DOME_RADIAL_CENTER_SPEED, DOME_RADIAL_EFFECT, DOME_RADIAL_SIZE,
    },
};

#[derive(Clone, Debug)]
pub(crate) struct RadialRuntime {
    buffer: DomeBuffer,
    current_angle: f64,
    current_gradient: f64,
    current_center_angle: f64,
    last_progress: f64,
}

impl RadialRuntime {
    pub(crate) fn new() -> Self {
        Self {
            buffer: DomeBuffer::new(),
            current_angle: 0.0,
            current_gradient: 0.0,
            current_center_angle: 0.0,
            last_progress: 0.0,
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Spectrum picks the radial gradient by truncating normalized volume times 8"
    )]
    pub(crate) fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        self.buffer
            .fade(1.0 - 10f64.powf(-DOME_GLOBAL_FADE_SPEED), 0.0);
        self.buffer.hue_rotate(10f64.powf(-DOME_GLOBAL_HUE_SPEED));

        let level = f64::from(input.volume.clamp(0.0, 1.0));
        let adjusted_level = level.sqrt().clamp(0.1, 1.0);
        let progress = input.beat_progress;
        let delta = wrap(progress - self.last_progress, 0.0, 1.0);
        self.current_angle = wrap(
            self.current_angle + VOLUME_ROTATION_SPEED * delta * 0.25,
            0.0,
            1.0,
        );
        self.current_gradient = wrap(
            self.current_gradient + VOLUME_GRADIENT_SPEED * delta,
            0.0,
            1.0,
        );
        self.current_center_angle = wrap(
            self.current_center_angle + DOME_RADIAL_CENTER_SPEED * delta * 0.25,
            0.0,
            1.0,
        );
        self.last_progress = progress;

        let center = polar_to_cartesian(
            DOME_RADIAL_CENTER_ANGLE + self.current_center_angle * std::f64::consts::TAU,
            DOME_RADIAL_CENTER_DISTANCE,
        );
        let which_gradient = ((level * 8.0) as usize).min(7);
        let size_limit = DOME_RADIAL_SIZE * adjusted_level;

        for pixel in &mut self.buffer.pixels {
            let px = (pixel.x + center.0) * 2.0 - 1.0;
            let py = (pixel.y + center.1) * 2.0 - 1.0;
            let angle = map_wrap(
                py.atan2(px),
                -std::f64::consts::PI,
                std::f64::consts::PI,
                0.0,
                1.0,
            );
            let dist = (px * px + py * py).sqrt();
            let (val, gradient_val) =
                radial_effect(DOME_RADIAL_EFFECT, angle, dist, self.current_angle);
            if val <= size_limit {
                let color = input.palette_entries[which_gradient].gradient_color(
                    gradient_val,
                    self.current_gradient,
                    true,
                );
                pixel.set_color(color);
            }
        }

        out.extend(self.buffer.frame_commands());
    }
}
