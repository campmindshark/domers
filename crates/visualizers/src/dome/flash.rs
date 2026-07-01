use domers_core::Rgb;
use domers_outputs::{dome_strut_length, DomeCommand};

use crate::{color_util::scale_rgb_f64, input::VisualizerInput};

use super::volume::{push_unique_usize, volume_gradient_pos, VolumeStrutLayout};

#[derive(Clone, Debug)]
pub(crate) struct FlashShape {
    pub(crate) layout: VolumeStrutLayout,
    pub(crate) struts: Vec<usize>,
    pub(crate) animation: Option<FlashPolygonAnimation>,
}

impl FlashShape {
    pub(crate) const ENABLED: bool = true;

    pub(crate) fn enabled() -> bool {
        Self::ENABLED
    }

    pub(crate) fn available(&self) -> bool {
        Self::enabled() && self.animation.is_none()
    }
}

/// Persistent Flash polygon animation mirroring `PolygonAnimation`.
#[derive(Clone, Debug)]
pub(crate) struct FlashPolygonAnimation {
    pub(crate) pad: u8,
    velocity: f64,
    animation_length: u64,
    starting_time: u64,
    peak_time: u64,
    end_time: u64,
    released: bool,
}

impl FlashPolygonAnimation {
    pub(crate) fn new(pad: u8, velocity: f64, measure_length_ms: u32, now_ms: u64) -> Self {
        let animation_length = u64::from(measure_length_ms) / 4;
        let starting_time = now_ms;
        let peak_time = starting_time + (animation_length * 8 / 10);
        let end_time = starting_time + animation_length;
        Self {
            pad,
            velocity,
            animation_length,
            starting_time,
            peak_time,
            end_time,
            released: false,
        }
    }

    pub(crate) fn active(&self, now_ms: u64, shape_enabled: bool) -> bool {
        shape_enabled && (!self.released || self.end_time > now_ms)
    }

    pub(crate) fn release(&mut self, now_ms: u64) {
        if self.released {
            return;
        }
        self.released = true;
        if now_ms > self.peak_time {
            self.end_time = now_ms + self.animation_length * 2 / 10;
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "Flash intensity mirrors Spectrum's millisecond timestamp ratios"
    )]
    pub(crate) fn intensity(&self, now_ms: u64) -> f64 {
        if now_ms < self.peak_time {
            (now_ms.saturating_sub(self.starting_time)) as f64
                / (self.peak_time.saturating_sub(self.starting_time)) as f64
        } else if !self.released {
            1.0
        } else if now_ms >= self.end_time {
            0.0
        } else {
            1.0 - (now_ms.saturating_sub(self.peak_time)) as f64
                / (self.end_time.saturating_sub(self.peak_time)) as f64
        }
    }
}

pub(crate) fn flash_layout_struts(layout: &VolumeStrutLayout) -> Vec<usize> {
    let mut struts = Vec::new();
    for segment in &layout.segments {
        for strut in &segment.struts {
            push_unique_usize(&mut struts, strut.index);
        }
    }
    struts
}

pub(crate) fn clear_flash_strut(strut_index: usize, out: &mut Vec<DomeCommand>) {
    let Some(length) = dome_strut_length(strut_index) else {
        return;
    };
    for led_index in 0..length {
        out.push(DomeCommand::Pixel {
            strut_index,
            led_index,
            color: Rgb::BLACK,
        });
    }
}

#[allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    reason = "Flash polygon animation mirrors Spectrum's exact layout ratios and filled-section checks"
)]
pub(crate) fn animate_flash_polygon(
    shape: &FlashShape,
    animation: &FlashPolygonAnimation,
    input: &VisualizerInput,
    now_ms: u64,
    out: &mut Vec<DomeCommand>,
) {
    let intensity = animation.intensity(now_ms);
    let scale_color = (intensity * 2.0 * animation.velocity).min(1.0);
    let total_parts = shape.layout.segments.len();
    let animation_split_into = 2 * ((total_parts - 1) / 2 + 1);

    for part in (0..total_parts).step_by(2) {
        let start_range = part as f64 / animation_split_into as f64;
        let end_range = (part + 2) as f64 / animation_split_into as f64;
        let scaled = if (end_range - start_range).abs() < f64::EPSILON {
            0.0
        } else {
            ((intensity - start_range) / (end_range - start_range)).clamp(0.0, 1.0)
        };
        let start_lit_range = if intensity == 0.0 {
            1.0
        } else {
            (start_range / intensity).min(1.0)
        };
        let end_lit_range = if intensity == 0.0 {
            1.0
        } else {
            (end_range / intensity).min(1.0)
        };

        let spoke_segment = &shape.layout.segments[part];
        for strut in &spoke_segment.struts {
            let Some(length) = dome_strut_length(strut.index) else {
                continue;
            };
            for led_index in 0..length {
                let color = volume_gradient_pos(
                    *strut,
                    length,
                    scaled,
                    start_lit_range,
                    end_lit_range,
                    led_index,
                )
                .map_or(Rgb::BLACK, |gradient_pos| {
                    scale_rgb_f64(
                        flash_pad_gradient_color(input, animation.pad, gradient_pos),
                        scale_color,
                    )
                });
                out.push(DomeCommand::Pixel {
                    strut_index: strut.index,
                    led_index,
                    color,
                });
            }
        }

        if part + 1 == total_parts {
            break;
        }

        let circle_color = if scaled == 1.0 {
            scale_rgb_f64(flash_pad_single_color(input, animation.pad), scale_color)
        } else {
            Rgb::BLACK
        };
        let circle_segment = &shape.layout.segments[part + 1];
        for strut in &circle_segment.struts {
            let Some(length) = dome_strut_length(strut.index) else {
                continue;
            };
            for led_index in 0..length {
                out.push(DomeCommand::Pixel {
                    strut_index: strut.index,
                    led_index,
                    color: circle_color,
                });
            }
        }
    }
}

pub(crate) fn flash_pad_single_color(input: &VisualizerInput, pad: u8) -> Rgb {
    scale_rgb_f64(
        input.palette_entries[pad as usize % input.palette_entries.len()].single_color(),
        input.dome_brightness,
    )
}

pub(crate) fn flash_pad_gradient_color(input: &VisualizerInput, pad: u8, pixel_pos: f64) -> Rgb {
    scale_rgb_f64(
        input.palette_entries[pad as usize % input.palette_entries.len()]
            .gradient_color(pixel_pos, 0.0, false),
        input.dome_brightness,
    )
}
