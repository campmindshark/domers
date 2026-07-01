use domers_core::Rgb;

use crate::{
    color_util::light_paint,
    geometry::{build_dome_led_points, DOME_LED_POINTS},
};
use domers_outputs::DomeCommand;

#[derive(Clone, Copy, Debug)]
pub(crate) struct DomeBufferPixel {
    pub(crate) x: f64,
    pub(crate) y: f64,
    color: u32,
    r: f64,
    g: f64,
    b: f64,
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Spectrum ClampByte truncates the fractional channel to a byte on pack"
)]
pub(crate) fn clamp_byte(value: f64) -> u32 {
    if value <= 0.0 {
        0
    } else if value >= 255.0 {
        255
    } else {
        value as u32
    }
}

impl DomeBufferPixel {
    pub(crate) fn set_color(&mut self, color: Rgb) {
        let packed = color.to_u24();
        self.color = packed;
        self.r = f64::from((packed >> 16) & 0xff);
        self.g = f64::from((packed >> 8) & 0xff);
        self.b = f64::from(packed & 0xff);
    }

    pub(crate) fn blend_light_paint(&mut self, paint: Rgb) {
        self.set_color(light_paint(self.rgb(), paint));
    }

    pub(crate) fn update_color(&mut self) {
        self.color = (clamp_byte(self.r) << 16) | (clamp_byte(self.g) << 8) | clamp_byte(self.b);
    }

    pub(crate) fn fade(&mut self, mul: f64, sub: f64) {
        if self.color == 0 {
            return;
        }
        self.r = self.r.mul_add(mul, -sub);
        self.g = self.g.mul_add(mul, -sub);
        self.b = self.b.mul_add(mul, -sub);
        self.update_color();
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_possible_wrap,
        clippy::many_single_char_names,
        reason = "Ported bit-for-bit from Spectrum LEDDomeOutputPixel.HueRotate"
    )]
    pub(crate) fn hue_rotate(&mut self, rate: f64) {
        if self.color == 0 {
            return;
        }
        let r = self.r / 255.0;
        let g = self.g / 255.0;
        let b = self.b / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let d = max - min;
        let s = if max == 0.0 { 0.0 } else { d / max };
        if s == 0.0 {
            return;
        }
        let v = max;
        let mut h = 0.0;
        if (max - min).abs() > f64::EPSILON {
            if r > g {
                if r > b {
                    h = (g - b) / d + if g < b { 6.0 } else { 0.0 };
                } else {
                    h = (r - g) / d + 4.0;
                }
            } else if g > b {
                h = (b - r) / d + 2.0;
            } else {
                h = (r - g) / d + 4.0;
            }
            h /= 6.0;
        }
        let mut shifted_hue = (h + rate) % 1.0;
        if shifted_hue > 1.0 {
            shifted_hue -= 1.0;
        }
        if shifted_hue < 0.0 {
            shifted_hue += 1.0;
        }
        let j = (shifted_hue * 6.0).floor() as i64;
        let f = shifted_hue * 6.0 - j as f64;
        let p = v * (1.0 - s);
        let q = v * (1.0 - f * s);
        let t = v * (1.0 - (1.0 - f) * s);
        let (nr, ng, nb) = match j.rem_euclid(6) {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        self.r = nr * 255.0;
        self.g = ng * 255.0;
        self.b = nb * 255.0;
        self.update_color();
    }

    pub(crate) fn rgb(&self) -> Rgb {
        Rgb::from_u24(self.color)
    }
}

/// Persistent full-dome buffer mirroring `LEDDomeOutputBuffer`.
#[derive(Clone, Debug)]
pub(crate) struct DomeBuffer {
    pub(crate) pixels: Vec<DomeBufferPixel>,
}

impl DomeBuffer {
    pub(crate) fn new() -> Self {
        let points = DOME_LED_POINTS.get_or_init(build_dome_led_points);
        Self {
            pixels: points
                .iter()
                .map(|point| DomeBufferPixel {
                    x: point.x,
                    y: point.y,
                    color: 0,
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                })
                .collect(),
        }
    }

    pub(crate) fn fade(&mut self, mul: f64, sub: f64) {
        for pixel in &mut self.pixels {
            pixel.fade(mul, sub);
        }
    }

    pub(crate) fn hue_rotate(&mut self, rate: f64) {
        for pixel in &mut self.pixels {
            pixel.hue_rotate(rate);
        }
    }

    pub(crate) fn frame_commands(&self) -> Vec<DomeCommand> {
        vec![
            DomeCommand::Frame(self.pixels.iter().map(DomeBufferPixel::rgb).collect()),
            DomeCommand::Flush,
        ]
    }
}

// Spectrum dome config defaults (`SpectrumConfiguration`) hardcoded here because
// domers pins Spectrum's default preset; wiring them from DomersConfig is the
