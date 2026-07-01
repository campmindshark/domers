use domers_core::{ColorPalette, PaletteEntry, Rgb};
use serde::Deserialize;

use crate::quaternion::Quaternion;

/// Supported initial live visualizer ports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveVisualizer {
    /// TV static fallback.
    TvStatic,
    /// Audio volume mode.
    Volume,
    /// MIDI flash overlay.
    Flash,
    /// Radial buffer mode.
    Radial,
    /// Splat buffer mode.
    Splat,
    /// Race mode.
    Race,
    /// Snakes mode.
    Snakes,
    /// Quaternion test mode.
    QuaternionTest,
    /// Quaternion multi-test mode.
    QuaternionMultiTest,
    /// Quaternion paintbrush mode.
    QuaternionPaintbrush,
}

/// Used dome diagnostic visualizers from Spectrum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomeDiagnosticVisualizer {
    /// `LEDDomeFlashColorsDiagnosticVisualizer`.
    FlashColors,
    /// `LEDDomeStrutIterationDiagnosticVisualizer`.
    StrutIteration,
    /// `LEDDomeStrandTestDiagnosticVisualizer`.
    StrandTest,
    /// `LEDDomeFullColorFlashDiagnosticVisualizer`.
    FullColorFlash,
}

/// Used bar diagnostic visualizers from Spectrum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarDiagnosticVisualizer {
    /// `LEDBarFlashColorsDiagnosticVisualizer`.
    FlashColors,
}

/// Used stage visualizers from Spectrum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageVisualizer {
    /// `LEDStageFlashColorsDiagnosticVisualizer`.
    FlashColorsDiagnostic,
    /// `LEDStageDepthLevelVisualizer`.
    DepthLevel,
}

/// Deterministic diagnostic frame controls.
#[derive(Clone, Copy, Debug)]
pub struct DiagnosticInput {
    /// Diagnostic state counter, matching Spectrum's timer-advanced state.
    pub state: u8,
    /// Step index for iteration-style diagnostics.
    pub step: usize,
    /// Brightness multiplier in `[0.0, 1.0]`.
    pub brightness: f32,
    /// Normalized volume for audio-reactive support modes.
    pub volume: f32,
    /// Beat progress in `[0.0, 1.0)`.
    pub beat_progress: f64,
}

impl Default for DiagnosticInput {
    fn default() -> Self {
        Self {
            state: 1,
            step: 0,
            brightness: 1.0,
            volume: 0.7,
            beat_progress: 0.25,
        }
    }
}

/// Maximum synthetic MIDI notes replayed on one visualizer frame.
pub const MAX_FRAME_MIDI_NOTES: usize = 4;
/// Maximum live orientation devices passed to visualizers per frame.
pub const MAX_ORIENTATION_DEVICES: usize = 8;

/// Minimal deterministic visualizer input for no-hardware frame tests.
#[derive(Clone, Copy, Debug)]
pub struct VisualizerInput {
    /// Normalized audio volume.
    pub volume: f32,
    /// Beat progress in `[0.0, 1.0)`.
    pub beat_progress: f64,
    /// Runtime frame index for visualizers with Spectrum-style internal motion.
    pub animation_frame: u64,
    /// Monotonic wall-clock time in milliseconds for stateful runtime stepping.
    pub now_ms: u64,
    /// Current measure length in milliseconds when a tempo is known.
    pub measure_length_ms: Option<u32>,
    /// Live wall-clock `ProgressThroughBeat(domeVolumeRotationSpeed)` when tempo is known.
    pub beat_progress_rotation: Option<f64>,
    /// Live wall-clock `ProgressThroughBeat(domeGradientSpeed)` when tempo is known.
    pub beat_progress_gradient: Option<f64>,
    /// Optional yaw/pitch/roll override for simulator-driven orientation previews.
    pub orientation_override: Option<OrientationOverride>,
    /// Live orientation device snapshots (Spectrum `OrientationInput.DevicesSnapshot`).
    pub orientation_devices: [Option<OrientationDeviceInput>; MAX_ORIENTATION_DEVICES],
    /// Synthetic or live MIDI note events for this frame (Flash and overlays).
    pub midi_notes: [Option<MidiNoteInput>; MAX_FRAME_MIDI_NOTES],
    /// Whether a MIDI flash note is active.
    pub flash_active: bool,
    /// Primary operator palette color.
    pub primary: Rgb,
    /// Secondary operator palette color.
    pub secondary: Rgb,
    /// Accent operator palette color.
    pub accent: Rgb,
    /// Active Spectrum palette bank colors 0-7.
    pub palette: [Rgb; 8],
    /// Active Spectrum palette bank entries 0-7.
    pub palette_entries: [PaletteEntry; 8],
    /// Product of Spectrum `domeMaxBrightness` and `domeBrightness`.
    pub dome_brightness: f64,
}

impl Default for VisualizerInput {
    fn default() -> Self {
        let primary = Rgb::from_u24(0x00_ff_00);
        let secondary = Rgb::from_u24(0x00_80_ff);
        let accent = Rgb::from_u24(0xff_40_80);
        Self {
            volume: 0.5,
            beat_progress: 0.25,
            animation_frame: 0,
            now_ms: 0,
            measure_length_ms: None,
            beat_progress_rotation: None,
            beat_progress_gradient: None,
            orientation_override: None,
            orientation_devices: [None; MAX_ORIENTATION_DEVICES],
            midi_notes: [None; MAX_FRAME_MIDI_NOTES],
            flash_active: true,
            primary,
            secondary,
            accent,
            palette: [
                primary,
                secondary,
                accent,
                Rgb::from_u24(0xff_ff_00),
                Rgb::from_u24(0xff_00_ff),
                Rgb::from_u24(0x00_ff_ff),
                Rgb::from_u24(0xff_ff_ff),
                Rgb::BLACK,
            ],
            palette_entries: [
                PaletteEntry::solid(primary.to_u24()),
                PaletteEntry::solid(secondary.to_u24()),
                PaletteEntry::solid(accent.to_u24()),
                PaletteEntry::solid(0xff_ff_00),
                PaletteEntry::solid(0xff_00_ff),
                PaletteEntry::solid(0x00_ff_ff),
                PaletteEntry::solid(0xff_ff_ff),
                PaletteEntry::solid(0),
            ],
            dome_brightness: 1.0,
        }
    }
}

/// One live orientation device snapshot for wand/poi visualizers.
#[derive(Clone, Copy, Debug)]
pub struct OrientationDeviceInput {
    /// Device id from the datagram.
    pub device_id: u8,
    /// Calibrated rotation quaternion (`w`, `x`, `y`, `z`).
    pub rotation: Quaternion,
    /// Spectrum action flag (button press state).
    pub action_flag: u8,
}

/// One MIDI note event delivered during a frame (`index` = pad, `value` = velocity).
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct MidiNoteInput {
    /// Note/controller index (Flash pads 0–15).
    pub index: u8,
    /// Note velocity; `0.0` is note-off.
    pub value: f64,
}

/// Simulator-provided orientation angles in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrientationOverride {
    /// Yaw angle in radians.
    pub yaw: f64,
    /// Pitch angle in radians.
    pub pitch: f64,
    /// Roll angle in radians.
    pub roll: f64,
}

/// Deterministic stage visualizer input with Spectrum palette context.
#[derive(Clone, Debug)]
pub struct StageVisualizerInput {
    /// Shared timing, volume, and diagnostic controls.
    pub diagnostic: DiagnosticInput,
    /// Active Spectrum palette.
    pub color_palette: ColorPalette,
    /// Active Spectrum palette bank.
    pub color_palette_index: u8,
    /// Stage brightness multiplier.
    pub stage_brightness: f64,
}

impl Default for StageVisualizerInput {
    fn default() -> Self {
        Self {
            diagnostic: DiagnosticInput::default(),
            color_palette: ColorPalette::default(),
            color_palette_index: 0,
            stage_brightness: 1.0,
        }
    }
}
