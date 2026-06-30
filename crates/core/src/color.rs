//! RGB color helpers shared by outputs, visualizers, and simulator rendering.

/// Packed RGB color with explicit channels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Rgb {
    /// Black/off.
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };

    /// Construct from Spectrum's `0xRRGGBB` integer convention.
    #[must_use]
    pub const fn from_u24(value: u32) -> Self {
        Self {
            r: ((value >> 16) & 0xff) as u8,
            g: ((value >> 8) & 0xff) as u8,
            b: (value & 0xff) as u8,
        }
    }

    /// Convert to Spectrum's `0xRRGGBB` integer convention.
    #[must_use]
    pub const fn to_u24(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }

    /// Scale by brightness, clamping into displayable RGB.
    #[must_use]
    pub fn scale(self, factor: f32) -> Self {
        fn ch(value: u8, factor: f32) -> u8 {
            ((value as f32 * factor).clamp(0.0, 255.0)).round() as u8
        }
        Self {
            r: ch(self.r, factor),
            g: ch(self.g, factor),
            b: ch(self.b, factor),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Rgb;

    #[test]
    fn converts_spectrum_packed_rgb() {
        let rgb = Rgb::from_u24(0x12_34_56);
        assert_eq!(
            rgb,
            Rgb {
                r: 0x12,
                g: 0x34,
                b: 0x56
            }
        );
        assert_eq!(rgb.to_u24(), 0x12_34_56);
    }

    #[test]
    fn scales_with_clamping() {
        assert_eq!(Rgb::from_u24(0x80_40_20).scale(0.5).to_u24(), 0x40_20_10);
        assert_eq!(Rgb::from_u24(0xff_80_01).scale(2.0).to_u24(), 0xff_ff_02);
    }
}
