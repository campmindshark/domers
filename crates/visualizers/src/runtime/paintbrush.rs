use domers_core::Rgb;
use domers_outputs::DomeCommand;

use crate::{
    buffer::DomeBuffer,
    color_util::hsv_to_rgb,
    geometry::{distance3, hemisphere_point},
    input::VisualizerInput,
    math::{
        paintbrush_twinkle, spectrum_nudge, DOME_GLOBAL_FADE_SPEED, DOME_GLOBAL_HUE_SPEED,
        DOME_RADIAL_SIZE,
    },
    quaternion::Quaternion,
    rng::DotNetRandom,
};

pub(crate) const DOME_RIPPLE_CD_STEP: f64 = 1.0;
pub(crate) const DOME_RIPPLE_STEP: f64 = 1.0;

/// Persistent Paintbrush runtime mirroring `LEDDomeQuaternionPaintbrushVisualizer`.
#[derive(Clone, Debug)]
pub(crate) struct PaintbrushRuntime {
    buffer: DomeBuffer,
    rng: DotNetRandom,
    yaw: f64,
    pitch: f64,
    roll: f64,
    yaw_momentum: f64,
    pitch_momentum: f64,
    roll_momentum: f64,
    counter: u64,
    cooldown: i32,
    stamp_fired: bool,
    stamp_effect: i32,
    last_progress: f64,
    ripple_counter: f64,
    ripple_firing: bool,
    ripple_cooldown: f64,
    ripple_type: i32,
    contour_counter: f64,
    stamp_center: Quaternion,
    ripple_center: Quaternion,
    last_animation_frame: Option<u64>,
}

impl PaintbrushRuntime {
    pub(crate) fn new() -> Self {
        Self {
            buffer: DomeBuffer::new(),
            rng: DotNetRandom::new(0),
            yaw: 0.0,
            pitch: -0.25,
            roll: 0.0,
            yaw_momentum: 0.0,
            pitch_momentum: 0.0005,
            roll_momentum: 0.0,
            counter: 0,
            cooldown: 7,
            stamp_fired: false,
            stamp_effect: 0,
            last_progress: 0.0,
            ripple_counter: 0.0,
            ripple_firing: false,
            ripple_cooldown: 100.0,
            ripple_type: 0,
            contour_counter: 0.0,
            stamp_center: Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            ripple_center: Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            last_animation_frame: None,
        }
    }

    pub(crate) fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        // Spectrum capture and the live preview advance once per 10 ms preview
        // frame (`SIMULATOR_FRAME_STRIDE`), not on every 400 Hz engine tick.
        if self.last_animation_frame == Some(input.animation_frame) {
            out.extend(self.buffer.frame_commands());
            return;
        }
        self.last_animation_frame = Some(input.animation_frame);

        let progress = input.beat_progress;
        let level = f64::from(input.volume.clamp(0.0, 1.0));

        self.buffer
            .fade(1.0 - 5f64.powf(-DOME_GLOBAL_FADE_SPEED), 0.0);
        let hue_rate =
            (3.0 * progress * progress - 3.0 * progress + 1.0) * 10f64.powf(-DOME_GLOBAL_HUE_SPEED);
        self.buffer.hue_rotate(hue_rate);
        self.counter += 1;

        let orientation = self.idle_orientation(input);
        self.update_stamp_and_ripple(level, progress, orientation);
        self.contour_counter += 4.0 * level;
        if self.contour_counter >= 100.0 {
            self.contour_counter = 0.0;
        }

        let threshold_factor = DOME_RADIAL_SIZE / 4.0 + level + 0.01;
        let threshold = 2.0 / threshold_factor;
        let saturation = (1.3 / level.max(0.01) - 1.0).clamp(0.2, 1.0);
        let metaball_hue = (1.0 + orientation.w) / 2.0;

        for (point_index, pixel) in self.buffer.pixels.iter_mut().enumerate() {
            let (x, y, z) = hemisphere_point(pixel.x, pixel.y);
            let (rx, ry, rz) = orientation.transform_vector(x, y, z);
            let distance = distance3(rx, ry, rz, -1.0, 0.0, 0.0);
            let neg_distance = distance3(rx, ry, rz, 1.0, 0.0, 0.0);
            let potential = 1.0 / (distance * neg_distance);
            let strength = potential - threshold;

            let twinkle = paintbrush_twinkle(*input, point_index, z);
            if twinkle != Rgb::BLACK {
                pixel.set_color(twinkle);
            }

            if strength > 0.0 {
                pixel.blend_light_paint(hsv_to_rgb(metaball_hue, saturation, 1.0));
            }

            if self.ripple_firing && self.ripple_counter > 0.0 {
                let (tx, ty, tz) = self.ripple_center.transform_vector(x, y, z);
                let ripple_radius = self.ripple_counter / 300.0;
                let distance_to_spot = distance3(tx, ty, tz, -1.0, 0.0, 0.0);
                if (distance_to_spot - ripple_radius).abs() < 0.012 {
                    let ripple_saturation = (1.0 - self.ripple_counter / 600.0).clamp(0.0, 1.0);
                    let ripple_value = (1.0 - self.ripple_counter / 800.0).clamp(0.0, 1.0);
                    pixel.blend_light_paint(hsv_to_rgb(
                        metaball_hue,
                        ripple_saturation,
                        ripple_value,
                    ));
                }
            }

            if self.stamp_fired {
                let (sx, sy, sz) = self.stamp_center.transform_vector(x, y, z);
                let distance_to_spot = distance3(sx, sy, sz, -1.0, 0.0, 0.0);
                if self.stamp_effect == 1 && distance_to_spot.rem_euclid(0.4) < 0.05 {
                    pixel.set_color(hsv_to_rgb(metaball_hue, 0.2, 1.0));
                } else if self.stamp_effect == 2 {
                    let ring_distance =
                        2.4 - (1.8 / (4.0 - f64::from(self.cooldown) / 2.0)).clamp(0.0, 2.4);
                    let half_width = 0.003 * f64::from(self.cooldown * self.cooldown);
                    if (ring_distance - half_width..=ring_distance + half_width)
                        .contains(&distance_to_spot)
                    {
                        pixel.set_color(hsv_to_rgb(metaball_hue, 0.2, 1.0));
                    }
                }
            }
        }

        if self.cooldown < 7 && self.stamp_effect == 1 {
            self.stamp_fired = false;
        }
        self.last_progress = progress;
        out.extend(self.buffer.frame_commands());
    }

    pub(crate) fn idle_orientation(&mut self, input: &VisualizerInput) -> Quaternion {
        if let Some(orientation) = input.orientation_override {
            return Quaternion::from_yaw_pitch_roll(
                orientation.yaw,
                orientation.pitch,
                orientation.roll,
            );
        }
        if let Some(device) = input.orientation_devices.iter().find_map(|device| *device) {
            return device.rotation;
        }

        let noise = 0.0001;
        self.yaw_momentum =
            (self.yaw_momentum + spectrum_nudge(&mut self.rng, noise)).clamp(-0.001, 0.001);
        self.roll_momentum =
            (self.roll_momentum + spectrum_nudge(&mut self.rng, noise)).clamp(-0.001, 0.001);
        self.pitch_momentum =
            (self.pitch_momentum + spectrum_nudge(&mut self.rng, noise)).clamp(-0.001, 0.001);

        let motion_scale = 4.0 * (f64::from(input.volume.clamp(0.0, 1.0)) + 0.25);
        self.yaw += motion_scale * self.yaw_momentum;
        self.pitch += motion_scale * self.pitch_momentum;
        self.roll += motion_scale * self.roll_momentum;

        Quaternion::from_unitless_yaw_pitch_roll(self.yaw, self.pitch, self.roll)
    }

    pub(crate) fn update_stamp_and_ripple(
        &mut self,
        level: f64,
        progress: f64,
        orientation: Quaternion,
    ) {
        if self.cooldown > 0 && self.last_progress > progress {
            self.cooldown -= 1;
            if self.cooldown <= 0 {
                self.stamp_fired = false;
            }
        }
        if self.counter > 1_000 && level > 0.3 {
            self.stamp_fired = true;
            self.counter = 0;
            self.cooldown = 10;
            let mut effect = self.stamp_effect;
            if effect == 0 {
                effect = 1;
            }
            if effect == 1 {
                effect = 2;
            }
            if effect == 2 {
                effect = 1;
            }
            self.stamp_effect = effect;
            self.stamp_center = orientation;
        }

        if self.ripple_counter > 1_000.0 {
            self.ripple_counter = 0.0;
            self.ripple_firing = false;
        }
        if !self.ripple_firing {
            self.ripple_cooldown -= DOME_RIPPLE_CD_STEP;
        }
        if self.ripple_cooldown < 0.0 {
            self.ripple_firing = true;
            self.ripple_type = (self.ripple_type + 1) % 2;
            self.ripple_center = orientation;
            self.ripple_cooldown = 100.0;
        }
        if self.ripple_firing {
            self.ripple_counter += DOME_RIPPLE_STEP;
            if self.ripple_type == 1 {
                self.ripple_center = orientation;
            }
        }
    }
}
