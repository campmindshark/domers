//! RGB and palette helpers shared by outputs, visualizers, and simulator rendering.

use serde::{Deserialize, Serialize};

/// Packed RGB color with explicit channels.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

/// One Spectrum palette entry.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct PaletteEntry {
    /// First color encoded as `0xRRGGBB`.
    pub color1: u32,
    /// Second color encoded as `0xRRGGBB`.
    pub color2: u32,
    /// Whether this entry blends between both colors.
    pub color2_enabled: bool,
}

impl PaletteEntry {
    /// Create a solid palette entry.
    #[must_use]
    pub const fn solid(color: u32) -> Self {
        Self {
            color1: color,
            color2: 0,
            color2_enabled: false,
        }
    }

    /// Create a gradient palette entry.
    #[must_use]
    pub const fn gradient(color1: u32, color2: u32) -> Self {
        Self {
            color1,
            color2,
            color2_enabled: true,
        }
    }

    /// Return the solid color for this entry.
    #[must_use]
    pub const fn single_color(self) -> Rgb {
        Rgb::from_u24(self.color1)
    }

    /// Match Spectrum's gradient blend calculation.
    #[must_use]
    pub fn gradient_color(self, pixel_pos: f64, focus_pos: f64, wrap: bool) -> Rgb {
        if !self.color2_enabled {
            return self.single_color();
        }

        let raw_distance = (pixel_pos - focus_pos).abs();
        let distance = if wrap {
            raw_distance.min(1.0 - raw_distance) * 2.0
        } else {
            raw_distance
        }
        .clamp(0.0, 1.0);

        blend_spectrum(
            Rgb::from_u24(self.color1),
            Rgb::from_u24(self.color2),
            distance,
        )
    }
}

impl Default for PaletteEntry {
    fn default() -> Self {
        Self::solid(0)
    }
}

/// Spectrum-compatible palette: eight banks with eight entries each.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ColorPalette {
    /// Palette entries in Spectrum's absolute color index order.
    pub colors: Vec<PaletteEntry>,
}

impl ColorPalette {
    /// Number of palette banks in Spectrum.
    pub const BANKS: usize = 8;
    /// Number of colors in each bank.
    pub const COLORS_PER_BANK: usize = 8;
    /// Total palette entries.
    pub const ENTRY_COUNT: usize = Self::BANKS * Self::COLORS_PER_BANK;

    /// Convert a bank-relative color index into Spectrum's absolute index.
    #[must_use]
    pub const fn absolute_index(relative_color_index: usize, color_palette_index: u8) -> usize {
        relative_color_index + color_palette_index as usize * Self::COLORS_PER_BANK
    }

    /// Return one palette entry if present.
    #[must_use]
    pub fn entry(&self, absolute_index: usize) -> PaletteEntry {
        self.colors.get(absolute_index).copied().unwrap_or_default()
    }

    /// Return a bank-relative solid color.
    #[must_use]
    pub fn single_color(&self, relative_color_index: usize, color_palette_index: u8) -> Rgb {
        self.entry(Self::absolute_index(
            relative_color_index,
            color_palette_index,
        ))
        .single_color()
    }

    /// Return a bank-relative gradient color.
    #[must_use]
    pub fn gradient_color(
        &self,
        relative_color_index: usize,
        color_palette_index: u8,
        pixel_pos: f64,
        focus_pos: f64,
        wrap: bool,
    ) -> Rgb {
        self.entry(Self::absolute_index(
            relative_color_index,
            color_palette_index,
        ))
        .gradient_color(pixel_pos, focus_pos, wrap)
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        let mut colors = vec![PaletteEntry::default(); Self::ENTRY_COUNT];
        colors[0] = PaletteEntry::solid(0x00_ff_00);
        colors[1] = PaletteEntry::solid(0x00_80_ff);
        colors[2] = PaletteEntry::solid(0xff_40_80);
        Self { colors }
    }
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
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "RGB protocol values are clamped before conversion back to u8"
        )]
        fn ch(value: u8, factor: f32) -> u8 {
            (f32::from(value) * factor).clamp(0.0, 255.0) as u8
        }
        Self {
            r: ch(self.r, factor),
            g: ch(self.g, factor),
            b: ch(self.b, factor),
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Spectrum gradient math truncates blended channel values to byte"
)]
fn blend_spectrum(color1: Rgb, color2: Rgb, distance: f64) -> Rgb {
    let inverse = 1.0 - distance;
    Rgb {
        r: (distance * f64::from(color1.r) + inverse * f64::from(color2.r)) as u8,
        g: (distance * f64::from(color1.g) + inverse * f64::from(color2.g)) as u8,
        b: (distance * f64::from(color1.b) + inverse * f64::from(color2.b)) as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::{ColorPalette, PaletteEntry, Rgb};

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

    #[test]
    fn palette_uses_spectrum_absolute_indices() {
        assert_eq!(ColorPalette::absolute_index(0, 0), 0);
        assert_eq!(ColorPalette::absolute_index(0, 3), 24);
        assert_eq!(ColorPalette::absolute_index(7, 7), 63);
    }

    #[test]
    fn palette_matches_spectrum_gradient_focus_math() {
        let palette = ColorPalette {
            colors: vec![PaletteEntry::gradient(0xff_00_00, 0x00_00_ff)],
        };

        assert_eq!(
            palette.gradient_color(0, 0, 0.0, 0.0, false).to_u24(),
            0x00_00_ff
        );
        assert_eq!(
            palette.gradient_color(0, 0, 1.0, 0.0, false).to_u24(),
            0xff_00_00
        );
        assert_eq!(
            palette.gradient_color(0, 0, 0.5, 0.0, false).to_u24(),
            0x7f_00_7f
        );
    }
}
