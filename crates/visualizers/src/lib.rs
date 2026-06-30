//! Visualizer inventory and porting order.

use domers_core::Rgb;
use domers_outputs::{topology::DOME_PIXELS, DomeCommand, DomeOutputSink};

/// Porting classification for a Spectrum visualizer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Classification {
    /// Must be ported for parity.
    Live,
    /// Port as diagnostic, fixture, or helper.
    Support,
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
        name: "LEDDomeStrutIterationDiagnosticVisualizer",
        classification: Classification::Support,
    },
    VisualizerInventory {
        name: "LEDDomeFlashColorsDiagnosticVisualizer",
        classification: Classification::Support,
    },
    VisualizerInventory {
        name: "LEDDomeStrandTestDiagnosticVisualizer",
        classification: Classification::Support,
    },
    VisualizerInventory {
        name: "LEDDomeFullColorFlashDiagnosticVisualizer",
        classification: Classification::Support,
    },
    VisualizerInventory {
        name: "LEDDomeVolumeVisualizer",
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
        name: "LEDDomeQuaternionTestVisualizer",
        classification: Classification::Live,
    },
    VisualizerInventory {
        name: "LEDDomeQuaternionMultiTestVisualizer",
        classification: Classification::Live,
    },
    VisualizerInventory {
        name: "LEDDomeQuaternionPaintbrushVisualizer",
        classification: Classification::Live,
    },
    VisualizerInventory {
        name: "LEDDomeTVStaticVisualizer",
        classification: Classification::Live,
    },
    VisualizerInventory {
        name: "LEDDomeFlashVisualizer",
        classification: Classification::Live,
    },
    VisualizerInventory {
        name: "LEDBarFlashColorsDiagnosticVisualizer",
        classification: Classification::Support,
    },
    VisualizerInventory {
        name: "LEDStageFlashColorsDiagnosticVisualizer",
        classification: Classification::Support,
    },
    VisualizerInventory {
        name: "LEDStageDepthLevelVisualizer",
        classification: Classification::Live,
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
    /// Primary operator palette color.
    pub primary: Rgb,
    /// Secondary operator palette color.
    pub secondary: Rgb,
    /// Accent operator palette color.
    pub accent: Rgb,
}

impl Default for VisualizerInput {
    fn default() -> Self {
        Self {
            volume: 0.5,
            beat_progress: 0.25,
            flash_active: true,
            primary: Rgb::from_u24(0x00_ff_00),
            secondary: Rgb::from_u24(0x00_80_ff),
            accent: Rgb::from_u24(0xff_40_80),
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
    sink.write_buffer(match visualizer {
        LiveVisualizer::TvStatic => tv_static_frame(input),
        LiveVisualizer::Volume => volume_frame(input),
        LiveVisualizer::Flash => flash_frame(input),
        LiveVisualizer::Radial => radial_frame(input),
        LiveVisualizer::Splat => splat_frame(input),
        LiveVisualizer::Race => race_frame(input),
        LiveVisualizer::Snakes => snakes_frame(input),
        LiveVisualizer::QuaternionTest => quaternion_test_frame(input),
        LiveVisualizer::QuaternionMultiTest => quaternion_multi_test_frame(input),
        LiveVisualizer::QuaternionPaintbrush => quaternion_paintbrush_frame(input),
    });
    sink.flush();
    sink.drain_commands()
}

fn tv_static_frame(input: VisualizerInput) -> Vec<Rgb> {
    preview_frame(|index| {
        if index % 3 == 0 {
            input.secondary.scale(0.35)
        } else if index % 5 == 0 {
            input.accent.scale(0.25)
        } else {
            Rgb::from_u24(0x18_18_18)
        }
    })
}

fn volume_frame(input: VisualizerInput) -> Vec<Rgb> {
    let lit = lit_count(input.volume);
    preview_frame(|index| {
        if index <= lit {
            if index % 2 == 0 {
                input.primary.scale(input.volume)
            } else {
                input.secondary.scale(input.volume)
            }
        } else {
            Rgb::from_u24(0x02_02_02)
        }
    })
}

fn flash_frame(input: VisualizerInput) -> Vec<Rgb> {
    let color = if input.flash_active {
        input.accent
    } else {
        input.primary.scale(0.2)
    };
    preview_frame(|index| {
        if index % 4 == 0 {
            color
        } else {
            input.secondary.scale(0.2)
        }
    })
}

fn radial_frame(input: VisualizerInput) -> Vec<Rgb> {
    let offset = phase_offset(input.beat_progress);
    preview_frame(|index| match (index + offset) % 3 {
        0 => input.primary,
        1 => input.secondary,
        _ => input.accent,
    })
}

fn splat_frame(input: VisualizerInput) -> Vec<Rgb> {
    preview_frame(|index| {
        if index % 11 == 0 || index % 17 == 0 {
            input.accent
        } else {
            input.primary.scale(0.18)
        }
    })
}

fn race_frame(input: VisualizerInput) -> Vec<Rgb> {
    let offset = phase_offset(input.beat_progress);
    preview_frame(|index| {
        let distance = (index + DOME_PIXELS - offset) % DOME_PIXELS;
        if distance < 320 {
            input.accent
        } else if distance < 640 {
            input.secondary.scale(0.45)
        } else {
            Rgb::BLACK
        }
    })
}

fn snakes_frame(input: VisualizerInput) -> Vec<Rgb> {
    let offset = phase_offset(input.beat_progress);
    preview_frame(|index| {
        let lane = (index + offset) % 24;
        if lane < 5 {
            input.primary
        } else if lane < 9 {
            input.secondary.scale(0.6)
        } else {
            Rgb::BLACK
        }
    })
}

fn quaternion_test_frame(input: VisualizerInput) -> Vec<Rgb> {
    preview_frame(|index| {
        if index % 8 < 4 {
            input.secondary
        } else {
            input.primary.scale(0.3)
        }
    })
}

fn quaternion_multi_test_frame(input: VisualizerInput) -> Vec<Rgb> {
    preview_frame(|index| match index % 4 {
        0 => input.primary,
        1 => input.secondary,
        2 => input.accent,
        _ => Rgb::BLACK,
    })
}

fn quaternion_paintbrush_frame(input: VisualizerInput) -> Vec<Rgb> {
    let offset = phase_offset(input.beat_progress);
    preview_frame(|index| {
        if (index + offset) % 13 < 6 {
            input.accent
        } else {
            Rgb::BLACK
        }
    })
}

fn preview_frame(mut color_for_index: impl FnMut(usize) -> Rgb) -> Vec<Rgb> {
    (0..DOME_PIXELS).map(&mut color_for_index).collect()
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "Simulator preview clamps normalized controls before converting to an index"
)]
fn lit_count(volume: f32) -> usize {
    (volume.clamp(0.0, 1.0) * DOME_PIXELS as f32) as usize
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "Simulator preview clamps normalized beat progress before converting to an index"
)]
fn phase_offset(beat_progress: f64) -> usize {
    (beat_progress.clamp(0.0, 1.0) * DOME_PIXELS as f64) as usize
}

#[cfg(test)]
mod tests {
    use domers_outputs::{topology::DOME_PIXELS, DomeCommand};

    use super::{
        render_dome_visualizer, Classification, LiveVisualizer, VisualizerInput, INVENTORY,
    };

    #[test]
    fn inventory_tracks_used_spectrum_visualizers() {
        assert_eq!(INVENTORY.len(), 17);
        assert_eq!(
            INVENTORY
                .iter()
                .filter(|visualizer| visualizer.classification == Classification::Live)
                .count(),
            11
        );
        assert_eq!(
            INVENTORY
                .iter()
                .filter(|visualizer| visualizer.classification == Classification::Support)
                .count(),
            6
        );
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

    #[test]
    fn default_volume_preview_is_dome_sized_and_visible() {
        let commands = render_dome_visualizer(LiveVisualizer::Volume, VisualizerInput::default());
        let frame = commands
            .iter()
            .find_map(|command| match command {
                DomeCommand::Frame(colors) => Some(colors),
                DomeCommand::Flush | DomeCommand::Pixel { .. } => None,
            })
            .expect("volume visualizer should write a whole preview frame");

        assert_eq!(frame.len(), DOME_PIXELS);
        assert!(
            frame
                .iter()
                .filter(|color| **color != domers_core::Rgb::BLACK)
                .count()
                > 3_000
        );
    }
}
