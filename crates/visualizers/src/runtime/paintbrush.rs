use domers_core::Rgb;
use domers_outputs::DomeCommand;

use crate::{
    buffer::DomeBuffer,
    color_util::hsv_to_rgb,
    geometry::{distance3, hemisphere_point},
    input::{OrientationDeviceInput, VisualizerInput},
    math::{spectrum_nudge, DOME_GLOBAL_FADE_SPEED, DOME_GLOBAL_HUE_SPEED, DOME_RADIAL_SIZE},
    quaternion::Quaternion,
    rng::DotNetRandom,
};

pub(crate) const DOME_RIPPLE_CD_STEP: f64 = 1.0;
pub(crate) const DOME_RIPPLE_STEP: f64 = 1.0;
const IDLE_TIME: i32 = 1_000;
const SPOT_X: f64 = -1.0;
const SPOT_Y: f64 = 0.0;
const SPOT_Z: f64 = 0.0;
const SPOTLIGHT_DISABLE_ALL_WANDS: i32 = -2;
const POI_MIN_SCALE: f64 = 0.5;
const POI_MAX_SCALE: f64 = 5.0;
const RIPPLE_CLOSE_TOLERANCE: f64 = 0.01;

/// Persistent Paintbrush runtime mirroring `LEDDomeQuaternionPaintbrushVisualizer`.
#[derive(Clone, Debug)]
pub(crate) struct PaintbrushRuntime {
    buffer: DomeBuffer,
    rng: DotNetRandom,
    current_orientation: Quaternion,
    last_orientation: Quaternion,
    idle_timer: i32,
    idle: bool,
    yaw: f64,
    pitch: f64,
    roll: f64,
    yaw_momentum: f64,
    pitch_momentum: f64,
    roll_momentum: f64,
    spotlight_id: i32,
    spotlight_center: Quaternion,
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
            current_orientation: identity(),
            last_orientation: identity(),
            idle_timer: 0,
            idle: false,
            yaw: 0.0,
            pitch: -0.25,
            roll: 0.0,
            yaw_momentum: 0.0,
            pitch_momentum: 0.0005,
            roll_momentum: 0.0,
            spotlight_id: -1,
            spotlight_center: identity(),
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
            stamp_center: identity(),
            ripple_center: identity(),
            last_animation_frame: None,
        }
    }

    pub(crate) fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
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

        let devices = live_devices(input);
        self.update_idle_state(input, &devices);
        self.update_stamp_and_ripple(level, progress);
        self.contour_counter += 4.0 * level;
        if self.contour_counter >= 100.0 {
            self.contour_counter = 0.0;
        }

        let threshold_factor = (DOME_RADIAL_SIZE / 4.0) + level + 0.01;
        let threshold = 2.0 / threshold_factor;
        let stamp_hue = (1.0 + self.current_orientation.w) / 2.0;
        let paint_state = PaintbrushPaintState {
            idle: self.idle,
            current_orientation: self.current_orientation,
            only_poi: input.orientation_only_poi,
        };

        for (point_index, pixel) in self.buffer.pixels.iter_mut().enumerate() {
            let (x, y, z) = hemisphere_point(pixel.x, pixel.y);

            if self.rng.next_double() < input.dome_twinkle_density && z > 0.2 {
                pixel.set_color(Rgb::from_u24(0xff_ff_ff));
            }

            let (potential, metaball_hue, _orientation_w) =
                paintbrush_metaball_at(paint_state, (x, y, z), &devices);
            let strength = potential - threshold;

            if strength > 0.0 {
                let saturation = (1.3 / level.max(0.01) - 1.0).clamp(0.2, 1.0);
                pixel.blend_light_paint(hsv_to_rgb(metaball_hue, saturation, 1.0));
            }

            if input.orientation_show_contours {
                if let Some((hue, saturation, value)) =
                    paintbrush_contour_color(potential, self.contour_counter, metaball_hue)
                {
                    pixel.blend_light_paint(hsv_to_rgb(hue, saturation, value));
                }
            }

            if self.ripple_firing && self.ripple_counter > 0.0 {
                let (tx, ty, tz) = self.ripple_center.transform_vector(x, y, z);
                let ripple_radius = self.ripple_counter / 300.0;
                let distance_to_spot = distance3(tx, ty, tz, SPOT_X, SPOT_Y, SPOT_Z);
                if (distance_to_spot - ripple_radius).abs() < RIPPLE_CLOSE_TOLERANCE {
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
                let distance_to_spot = distance3(sx, sy, sz, SPOT_X, SPOT_Y, SPOT_Z);
                if self.stamp_effect == 1 && distance_to_spot.rem_euclid(0.4) < 0.05 {
                    pixel.set_color(hsv_to_rgb(stamp_hue, 0.2, 1.0));
                } else if self.stamp_effect == 2 {
                    let ring_distance =
                        2.4 - (1.8 / (4.0 - f64::from(self.cooldown) / 2.0)).clamp(0.0, 2.4);
                    let half_width = 0.003 * f64::from(self.cooldown * self.cooldown);
                    if (ring_distance - half_width..=ring_distance + half_width)
                        .contains(&distance_to_spot)
                    {
                        pixel.set_color(hsv_to_rgb(stamp_hue, 0.2, 1.0));
                    }
                }
            }

            let _ = point_index;
        }

        if self.cooldown < 7 && self.stamp_effect == 1 {
            self.stamp_fired = false;
        }
        self.last_progress = progress;
        out.extend(self.buffer.frame_commands());
    }

    fn update_idle_state(&mut self, input: &VisualizerInput, devices: &[OrientationDeviceInput]) {
        if input.orientation_device_spotlight == SPOTLIGHT_DISABLE_ALL_WANDS {
            self.idle = true;
        }

        if let Some(orientation) = input.orientation_override {
            self.current_orientation = Quaternion::from_yaw_pitch_roll(
                orientation.yaw,
                orientation.pitch,
                orientation.roll,
            );
            self.idle = false;
            self.spotlight_id = -1;
            return;
        }

        let level = f64::from(input.volume.clamp(0.0, 1.0));
        let spotlight_id = input.orientation_device_spotlight;

        if devices
            .iter()
            .any(|device| i32::from(device.device_id) == spotlight_id)
        {
            let spotlight = devices
                .iter()
                .find(|device| i32::from(device.device_id) == spotlight_id)
                .expect("spotlight device exists");
            self.spotlight_id = spotlight_id;
            self.spotlight_center = spotlight.rotation;
        }

        if devices.is_empty() {
            self.idle = true;
        } else if devices.len() == 1 {
            let device = devices[0];
            self.current_orientation = device.rotation;
            let diff = (1.0 - quaternion_dot(self.last_orientation, device.rotation)).abs();
            if diff < 0.0001 || is_zero_quaternion(device.rotation) {
                if self.idle_timer > 0 {
                    self.idle_timer -= 1;
                }
            } else {
                self.idle = false;
                self.idle_timer = IDLE_TIME;
            }
            self.last_orientation = device.rotation;
            if self.idle_timer <= 0 {
                self.idle = true;
            }
        } else {
            self.idle = false;
        }

        if self.idle {
            let noise = 0.0001;
            self.yaw_momentum =
                (self.yaw_momentum + spectrum_nudge(&mut self.rng, noise)).clamp(-0.001, 0.001);
            self.roll_momentum =
                (self.roll_momentum + spectrum_nudge(&mut self.rng, noise)).clamp(-0.001, 0.001);
            self.pitch_momentum =
                (self.pitch_momentum + spectrum_nudge(&mut self.rng, noise)).clamp(-0.001, 0.001);

            let motion_scale = 4.0 * (level + 0.25);
            self.yaw += motion_scale * self.yaw_momentum;
            self.pitch += motion_scale * self.pitch_momentum;
            self.roll += motion_scale * self.roll_momentum;
            self.current_orientation =
                Quaternion::from_unitless_yaw_pitch_roll(self.yaw, self.pitch, self.roll);
            self.spotlight_id = -1;
        } else {
            for device in devices {
                if !devices
                    .iter()
                    .any(|candidate| i32::from(candidate.device_id) == spotlight_id)
                {
                    self.spotlight_id = i32::from(device.device_id);
                    self.spotlight_center = device.rotation;
                    break;
                }
            }
        }
    }

    fn update_stamp_and_ripple(&mut self, level: f64, progress: f64) {
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
            self.stamp_center = if self.spotlight_id == -1 {
                self.current_orientation
            } else {
                self.spotlight_center
            };
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
            self.ripple_center = if self.spotlight_id == -1 {
                self.current_orientation
            } else {
                self.spotlight_center
            };
            self.ripple_cooldown = 100.0;
        }
        if self.ripple_firing {
            self.ripple_counter += DOME_RIPPLE_STEP;
            if self.ripple_type == 1 {
                self.ripple_center = if self.spotlight_id == -1 {
                    self.current_orientation
                } else {
                    self.spotlight_center
                };
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PaintbrushPaintState {
    idle: bool,
    current_orientation: Quaternion,
    only_poi: bool,
}

fn paintbrush_metaball_at(
    state: PaintbrushPaintState,
    pixel: (f64, f64, f64),
    devices: &[OrientationDeviceInput],
) -> (f64, f64, f64) {
    let (x, y, z) = pixel;
    if state.idle {
        let (rx, ry, rz) = state.current_orientation.transform_vector(x, y, z);
        let distance = distance3(rx, ry, rz, SPOT_X, SPOT_Y, SPOT_Z);
        let neg_distance = distance3(rx, ry, rz, -SPOT_X, -SPOT_Y, -SPOT_Z);
        let potential = 1.0 / (distance * neg_distance);
        let hue = (1.0 + state.current_orientation.w) / 2.0;
        return (potential, hue, state.current_orientation.w);
    }

    let device_count = devices.len().max(1);
    let mut potential = 0.0;
    let mut color_center = Quaternion {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };

    for device in devices {
        let orientation = device.rotation;
        let (rx, ry, rz) = orientation.transform_vector(x, y, z);
        let distance = distance3(rx, ry, rz, SPOT_X, SPOT_Y, SPOT_Z);
        let neg_distance = distance3(rx, ry, rz, -SPOT_X, -SPOT_Y, -SPOT_Z);
        let mut scale = 1.0 / (distance * neg_distance);
        if (1..=3).contains(&device.action_flag) {
            scale *= 4.0;
        }
        if device.device_type == 2 && state.only_poi {
            scale = scale * (device.avg_distance_short * (POI_MAX_SCALE - POI_MIN_SCALE))
                + POI_MIN_SCALE;
        }

        if distance < neg_distance {
            color_center = quaternion_add_scaled(color_center, orientation, scale);
        } else {
            color_center = quaternion_add_scaled(color_center, orientation, -scale);
        }
        potential += scale;
    }

    color_center = color_center.normalize();
    potential /= f64::from(u32::try_from(device_count).expect("device count fits in u32"));
    let hue = (1.0 + color_center.w) / 2.0;
    (potential, hue, color_center.w)
}

fn identity() -> Quaternion {
    Quaternion {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    }
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

fn paintbrush_contour_color(
    potential: f64,
    contour_counter: f64,
    metaball_hue: f64,
) -> Option<(f64, f64, f64)> {
    let potential_contours = (1000.0 * (potential - 0.5)).ln() + contour_counter / 100.0;
    let contour_bracket = potential_contours.trunc();
    let contour_value = potential_contours - contour_bracket;
    if contour_value >= 0.2 {
        return None;
    }
    let value = 0.8 - (1.0 - (contour_bracket / 10.0).clamp(0.0, 0.8));
    Some((metaball_hue, 0.4, value))
}

fn quaternion_dot(left: Quaternion, right: Quaternion) -> f64 {
    left.x * right.x + left.y * right.y + left.z * right.z + left.w * right.w
}

fn is_zero_quaternion(quaternion: Quaternion) -> bool {
    quaternion.w == 0.0 && quaternion.x == 0.0 && quaternion.y == 0.0 && quaternion.z == 0.0
}

fn quaternion_add_scaled(mut base: Quaternion, delta: Quaternion, scale: f64) -> Quaternion {
    base.x += delta.x * scale;
    base.y += delta.y * scale;
    base.z += delta.z * scale;
    base.w += delta.w * scale;
    base
}
