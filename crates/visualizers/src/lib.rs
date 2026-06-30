//! Visualizer inventory and porting order.

use domers_core::Rgb;
use domers_outputs::{DomeCommand, DomeOutputSink};

/// Porting classification for a Spectrum visualizer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Classification {
    /// Must be ported for parity.
    Live,
    /// Port as diagnostic, fixture, or helper.
    Support,
    /// Do not port unless intentionally redesigned.
    Dead,
}

/// A visualizer inventory row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisualizerInventory {
    /// Stable name.
    pub name: &'static str,
    /// Classification.
    pub classification: Classification,
}

/// Initial reviewed visualizer inventory.
pub const INVENTORY: &[VisualizerInventory] = &[
    VisualizerInventory {
        name: "LEDDomeVolumeVisualizer",
        classification: Classification::Live,
    },
    VisualizerInventory {
        name: "LEDDomeFlashVisualizer",
        classification: Classification::Live,
    },
    VisualizerInventory {
        name: "LEDDomeRadialVisualizer",
        classification: Classification::Live,
    },
    VisualizerInventory {
        name: "LEDDomeRaceVisualizer",
        classification: Classification::Live,
    },
    VisualizerInventory {
        name: "LEDDomeSnakesVisualizer",
        classification: Classification::Live,
    },
    VisualizerInventory {
        name: "LEDDomeSplatVisualizer",
        classification: Classification::Live,
    },
    VisualizerInventory {
        name: "LEDStageDepthLevelVisualizer",
        classification: Classification::Live,
    },
    VisualizerInventory {
        name: "LEDDomeMidiTestVisualizer",
        classification: Classification::Dead,
    },
    VisualizerInventory {
        name: "LEDStageTracerVisualizer",
        classification: Classification::Dead,
    },
];

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

/// Minimal deterministic visualizer input for no-hardware frame tests.
#[derive(Clone, Copy, Debug)]
pub struct VisualizerInput {
    /// Normalized audio volume.
    pub volume: f32,
    /// Beat progress in `[0.0, 1.0)`.
    pub beat_progress: f64,
    /// Whether a MIDI flash note is active.
    pub flash_active: bool,
}

impl Default for VisualizerInput {
    fn default() -> Self {
        Self {
            volume: 0.5,
            beat_progress: 0.25,
            flash_active: true,
        }
    }
}

/// Render one deterministic simulator frame for a live visualizer.
#[must_use]
pub fn render_dome_visualizer(
    visualizer: LiveVisualizer,
    input: VisualizerInput,
) -> Vec<DomeCommand> {
    let mut sink = DomeOutputSink::new(false, true);
    match visualizer {
        LiveVisualizer::TvStatic => {
            sink.set_pixel(0, 0, Rgb::from_u24(0x20_20_20));
            sink.set_pixel(1, 0, Rgb::BLACK);
        }
        LiveVisualizer::Volume => {
            sink.set_pixel(0, 0, Rgb::from_u24(0x00_ff_00).scale(input.volume));
        }
        LiveVisualizer::Flash => {
            if input.flash_active {
                sink.set_pixel(2, 0, Rgb::from_u24(0xff_ff_ff));
            }
        }
        LiveVisualizer::Radial => sink.write_buffer(vec![
            Rgb::from_u24(0xff_00_00),
            Rgb::from_u24(0x00_ff_00),
            Rgb::from_u24(0x00_00_ff),
        ]),
        LiveVisualizer::Splat => sink.write_buffer(vec![Rgb::from_u24(0xff_80_00)]),
        LiveVisualizer::Race => {
            let strut = if input.beat_progress < 0.5 { 3 } else { 4 };
            sink.set_pixel(strut, 0, Rgb::from_u24(0xff_00_ff));
        }
        LiveVisualizer::Snakes => {
            sink.set_pixel(5, 0, Rgb::from_u24(0x00_ff_ff));
            sink.set_pixel(6, 0, Rgb::from_u24(0x00_80_80));
        }
        LiveVisualizer::QuaternionTest => sink.write_buffer(vec![Rgb::from_u24(0x10_20_30)]),
        LiveVisualizer::QuaternionMultiTest => {
            sink.write_buffer(vec![Rgb::from_u24(0x10_00_00), Rgb::from_u24(0x00_10_00)]);
        }
        LiveVisualizer::QuaternionPaintbrush => {
            sink.write_buffer(vec![Rgb::from_u24(0xff_00_80)]);
        }
    }
    sink.flush();
    sink.drain_commands()
}

#[cfg(test)]
mod tests {
    use domers_outputs::DomeCommand;

    use super::{
        render_dome_visualizer, Classification, LiveVisualizer, VisualizerInput, INVENTORY,
    };

    #[test]
    fn records_confirmed_dead_visualizers() {
        assert!(INVENTORY
            .iter()
            .any(|v| v.name == "LEDDomeMidiTestVisualizer"
                && v.classification == Classification::Dead));
        assert!(INVENTORY
            .iter()
            .any(|v| v.name == "LEDStageTracerVisualizer"
                && v.classification == Classification::Dead));
    }

    #[test]
    fn every_initial_live_dome_visualizer_produces_a_simulator_frame() {
        for visualizer in [
            LiveVisualizer::TvStatic,
            LiveVisualizer::Volume,
            LiveVisualizer::Flash,
            LiveVisualizer::Radial,
            LiveVisualizer::Splat,
            LiveVisualizer::Race,
            LiveVisualizer::Snakes,
            LiveVisualizer::QuaternionTest,
            LiveVisualizer::QuaternionMultiTest,
            LiveVisualizer::QuaternionPaintbrush,
        ] {
            let commands = render_dome_visualizer(visualizer, VisualizerInput::default());
            assert!(
                commands
                    .iter()
                    .any(|command| matches!(command, DomeCommand::Flush)),
                "{visualizer:?} should flush"
            );
            assert!(
                commands.len() >= 2,
                "{visualizer:?} should write before flush"
            );
        }
    }

    #[test]
    fn buffer_based_modes_use_whole_frame_commands() {
        let commands = render_dome_visualizer(LiveVisualizer::Radial, VisualizerInput::default());
        assert!(commands
            .iter()
            .any(|command| matches!(command, DomeCommand::Frame(_))));
    }
}
