use domers_core::Rgb;
use domers_outputs::{
    dome_strut_length,
    topology::{DOME_PIXELS, DOME_STRUTS},
    DomeCommand,
};

use crate::{
    color_util::scale_rgb_f64,
    geometry::{build_dome_led_points, DomeLedPoint, DOME_LED_POINTS},
    input::VisualizerInput,
};

/// Spectrum `domeVolumeRotationSpeed` default used by Race rotation math.
pub(crate) const VOLUME_ROTATION_SPEED: f64 = 0.25;
/// Spectrum Race band half-width when `domeRadialSize` is 1.0 (see `LEDDomeRaceVisualizer`).
pub(crate) const RACE_RACER_SPACING: f64 = 1.0;

#[derive(Clone, Copy, Debug)]
pub(crate) enum RaceRotation {
    Constant,
    VolumeSquared,
    Beat,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum RaceColoring {
    Multi,
    FadeExp,
}

#[derive(Clone, Copy, Debug)]
#[allow(
    dead_code,
    reason = "coloring/color_index mirror Spectrum RacerConfig for future wiring"
)]
pub(crate) struct RaceRacerConfig {
    rotation: RaceRotation,
    width: f64,
    coloring: RaceColoring,
    color_index: usize,
}

/// Spectrum `LEDDomeRaceVisualizer.racerConfig`, ground-to-pole order.
pub(crate) const RACE_RACER_CONFIGS: [RaceRacerConfig; 4] = [
    RaceRacerConfig {
        rotation: RaceRotation::VolumeSquared,
        width: 1.0,
        coloring: RaceColoring::Multi,
        color_index: 0,
    },
    RaceRacerConfig {
        rotation: RaceRotation::VolumeSquared,
        width: 0.25,
        coloring: RaceColoring::FadeExp,
        color_index: 1,
    },
    RaceRacerConfig {
        rotation: RaceRotation::Beat,
        width: 0.125,
        coloring: RaceColoring::FadeExp,
        color_index: 2,
    },
    RaceRacerConfig {
        rotation: RaceRotation::Constant,
        width: 1.0,
        coloring: RaceColoring::Multi,
        color_index: 3,
    },
];

#[derive(Clone, Debug)]
#[allow(
    dead_code,
    reason = "radians mirrors Spectrum Racer width; rendering uses shared RACER_WIDTHS table"
)]
pub(crate) struct RaceRacer {
    pub(crate) angle: f64,
    radians: f64,
    accumulated_seconds: f64,
    config: RaceRacerConfig,
}

impl RaceRacer {
    pub(crate) fn new(config: RaceRacerConfig) -> Self {
        Self {
            angle: 0.0,
            radians: std::f64::consts::TAU * config.width,
            accumulated_seconds: 0.0,
            config,
        }
    }

    pub(crate) fn revs_per_second(&self, volume: f64, measure_length_ms: Option<u32>) -> f64 {
        match self.config.rotation {
            RaceRotation::VolumeSquared => volume.mul_add(volume, VOLUME_ROTATION_SPEED / 12.0),
            RaceRotation::Beat => {
                let beats_per_second = match measure_length_ms {
                    Some(measure) if measure > 0 => 1000.0 / f64::from(measure),
                    _ => 1.0,
                };
                beats_per_second / 4.0
            }
            RaceRotation::Constant => VOLUME_ROTATION_SPEED / 4.0,
        }
    }

    pub(crate) fn move_racer(
        &mut self,
        num_seconds: f64,
        volume: f64,
        measure_length_ms: Option<u32>,
    ) {
        let rads_per_second =
            std::f64::consts::TAU * self.revs_per_second(volume, measure_length_ms);
        let rads = (num_seconds + self.accumulated_seconds) * rads_per_second;
        if rads < 0.0001 {
            // Too small to move at f64 precision; bank the time for a later step.
            self.accumulated_seconds += num_seconds;
            return;
        }
        self.angle += rads;
        if self.angle > std::f64::consts::PI {
            self.angle -= std::f64::consts::TAU;
        }
        self.accumulated_seconds = 0.0;
    }
}

/// Persistent Race runtime porting `LEDDomeRaceVisualizer`'s wall-clock racers.
pub(crate) fn race_commands(input: VisualizerInput) -> Vec<DomeCommand> {
    let points = DOME_LED_POINTS.get_or_init(build_dome_led_points);
    let mut commands = Vec::with_capacity(DOME_PIXELS + 1);
    let mut point_index = 0;
    for strut_index in 0..DOME_STRUTS {
        let Some(strut_length) = dome_strut_length(strut_index) else {
            continue;
        };
        for led_index in 0..strut_length {
            let point = points.get(point_index).copied().unwrap_or(DomeLedPoint {
                index: point_index,
                x: 0.5,
                y: 0.5,
            });
            point_index += 1;
            commands.push(DomeCommand::Pixel {
                strut_index,
                led_index,
                color: race_pixel_color(input, point.x, point.y, None),
            });
        }
    }
    commands.push(DomeCommand::Flush);
    commands
}

pub(crate) fn race_pixel_color(
    input: VisualizerInput,
    projected_x: f64,
    projected_y: f64,
    start_angles: Option<[f64; 4]>,
) -> Rgb {
    let px = projected_x * 2.0 - 1.0;
    let py = projected_y * 2.0 - 1.0;
    let y = 1.0 - (px * px + py * py).sqrt();
    let angle = py.atan2(px);
    let Some((racer_index, loc_ang)) = race_location(input, y, angle, start_angles) else {
        return Rgb::BLACK;
    };
    match racer_index {
        0 | 3 => race_multi_color(input, loc_ang),
        1 => scale_rgb_f64(input.palette[1], 1.0 / (1.0 + (4.0 - 4.0 * loc_ang).exp())),
        2 => scale_rgb_f64(input.palette[2], 1.0 / (1.0 + (4.0 - 4.0 * loc_ang).exp())),
        _ => Rgb::BLACK,
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "Spectrum truncates floating racer positions to integer band indexes"
)]
pub(crate) fn race_location(
    input: VisualizerInput,
    mut y: f64,
    mut angle: f64,
    start_angles: Option<[f64; 4]>,
) -> Option<(usize, f64)> {
    pub(crate) const RACER_COUNT: f64 = 4.0;
    pub(crate) const RACER_WIDTHS: [f64; 4] = [1.0, 0.25, 0.125, 1.0];
    if y > 0.9999 {
        y = 0.9999;
    }
    let racer_loc_y = y * RACER_COUNT;
    let racer_index = usize::try_from(racer_loc_y as isize).ok()?;
    let local_y = (racer_loc_y - racer_index as f64 - 0.5).abs();
    if local_y > RACE_RACER_SPACING {
        return None;
    }
    if racer_index >= RACER_WIDTHS.len() {
        return None;
    }
    if angle < 0.0 {
        angle += std::f64::consts::PI * 2.0;
    }
    let start_angle = match start_angles {
        Some(angles) => angles[racer_index].rem_euclid(std::f64::consts::TAU),
        None => race_start_angle(input, racer_index),
    };
    let mut offset = angle - start_angle;
    if offset < 0.0 {
        offset += std::f64::consts::PI * 2.0;
    }
    let radians = std::f64::consts::PI * 2.0 * RACER_WIDTHS[racer_index];
    if offset < std::f64::consts::PI * 2.0 - radians {
        return None;
    }
    Some((
        racer_index,
        1.0 - (std::f64::consts::PI * 2.0 - offset) / radians,
    ))
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Runtime preview frame counter is small and used as elapsed Spectrum seconds"
)]
pub(crate) fn race_start_angle(input: VisualizerInput, racer_index: usize) -> f64 {
    if input.animation_frame == 0 {
        return 0.0;
    }
    let seconds = input.animation_frame as f64 / 100.0;
    let volume = f64::from(input.volume.clamp(0.0, 1.0));
    let revs_per_second = match racer_index {
        0 | 1 => volume.mul_add(volume, VOLUME_ROTATION_SPEED / 12.0),
        2 => 0.25,
        3 => VOLUME_ROTATION_SPEED / 4.0,
        _ => 0.0,
    };
    (seconds * revs_per_second * std::f64::consts::TAU).rem_euclid(std::f64::consts::TAU)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "Spectrum truncates palette segment selection from normalized racer location"
)]
pub(crate) fn race_multi_color(input: VisualizerInput, loc_ang: f64) -> Rgb {
    let scaled = loc_ang.clamp(0.0, 1.0) * 4.0;
    let min_color_index = (scaled as usize).min(4);
    let max_color_index = min_color_index + 1;
    let scaled_pixel_pos = (loc_ang - min_color_index as f64 / 4.0) * 4.0;
    blend_spectrum_rgb(
        input.palette[min_color_index],
        input.palette[max_color_index],
        scaled_pixel_pos,
    )
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Spectrum LEDColor.GradientColor truncates blended double channels to bytes"
)]
pub(crate) fn blend_spectrum_rgb(color1: Rgb, color2: Rgb, distance: f64) -> Rgb {
    let distance = distance.clamp(0.0, 1.0);
    let inverse = 1.0 - distance;
    Rgb {
        r: (distance * f64::from(color1.r) + inverse * f64::from(color2.r)) as u8,
        g: (distance * f64::from(color1.g) + inverse * f64::from(color2.g)) as u8,
        b: (distance * f64::from(color1.b) + inverse * f64::from(color2.b)) as u8,
    }
}
