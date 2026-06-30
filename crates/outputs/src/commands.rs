//! Simulator command protocol matching Spectrum's LEDCommand structs.

use domers_core::Rgb;

/// Dome simulator command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomeCommand {
    /// Flush/redraw marker.
    Flush,
    /// Whole frame in canonical strut-major order.
    Frame(Vec<Rgb>),
    /// Single logical LED write.
    Pixel {
        /// Strut index.
        strut_index: usize,
        /// LED index within the strut.
        led_index: usize,
        /// RGB color.
        color: Rgb,
    },
}

/// Bar simulator command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BarCommand {
    /// Flush marker; redraw behavior is intentionally a no-op for parity.
    Flush,
    /// Bar pixel write.
    Pixel {
        /// Whether the pixel is on the runner strip.
        is_runner: bool,
        /// Logical LED index.
        led_index: usize,
        /// RGB color.
        color: Rgb,
    },
}

/// Stage simulator command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StageCommand {
    /// Flush/redraw marker.
    Flush,
    /// Stage pixel write.
    Pixel {
        /// Side index.
        side_index: usize,
        /// LED index.
        led_index: usize,
        /// Layer index.
        layer_index: usize,
        /// RGB color.
        color: Rgb,
    },
}
