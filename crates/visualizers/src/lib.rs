//! Visualizer inventory and porting order.

use domers_core::{ColorPalette, PaletteEntry, Rgb};
use domers_outputs::{
    dome_strut_index_for_control_box, dome_strut_length,
    topology::{DOME_PIXELS, DOME_STRUTS, STAGE_LAYERS},
    BarCommand, DomeCommand, DomeOutputSink, StageCommand,
};
use serde::Deserialize;
use std::sync::OnceLock;

const DOME_GEOMETRY_JSON: &str =
    include_str!("../../../fixtures/spectrum-csharp/dome_geometry.json");
const DOME_MAPPING_JSON: &str = include_str!("../../../fixtures/spectrum-csharp/dome_mapping.json");
static DOME_LED_POINTS: OnceLock<Vec<DomeLedPoint>> = OnceLock::new();

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

/// Minimal deterministic visualizer input for no-hardware frame tests.
#[derive(Clone, Copy, Debug)]
pub struct VisualizerInput {
    /// Normalized audio volume.
    pub volume: f32,
    /// Beat progress in `[0.0, 1.0)`.
    pub beat_progress: f64,
    /// Runtime frame index for visualizers with Spectrum-style internal motion.
    pub animation_frame: u64,
    /// Optional yaw/pitch/roll override for simulator-driven orientation previews.
    pub orientation_override: Option<OrientationOverride>,
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
            orientation_override: None,
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
        }
    }
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

/// Render one deterministic simulator frame for a live visualizer.
#[must_use]
pub fn render_dome_visualizer(
    visualizer: LiveVisualizer,
    input: VisualizerInput,
) -> Vec<DomeCommand> {
    if visualizer == LiveVisualizer::Flash {
        return Vec::new();
    }
    let mut sink = DomeOutputSink::new(false, true);
    sink.write_buffer(match visualizer {
        LiveVisualizer::TvStatic => tv_static_frame(input),
        LiveVisualizer::Volume => volume_frame(input),
        LiveVisualizer::Flash => unreachable!("Flash visualizer is event-driven"),
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

/// Render one used dome diagnostic visualizer frame.
#[must_use]
pub fn render_dome_diagnostic(
    visualizer: DomeDiagnosticVisualizer,
    input: DiagnosticInput,
) -> Vec<DomeCommand> {
    match visualizer {
        DomeDiagnosticVisualizer::FlashColors => dome_flash_colors_commands(input),
        DomeDiagnosticVisualizer::StrutIteration => dome_strut_iteration_commands(input),
        DomeDiagnosticVisualizer::StrandTest => dome_strand_test_commands(input),
        DomeDiagnosticVisualizer::FullColorFlash => dome_full_color_flash_commands(input),
    }
}

/// Render one used bar diagnostic visualizer frame.
#[must_use]
pub fn render_bar_diagnostic(
    visualizer: BarDiagnosticVisualizer,
    input: DiagnosticInput,
    infinity_width: usize,
    infinity_length: usize,
    runner_length: usize,
) -> Vec<BarCommand> {
    match visualizer {
        BarDiagnosticVisualizer::FlashColors => {
            bar_flash_colors(input, infinity_width, infinity_length, runner_length)
        }
    }
}

/// Render one used stage visualizer frame.
#[must_use]
pub fn render_stage_visualizer(
    visualizer: StageVisualizer,
    input: DiagnosticInput,
    side_lengths: &[usize],
) -> Vec<StageCommand> {
    render_stage_visualizer_with_input(
        visualizer,
        StageVisualizerInput {
            diagnostic: input,
            ..StageVisualizerInput::default()
        },
        side_lengths,
    )
}

/// Render one used stage visualizer frame with full Spectrum palette context.
#[must_use]
pub fn render_stage_visualizer_with_input(
    visualizer: StageVisualizer,
    input: StageVisualizerInput,
    side_lengths: &[usize],
) -> Vec<StageCommand> {
    match visualizer {
        StageVisualizer::FlashColorsDiagnostic => {
            stage_flash_colors(input.diagnostic, side_lengths)
        }
        StageVisualizer::DepthLevel => stage_depth_level(input, side_lengths),
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "TV static clamps normalized brightness before converting to an RGB byte cap"
)]
fn tv_static_frame(input: VisualizerInput) -> Vec<Rgb> {
    let seed = phase_offset(input.beat_progress) as u32;
    let brightness = (input.volume.clamp(0.05, 1.0) * 255.0).round() as u8;
    preview_frame(|index| {
        let index = index as u32;
        Rgb {
            r: static_channel(index, 0, seed, brightness),
            g: static_channel(index, 1, seed, brightness),
            b: static_channel(index, 2, seed, brightness),
        }
    })
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "Pseudo-random generator intentionally takes the high byte after mixing"
)]
fn static_channel(index: u32, channel: u32, seed: u32, brightness: u8) -> u8 {
    let mut value = index
        .wrapping_mul(1_664_525)
        .wrapping_add(channel.wrapping_mul(1_013_904_223))
        .wrapping_add(seed.wrapping_mul(22_695_477))
        .wrapping_add(1_013_904_223);
    value ^= value >> 16;
    value = value.wrapping_mul(2_246_822_519);
    ((value >> 24) as u8) % brightness.saturating_add(1)
}

fn dome_set_all_commands(color: Rgb) -> Vec<DomeCommand> {
    let mut commands = Vec::new();
    for strut_index in 0..DOME_STRUTS {
        let Some(length) = dome_strut_length(strut_index) else {
            continue;
        };
        for led_index in 0..length {
            commands.push(DomeCommand::Pixel {
                strut_index,
                led_index,
                color,
            });
        }
    }
    commands.push(DomeCommand::Flush);
    commands
}

fn dome_set_all_control_box_commands(color: Rgb) -> Vec<DomeCommand> {
    let mut commands = Vec::new();
    for control_box in 0..5 {
        for local_index in 0..38 {
            let Some(strut_index) = dome_strut_index_for_control_box(control_box, local_index)
            else {
                continue;
            };
            let Some(strut_length) = dome_strut_length(strut_index) else {
                continue;
            };
            for led_index in 0..strut_length {
                commands.push(DomeCommand::Pixel {
                    strut_index,
                    led_index,
                    color,
                });
            }
        }
    }
    commands.push(DomeCommand::Flush);
    commands
}

fn dome_flash_colors_commands(input: DiagnosticInput) -> Vec<DomeCommand> {
    if input.state == 0 {
        return dome_set_all_commands(Rgb::BLACK);
    }

    let colors = diagnostic_colors(input.brightness);
    let mut commands = Vec::new();
    for control_box in 0..5 {
        let mut color_index = 0;
        for local_index in 0..38 {
            let Some(strut_index) = dome_strut_index_for_control_box(control_box, local_index)
            else {
                continue;
            };
            let Some(strut_length) = dome_strut_length(strut_index) else {
                continue;
            };
            let color = colors[color_index % 6];
            if input.state == 2 {
                for led_index in 1..strut_length.saturating_sub(1) {
                    commands.push(DomeCommand::Pixel {
                        strut_index,
                        led_index,
                        color: Rgb::BLACK,
                    });
                }
                for led_index in [0, strut_length.saturating_sub(1)] {
                    commands.push(DomeCommand::Pixel {
                        strut_index,
                        led_index,
                        color,
                    });
                }
            } else {
                for led_index in 0..strut_length {
                    commands.push(DomeCommand::Pixel {
                        strut_index,
                        led_index,
                        color,
                    });
                }
            }
            color_index = (color_index + 1) % colors.len();
        }
    }
    commands.push(DomeCommand::Flush);
    commands
}

fn dome_strut_iteration_commands(input: DiagnosticInput) -> Vec<DomeCommand> {
    let mut commands = Vec::new();
    let local_index = input.step % 38;
    let control_box = (input.step / 38) % 5;
    if local_index == 0 {
        for strut_index in 0..DOME_STRUTS {
            let Some(strut_length) = dome_strut_length(strut_index) else {
                continue;
            };
            for led_index in 0..strut_length {
                commands.push(DomeCommand::Pixel {
                    strut_index,
                    led_index,
                    color: Rgb::from_u24(0x00_00_ff),
                });
            }
        }
    }
    let color_cycle = [
        Rgb::from_u24(0xff_00_00),
        Rgb::from_u24(0x00_ff_00),
        Rgb::from_u24(0x00_00_ff),
        Rgb::from_u24(0xff_ff_ff),
    ];
    let color =
        color_cycle[((input.step / (38 * 5)) + 1) % color_cycle.len()].scale(input.brightness);
    if let Some(strut_index) = dome_strut_index_for_control_box(control_box, local_index) {
        if let Some(strut_length) = dome_strut_length(strut_index) {
            for led_index in 0..strut_length {
                commands.push(DomeCommand::Pixel {
                    strut_index,
                    led_index,
                    color,
                });
            }
        }
    }
    commands.push(DomeCommand::Flush);
    commands
}

fn dome_strand_test_commands(input: DiagnosticInput) -> Vec<DomeCommand> {
    if input.state == 0 {
        dome_set_all_commands(Rgb::BLACK)
    } else {
        dome_set_all_control_box_commands(white(input.brightness))
    }
}

fn dome_full_color_flash_commands(input: DiagnosticInput) -> Vec<DomeCommand> {
    if input.state == 0 {
        dome_set_all_commands(Rgb::BLACK)
    } else {
        dome_set_all_control_box_commands(white(input.brightness))
    }
}

fn bar_flash_colors(
    input: DiagnosticInput,
    infinity_width: usize,
    infinity_length: usize,
    runner_length: usize,
) -> Vec<BarCommand> {
    let mut commands = Vec::new();
    let colors = diagnostic_colors(input.brightness);
    let infinity_pixels = 2 * infinity_length + 2 * infinity_width;

    for index in 0..infinity_pixels {
        let color = if input.state == 0
            || (input.state == 2 && !is_bar_border(index, infinity_width, infinity_length))
        {
            Rgb::BLACK
        } else {
            colors[bar_segment(index, infinity_width, infinity_length)]
        };
        commands.push(BarCommand::Pixel {
            is_runner: false,
            led_index: index,
            color,
        });
    }

    for index in 0..runner_length {
        let color =
            if input.state == 0 || (input.state == 2 && index != 0 && index + 1 != runner_length) {
                Rgb::BLACK
            } else {
                colors[4]
            };
        commands.push(BarCommand::Pixel {
            is_runner: true,
            led_index: index,
            color,
        });
    }

    commands.push(BarCommand::Flush);
    commands
}

fn stage_flash_colors(input: DiagnosticInput, side_lengths: &[usize]) -> Vec<StageCommand> {
    let mut commands = Vec::new();
    let colors = diagnostic_colors(input.brightness);
    if input.state == 0 {
        for (side_index, side_length) in side_lengths.iter().copied().enumerate() {
            for led_index in 0..side_length {
                for layer_index in 0..STAGE_LAYERS {
                    commands.push(StageCommand::Pixel {
                        side_index,
                        led_index,
                        layer_index,
                        color: Rgb::BLACK,
                    });
                }
            }
        }
        commands.push(StageCommand::Flush);
        return commands;
    }

    let mut color_index = 0;
    for (side_index, side_length) in side_lengths.iter().copied().enumerate() {
        for layer_index in 0..STAGE_LAYERS {
            for led_index in 0..side_length {
                let color = if input.state == 2 && led_index != 0 && led_index + 1 != side_length {
                    Rgb::BLACK
                } else {
                    colors[color_index % colors.len()]
                };
                commands.push(StageCommand::Pixel {
                    side_index,
                    led_index,
                    layer_index,
                    color,
                });
            }
            color_index += 1;
        }
    }
    commands.push(StageCommand::Flush);
    commands
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "Stage preview converts small fixture indices into normalized animation positions"
)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Stage visualizer input is assembled per frame and kept by value with other renderer inputs"
)]
fn stage_depth_level(input: StageVisualizerInput, side_lengths: &[usize]) -> Vec<StageCommand> {
    let mut commands = Vec::new();
    let diagnostic = input.diagnostic;
    let triangles = side_lengths.len() / 3;
    for triangle_index in 0..triangles {
        let tracer_index =
            stage_tracer_led_index(side_lengths, triangle_index, diagnostic.beat_progress);
        let max_triangle_counter = triangle_length(side_lengths, triangle_index);
        let mut triangle_counter = 0;
        for side_offset in 0..3 {
            let side_index = triangle_index * 3 + side_offset;
            let side_length = side_lengths[side_index];
            for led_index in 0..side_length {
                let second_part = stage_second_part(side_index) ^ (diagnostic.beat_progress > 0.5);
                let color = stage_gradient_color(
                    &input.color_palette,
                    input.color_palette_index,
                    usize::from(second_part),
                    triangle_counter,
                    max_triangle_counter,
                    tracer_index,
                    input.stage_brightness,
                    diagnostic.volume,
                );
                for layer_index in 0..STAGE_LAYERS {
                    commands.push(StageCommand::Pixel {
                        side_index,
                        led_index,
                        layer_index,
                        color,
                    });
                }
                triangle_counter += 1;
            }
        }
    }
    commands.push(StageCommand::Flush);
    commands
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "Stage tracer math mirrors Spectrum's double-to-int LED index calculation"
)]
fn stage_tracer_led_index(
    side_lengths: &[usize],
    triangle_index: usize,
    beat_progress: f64,
) -> usize {
    let progress = (beat_progress.fract() * 3.0).clamp(0.0, 2.999_999);
    let base = triangle_index * 3;
    if progress < 1.0 {
        (progress * side_lengths[base] as f64) as usize
    } else if progress < 2.0 {
        side_lengths[base] + ((progress - 1.0) * side_lengths[base + 1] as f64) as usize
    } else {
        side_lengths[base]
            + side_lengths[base + 1]
            + ((progress - 2.0) * side_lengths[base + 2] as f64) as usize
    }
}

fn triangle_length(side_lengths: &[usize], triangle_index: usize) -> usize {
    side_lengths[triangle_index * 3..triangle_index * 3 + 3]
        .iter()
        .sum()
}

fn stage_second_part(side_index: usize) -> bool {
    if (side_index / 12) == 1 {
        ((side_index / 3) % 4) == 2
    } else {
        ((side_index / 3) % 4) == 1
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "Visualizer color focus uses normalized LED counters before RGB scaling"
)]
#[allow(
    clippy::too_many_arguments,
    reason = "Mirrors Spectrum's gradient calculation inputs without hiding state in a temporary struct"
)]
fn stage_gradient_color(
    palette: &ColorPalette,
    palette_index: u8,
    relative_color_index: usize,
    triangle_counter: usize,
    max_triangle_counter: usize,
    tracer_index: usize,
    stage_brightness: f64,
    volume: f32,
) -> Rgb {
    let pixel_pos = triangle_counter as f64 / max_triangle_counter.max(1) as f64;
    let focus_pos = tracer_index as f64 / max_triangle_counter.max(1) as f64;
    let color = palette.gradient_color(
        relative_color_index,
        palette_index,
        pixel_pos,
        focus_pos,
        true,
    );
    scale_rgb_f64(scale_rgb_f64(color, stage_brightness), f64::from(volume))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Spectrum LEDColor.ScaleColor truncates scaled double channels to integers"
)]
fn scale_rgb_f64(color: Rgb, scale: f64) -> Rgb {
    Rgb {
        r: (f64::from(color.r) * scale).clamp(0.0, 255.0) as u8,
        g: (f64::from(color.g) * scale).clamp(0.0, 255.0) as u8,
        b: (f64::from(color.b) * scale).clamp(0.0, 255.0) as u8,
    }
}

fn diagnostic_colors(brightness: f32) -> [Rgb; 6] {
    [
        Rgb::from_u24(0xff_00_00).scale(brightness),
        Rgb::from_u24(0x00_ff_00).scale(brightness),
        Rgb::from_u24(0x00_00_ff).scale(brightness),
        Rgb::from_u24(0xff_ff_00).scale(brightness),
        Rgb::from_u24(0xff_00_ff).scale(brightness),
        Rgb::from_u24(0x00_ff_ff).scale(brightness),
    ]
}

fn white(brightness: f32) -> Rgb {
    Rgb::from_u24(0xff_ff_ff).scale(brightness)
}

fn is_bar_border(index: usize, infinity_width: usize, infinity_length: usize) -> bool {
    let second_length_start = infinity_width + infinity_length;
    let second_width_start = infinity_width + 2 * infinity_length;
    index == 0
        || index + 1 == infinity_length
        || index == infinity_length
        || index + 1 == infinity_length + infinity_width
        || index == second_length_start
        || index + 1 == second_length_start + infinity_length
        || index == second_width_start
        || index + 1 == second_width_start + infinity_width
}

fn bar_segment(index: usize, infinity_width: usize, infinity_length: usize) -> usize {
    if index < infinity_length {
        0
    } else if index < infinity_length + infinity_width {
        1
    } else if index < infinity_width + 2 * infinity_length {
        2
    } else {
        3
    }
}

fn volume_frame(input: VisualizerInput) -> Vec<Rgb> {
    let lit = lit_count(input.volume);
    preview_frame(|index| {
        if index <= lit {
            input.palette[(index / 233) % 4].scale(input.volume)
        } else {
            Rgb::from_u24(0x02_02_02)
        }
    })
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Spectrum chooses the radial gradient by truncating normalized volume times 8"
)]
fn radial_frame(input: VisualizerInput) -> Vec<Rgb> {
    let adjusted_level = f64::from(input.volume.clamp(0.0, 1.0))
        .sqrt()
        .clamp(0.1, 1.0);
    let progress = if input.animation_frame == 0 {
        0.0
    } else {
        input.beat_progress
    };
    let current_angle = wrap(progress * 0.25 * 0.25, 0.0, 1.0);
    let current_gradient = wrap(progress * 0.25, 0.0, 1.0);
    let which_gradient = (f64::from(input.volume.clamp(0.0, 1.0)) * 8.0) as usize;
    let size_limit = adjusted_level;

    DOME_LED_POINTS
        .get_or_init(build_dome_led_points)
        .iter()
        .map(|point| {
            let px = point.x * 2.0 - 1.0;
            let py = point.y * 2.0 - 1.0;
            let angle = map_wrap(
                py.atan2(px),
                -std::f64::consts::PI,
                std::f64::consts::PI,
                0.0,
                1.0,
            );
            let dist = (px * px + py * py).sqrt();
            let mut val = map_wrap(angle, current_angle, 1.0 + current_angle, 0.0, 1.0);
            val = wrap(val, 0.0, 1.0);
            val = (val * 2.0 - 1.0).abs();
            if val <= size_limit {
                input.palette_entries[which_gradient % input.palette_entries.len()].gradient_color(
                    dist,
                    current_gradient,
                    true,
                )
            } else {
                Rgb::BLACK
            }
        })
        .collect()
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "Splat preview clamps normalized brightness before RGB scaling"
)]
fn splat_frame(input: VisualizerInput) -> Vec<Rgb> {
    if input.animation_frame == 0 {
        return vec![Rgb::BLACK; DOME_PIXELS];
    }

    let adjusted_level = f64::from(input.volume.clamp(0.0, 1.0))
        .sqrt()
        .clamp(0.1, 1.0);
    let points = DOME_LED_POINTS.get_or_init(build_dome_led_points);
    let splats = [
        Splat {
            center_x: 0.22,
            center_y: 0.34,
            phase_offset: 0.00,
            color_index: 0,
        },
        Splat {
            center_x: 0.74,
            center_y: 0.42,
            phase_offset: 0.31,
            color_index: 3,
        },
        Splat {
            center_x: 0.46,
            center_y: 0.76,
            phase_offset: 0.63,
            color_index: 6,
        },
    ];

    points
        .iter()
        .map(|point| {
            let mut color = Rgb::BLACK;
            for splat in splats {
                let age = (input.beat_progress + splat.phase_offset).rem_euclid(1.0);
                let radius = adjusted_level * (0.06 + 0.24 * age);
                let distance = distance2(point.x, point.y, splat.center_x, splat.center_y);
                if distance < radius {
                    let falloff = 1.0 - distance / radius;
                    let fade = (1.0 - age).powi(2);
                    let strength = (falloff * fade).clamp(0.0, 1.0) as f32;
                    color = light_paint(
                        color,
                        input.palette[splat.color_index % input.palette.len()].scale(strength),
                    );
                }
            }
            color
        })
        .collect()
}

#[derive(Clone, Copy)]
struct Splat {
    center_x: f64,
    center_y: f64,
    phase_offset: f64,
    color_index: usize,
}

fn race_frame(input: VisualizerInput) -> Vec<Rgb> {
    let offset = phase_offset(input.beat_progress);
    preview_frame(|index| {
        let distance = (index + DOME_PIXELS - offset) % DOME_PIXELS;
        if distance < 320 {
            input.palette[2]
        } else if distance < 640 {
            input.palette[1].scale(0.45)
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
            input.palette[(index / 377) % input.palette.len()]
        } else if lane < 9 {
            input.palette[(index / 233 + 1) % input.palette.len()].scale(0.6)
        } else {
            Rgb::BLACK
        }
    })
}

fn quaternion_test_frame(input: VisualizerInput) -> Vec<Rgb> {
    let orientation = input.orientation_override.map_or(
        Quaternion {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
        |orientation| {
            Quaternion::from_yaw_pitch_roll(orientation.yaw, orientation.pitch, orientation.roll)
        },
    );
    DOME_LED_POINTS
        .get_or_init(build_dome_led_points)
        .iter()
        .map(|point| {
            let (x, y, z) = spectrum_quaternion_test_point(point.x, point.y);
            let (x, y, z) = orientation.transform_vector(x, y, z);
            match max_axis_by_abs(x, y, z) {
                0 => Rgb::from_u24(0xff_00_00),
                1 => Rgb::from_u24(0x00_ff_00),
                _ => Rgb::from_u24(0x00_00_ff),
            }
        })
        .collect()
}

fn quaternion_multi_test_frame(_input: VisualizerInput) -> Vec<Rgb> {
    vec![Rgb::BLACK; DOME_PIXELS]
}

fn quaternion_paintbrush_frame(input: VisualizerInput) -> Vec<Rgb> {
    // Spectrum's paintbrush idles by integrating yaw/pitch/roll momentum. This
    // stateless port derives a long-period equivalent from runtime frame time so
    // the brush follows a wandering path instead of one fixed beat loop.
    let orientation = input.orientation_override.map_or_else(
        || idle_paintbrush_orientation(input),
        |orientation| {
            Quaternion::from_yaw_pitch_roll(orientation.yaw, orientation.pitch, orientation.roll)
        },
    );
    let threshold_factor = 0.25 + f64::from(input.volume.clamp(0.0, 1.0)) + 0.01;
    let threshold = 2.0 / threshold_factor;
    let saturation = (1.3 / f64::from(input.volume.max(0.01)) - 1.0).clamp(0.2, 1.0);
    let contour_counter = (4.0 * f64::from(input.volume) * paintbrush_time(input)) % 100.0;

    DOME_LED_POINTS
        .get_or_init(build_dome_led_points)
        .iter()
        .map(|point| {
            let (x, y, z) = hemisphere_point(point.x, point.y);
            let (rx, ry, rz) = orientation.transform_vector(x, y, z);
            let distance = distance3(rx, ry, rz, -1.0, 0.0, 0.0).max(0.001);
            let neg_distance = distance3(rx, ry, rz, 1.0, 0.0, 0.0).max(0.001);
            let potential = 1.0 / (distance * neg_distance);
            let strength = potential - threshold;
            let hue = (1.0 + orientation.w) / 2.0;

            let mut color = Rgb::BLACK;
            if strength > 0.0 {
                color = hsv_to_rgb(hue, saturation, 1.0);
            }

            let potential_contours =
                (1000.0 * (potential - 0.5)).max(0.001).ln() + contour_counter / 100.0;
            let contour_value = potential_contours - potential_contours.trunc();
            if contour_value < 0.2 {
                let bracket = potential_contours.trunc();
                let value = 0.8 - (1.0 - bracket / 10.0).clamp(0.0, 0.8);
                color = light_paint(color, hsv_to_rgb(hue, 0.4, value.clamp(0.0, 1.0)));
            }

            // Keep the operator palette present in this port: the metaball
            // drives shape/brightness while the active palette colors tint dim
            // regions so the mode still responds to palette selection.
            if color == Rgb::BLACK && (point.index + phase_offset(input.beat_progress)) % 97 < 8 {
                input.palette[(point.index / 377) % input.palette.len()].scale(0.18)
            } else {
                color
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct Quaternion {
    x: f64,
    y: f64,
    z: f64,
    w: f64,
}

impl Quaternion {
    fn from_yaw_pitch_roll(yaw: f64, pitch: f64, roll: f64) -> Self {
        let (half_yaw_sin, half_yaw_cos) = (yaw * 0.5).sin_cos();
        let (half_pitch_sin, half_pitch_cos) = (pitch * 0.5).sin_cos();
        let (half_roll_sin, half_roll_cos) = (roll * 0.5).sin_cos();
        Self {
            x: half_yaw_cos.mul_add(
                half_pitch_sin * half_roll_cos,
                half_yaw_sin * half_pitch_cos * half_roll_sin,
            ),
            y: half_yaw_sin.mul_add(
                half_pitch_cos * half_roll_cos,
                -half_yaw_cos * half_pitch_sin * half_roll_sin,
            ),
            z: half_yaw_cos.mul_add(
                half_pitch_cos * half_roll_sin,
                -half_yaw_sin * half_pitch_sin * half_roll_cos,
            ),
            w: half_yaw_cos.mul_add(
                half_pitch_cos * half_roll_cos,
                half_yaw_sin * half_pitch_sin * half_roll_sin,
            ),
        }
        .normalize()
    }

    fn normalize(self) -> Self {
        let length = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if length <= f64::EPSILON {
            return Self {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            };
        }
        Self {
            x: self.x / length,
            y: self.y / length,
            z: self.z / length,
            w: self.w / length,
        }
    }

    fn transform_vector(self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let qx2 = self.x + self.x;
        let qy2 = self.y + self.y;
        let qz2 = self.z + self.z;
        let wx2 = self.w * qx2;
        let wy2 = self.w * qy2;
        let wz2 = self.w * qz2;
        let xx2 = self.x * qx2;
        let xy2 = self.x * qy2;
        let xz2 = self.x * qz2;
        let yy2 = self.y * qy2;
        let yz2 = self.y * qz2;
        let zz2 = self.z * qz2;
        (
            (1.0 - yy2 - zz2).mul_add(x, (xy2 - wz2).mul_add(y, (xz2 + wy2) * z)),
            (xy2 + wz2).mul_add(x, (1.0 - xx2 - zz2).mul_add(y, (yz2 - wx2) * z)),
            (xz2 - wy2).mul_add(x, (yz2 + wx2).mul_add(y, (1.0 - xx2 - yy2) * z)),
        )
    }
}

fn idle_paintbrush_orientation(input: VisualizerInput) -> Quaternion {
    let time = paintbrush_time(input);
    let level = f64::from(input.volume.clamp(0.0, 1.0));
    let speed = 0.65 + level;
    let yaw = std::f64::consts::TAU
        * (0.18 * speed * time + 0.08 * (0.73 * time).sin() + 0.03 * (1.91 * time).sin());
    let pitch = std::f64::consts::TAU
        * (-0.25 + 0.10 * (0.47 * time + 0.4).sin() + 0.035 * (1.37 * time).cos());
    let roll =
        std::f64::consts::TAU * (0.11 * (0.31 * time + 1.7).sin() + 0.05 * (1.13 * time).sin());
    Quaternion::from_yaw_pitch_roll(yaw, pitch, roll)
}

fn paintbrush_time(input: VisualizerInput) -> f64 {
    let frame_in_cycle: u32 = (input.animation_frame % 57_600)
        .try_into()
        .expect("paintbrush animation cycle fits in u32");
    f64::from(frame_in_cycle) / 120.0 + input.beat_progress.rem_euclid(1.0)
}

fn spectrum_quaternion_test_point(normalized_x: f64, normalized_y: f64) -> (f64, f64, f64) {
    let x = 2.0 * normalized_x - 1.0;
    let y = 1.0 - 2.0 * normalized_y;
    let z = (1.0 - x * x - y * y).sqrt();
    (x, y, z)
}

fn max_axis_by_abs(x: f64, y: f64, z: f64) -> u8 {
    if x.abs() > y.abs() {
        if x.abs() > z.abs() {
            0
        } else {
            2
        }
    } else if y.abs() > z.abs() {
        1
    } else {
        2
    }
}

fn map_value(
    value: f64,
    source_start: f64,
    source_end: f64,
    target_start: f64,
    target_end: f64,
) -> f64 {
    (value - source_start) * (target_end - target_start) / (source_end - source_start)
        + target_start
}

fn map_wrap(
    value: f64,
    source_start: f64,
    source_end: f64,
    target_start: f64,
    target_end: f64,
) -> f64 {
    wrap(
        map_value(value, source_start, source_end, target_start, target_end),
        target_start,
        target_end,
    )
}

fn wrap(mut value: f64, start: f64, end: f64) -> f64 {
    let range = end - start;
    while value < start {
        value += range;
    }
    while value > end {
        value -= range;
    }
    value
}

#[derive(Clone, Copy, Debug)]
struct DomeLedPoint {
    index: usize,
    x: f64,
    y: f64,
}

#[derive(Debug, Deserialize)]
struct DomeGeometryFixture {
    hand_drawn_points: Vec<GeometryPoint>,
    lines: Vec<GeometryLine>,
}

#[derive(Debug, Deserialize)]
struct GeometryPoint {
    normalized_x: f64,
    normalized_y: f64,
}

#[derive(Debug, Deserialize)]
struct GeometryLine {
    start: usize,
    end: usize,
}

#[derive(Debug, Deserialize)]
struct DomeMappingFixture {
    control_box_strut_order: Vec<Vec<String>>,
    strut_lengths: std::collections::HashMap<String, usize>,
    strut_positions: Vec<StrutPosition>,
}

#[derive(Debug, Deserialize)]
struct StrutPosition {
    control_box_strut_index: usize,
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Dome fixture LED counts are small and converted only for normalized interpolation"
)]
fn build_dome_led_points() -> Vec<DomeLedPoint> {
    let geometry: DomeGeometryFixture =
        serde_json::from_str(DOME_GEOMETRY_JSON).expect("dome geometry fixture is valid");
    let mapping: DomeMappingFixture =
        serde_json::from_str(DOME_MAPPING_JSON).expect("dome mapping fixture is valid");
    let mut points = Vec::with_capacity(DOME_PIXELS);
    for (strut_index, line) in geometry.lines.iter().enumerate() {
        let Some(start) = geometry.hand_drawn_points.get(line.start) else {
            continue;
        };
        let Some(end) = geometry.hand_drawn_points.get(line.end) else {
            continue;
        };
        let leds = mapping
            .strut_positions
            .get(strut_index)
            .map_or(0, |position| {
                mapping.strut_length(position.control_box_strut_index)
            });
        for led_index in 0..leds {
            let d = (led_index + 1) as f64 / (leds + 2) as f64;
            points.push(DomeLedPoint {
                index: points.len(),
                x: (end.normalized_x - start.normalized_x).mul_add(d, start.normalized_x),
                y: (end.normalized_y - start.normalized_y).mul_add(d, start.normalized_y),
            });
        }
    }
    points.resize(
        DOME_PIXELS,
        DomeLedPoint {
            index: 0,
            x: 0.5,
            y: 0.5,
        },
    );
    for (index, point) in points.iter_mut().enumerate() {
        point.index = index;
    }
    points
}

impl DomeMappingFixture {
    fn strut_length(&self, control_box_strut_index: usize) -> usize {
        let mut struts_left = control_box_strut_index;
        for strand in &self.control_box_strut_order {
            if strand.len() <= struts_left {
                struts_left -= strand.len();
                continue;
            }
            return self.strut_lengths[&strand[struts_left]];
        }
        0
    }
}

fn hemisphere_point(normalized_x: f64, normalized_y: f64) -> (f64, f64, f64) {
    let x = 2.0 * normalized_x - 1.0;
    let y = 1.0 - 2.0 * normalized_y;
    let z = if x.mul_add(x, y * y) > 1.0 {
        0.0
    } else {
        (1.0 - x * x - y * y).sqrt()
    };
    (x, y, z)
}

fn distance3(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> f64 {
    ((ax - bx).powi(2) + (ay - by).powi(2) + (az - bz).powi(2)).sqrt()
}

fn distance2(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    reason = "HSV channels are clamped before conversion to RGB bytes"
)]
fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> Rgb {
    let h = hue.rem_euclid(1.0) * 6.0;
    let i = h.floor() as i32;
    let f = h - f64::from(i);
    let value = value.clamp(0.0, 1.0);
    let saturation = saturation.clamp(0.0, 1.0);
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - f * saturation);
    let t = value * (1.0 - (1.0 - f) * saturation);
    let (r, g, b) = match i.rem_euclid(6) {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };
    Rgb {
        r: (255.0 * r) as u8,
        g: (255.0 * g) as u8,
        b: (255.0 * b) as u8,
    }
}

fn light_paint(base: Rgb, paint: Rgb) -> Rgb {
    if paint.r.max(paint.g).max(paint.b) > base.r.max(base.g).max(base.b) {
        paint
    } else {
        base
    }
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
fn frame_hash(commands: &[DomeCommand]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for command in commands {
        match command {
            DomeCommand::Flush => hash_byte(&mut hash, 0),
            DomeCommand::Frame(colors) => {
                hash_byte(&mut hash, 1);
                for color in colors {
                    hash_byte(&mut hash, color.r);
                    hash_byte(&mut hash, color.g);
                    hash_byte(&mut hash, color.b);
                }
            }
            DomeCommand::Pixel {
                strut_index,
                led_index,
                color,
            } => {
                hash_byte(&mut hash, 2);
                hash_usize(&mut hash, *strut_index);
                hash_usize(&mut hash, *led_index);
                hash_byte(&mut hash, color.r);
                hash_byte(&mut hash, color.g);
                hash_byte(&mut hash, color.b);
            }
        }
    }
    hash
}

#[cfg(test)]
fn bar_frame_hash(commands: &[BarCommand]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for command in commands {
        match command {
            BarCommand::Flush => hash_byte(&mut hash, 0),
            BarCommand::Pixel {
                is_runner,
                led_index,
                color,
            } => {
                hash_byte(&mut hash, 2);
                hash_byte(&mut hash, u8::from(*is_runner));
                hash_usize(&mut hash, *led_index);
                hash_byte(&mut hash, color.r);
                hash_byte(&mut hash, color.g);
                hash_byte(&mut hash, color.b);
            }
        }
    }
    hash
}

#[cfg(test)]
fn stage_frame_hash(commands: &[StageCommand]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for command in commands {
        match command {
            StageCommand::Flush => hash_byte(&mut hash, 0),
            StageCommand::Pixel {
                side_index,
                led_index,
                layer_index,
                color,
            } => {
                hash_byte(&mut hash, 2);
                hash_usize(&mut hash, *side_index);
                hash_usize(&mut hash, *led_index);
                hash_usize(&mut hash, *layer_index);
                hash_byte(&mut hash, color.r);
                hash_byte(&mut hash, color.g);
                hash_byte(&mut hash, color.b);
            }
        }
    }
    hash
}

#[cfg(test)]
fn hash_byte(hash: &mut u64, byte: u8) {
    *hash ^= u64::from(byte);
    *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
}

#[cfg(test)]
fn hash_usize(hash: &mut u64, value: usize) {
    for byte in value.to_le_bytes() {
        hash_byte(hash, byte);
    }
}

#[cfg(test)]
mod tests {
    use domers_core::import_spectrum_xml;
    use domers_outputs::{topology::DOME_PIXELS, DomeCommand};
    use serde::Deserialize;

    use super::{
        bar_frame_hash, frame_hash, render_bar_diagnostic, render_dome_diagnostic,
        render_dome_visualizer, render_stage_visualizer, render_stage_visualizer_with_input,
        stage_frame_hash, BarDiagnosticVisualizer, Classification, DiagnosticInput,
        DomeDiagnosticVisualizer, LiveVisualizer, OrientationOverride, StageVisualizer,
        StageVisualizerInput, VisualizerInput, INVENTORY,
    };

    #[derive(Deserialize)]
    struct VisualizerManifest {
        cases: Vec<VisualizerCase>,
    }

    #[derive(Deserialize)]
    struct VisualizerCase {
        case: String,
        name: String,
        expected: ExpectedHash,
        input: ManifestInput,
    }

    #[derive(Deserialize)]
    struct ExpectedHash {
        status: String,
        value: String,
    }

    #[derive(Clone, Copy, Deserialize)]
    struct ManifestInput {
        volume: f32,
        beat_progress: f64,
        flash_active: bool,
        diagnostic_state: u8,
        diagnostic_step: usize,
        palette_slot: u8,
    }

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
    fn spectrum_visualizer_fixture_manifest_covers_inventory() {
        let manifest =
            include_str!("../../../fixtures/spectrum-csharp/visualizer_frame_cases.json");
        for visualizer in INVENTORY {
            assert!(
                manifest.contains(&format!("\"name\": \"{}\"", visualizer.name)),
                "{} should have a source-traceable fixture case",
                visualizer.name
            );
            assert!(
                manifest.contains(&format!(
                    "spectrum/Spectrum/Visualizers/{}.cs",
                    visualizer.name
                )),
                "{} should cite its Spectrum source file",
                visualizer.name
            );
        }
        assert_eq!(
            manifest.matches("\"source_sha256\"").count(),
            INVENTORY.len()
        );
        assert_eq!(
            manifest.matches("\"status\": \"captured\"").count(),
            INVENTORY.len()
        );
        assert!(!manifest.contains("\"pending_csharp_execution\""));
        assert!(!manifest.contains("\"value\": null"));
    }

    #[test]
    #[ignore = "run explicitly while closing Spectrum visualizer exactness gaps"]
    fn rust_visualizer_hashes_match_spectrum_csharp_goldens() {
        let manifest: VisualizerManifest = serde_json::from_str(include_str!(
            "../../../fixtures/spectrum-csharp/visualizer_frame_cases.json"
        ))
        .expect("visualizer manifest parses");
        let spectrum_config = import_spectrum_xml(include_str!(
            "../../../fixtures/config/spectrum_default_config.xml"
        ))
        .config;
        let mut mismatches = Vec::new();

        for test_case in &manifest.cases {
            assert_eq!(
                test_case.expected.status, "captured",
                "{} must have captured Spectrum hash",
                test_case.name
            );
            let expected = test_case
                .expected
                .value
                .parse::<u64>()
                .expect("expected hash is u64");
            let actual = render_manifest_case_hash(test_case, &spectrum_config);
            if actual != expected {
                mismatches.push(format!(
                    "{} / {}: expected {expected}, got {actual}",
                    test_case.case, test_case.name
                ));
            }
        }

        assert!(
            mismatches.is_empty(),
            "Rust visualizer hashes differ from Spectrum C# goldens:\n{}",
            mismatches.join("\n")
        );
    }

    fn render_manifest_case_hash(
        test_case: &VisualizerCase,
        config: &domers_core::DomersConfig,
    ) -> u64 {
        let live_input = visualizer_input(test_case.input, config);
        let diagnostic_input = DiagnosticInput {
            state: test_case.input.diagnostic_state,
            step: test_case.input.diagnostic_step,
            brightness: 1.0,
            volume: test_case.input.volume,
            beat_progress: test_case.input.beat_progress,
        };
        match test_case.name.as_str() {
            "LEDDomeStrutIterationDiagnosticVisualizer" => frame_hash(&render_dome_diagnostic(
                DomeDiagnosticVisualizer::StrutIteration,
                diagnostic_input,
            )),
            "LEDDomeFlashColorsDiagnosticVisualizer" => frame_hash(&render_dome_diagnostic(
                DomeDiagnosticVisualizer::FlashColors,
                diagnostic_input,
            )),
            "LEDDomeStrandTestDiagnosticVisualizer" => frame_hash(&render_dome_diagnostic(
                DomeDiagnosticVisualizer::StrandTest,
                diagnostic_input,
            )),
            "LEDDomeFullColorFlashDiagnosticVisualizer" => frame_hash(&render_dome_diagnostic(
                DomeDiagnosticVisualizer::FullColorFlash,
                diagnostic_input,
            )),
            "LEDDomeVolumeVisualizer" => {
                frame_hash(&render_dome_visualizer(LiveVisualizer::Volume, live_input))
            }
            "LEDDomeRadialVisualizer" => {
                frame_hash(&render_dome_visualizer(LiveVisualizer::Radial, live_input))
            }
            "LEDDomeRaceVisualizer" => {
                frame_hash(&render_dome_visualizer(LiveVisualizer::Race, live_input))
            }
            "LEDDomeSnakesVisualizer" => {
                frame_hash(&render_dome_visualizer(LiveVisualizer::Snakes, live_input))
            }
            "LEDDomeSplatVisualizer" => {
                frame_hash(&render_dome_visualizer(LiveVisualizer::Splat, live_input))
            }
            "LEDDomeQuaternionTestVisualizer" => frame_hash(&render_dome_visualizer(
                LiveVisualizer::QuaternionTest,
                live_input,
            )),
            "LEDDomeQuaternionMultiTestVisualizer" => frame_hash(&render_dome_visualizer(
                LiveVisualizer::QuaternionMultiTest,
                live_input,
            )),
            "LEDDomeQuaternionPaintbrushVisualizer" => frame_hash(&render_dome_visualizer(
                LiveVisualizer::QuaternionPaintbrush,
                live_input,
            )),
            "LEDDomeTVStaticVisualizer" => frame_hash(&render_dome_visualizer(
                LiveVisualizer::TvStatic,
                live_input,
            )),
            "LEDDomeFlashVisualizer" => {
                frame_hash(&render_dome_visualizer(LiveVisualizer::Flash, live_input))
            }
            "LEDBarFlashColorsDiagnosticVisualizer" => bar_frame_hash(&render_bar_diagnostic(
                BarDiagnosticVisualizer::FlashColors,
                diagnostic_input,
                config.bar.infinity_width as usize,
                config.bar.infinity_length as usize,
                config.bar.runner_length as usize,
            )),
            "LEDStageFlashColorsDiagnosticVisualizer" => {
                stage_frame_hash(&render_stage_visualizer(
                    StageVisualizer::FlashColorsDiagnostic,
                    diagnostic_input,
                    &stage_side_lengths(config),
                ))
            }
            "LEDStageDepthLevelVisualizer" => {
                stage_frame_hash(&render_stage_visualizer_with_input(
                    StageVisualizer::DepthLevel,
                    StageVisualizerInput {
                        diagnostic: diagnostic_input,
                        color_palette: config.color_palette.clone(),
                        color_palette_index: test_case.input.palette_slot,
                        stage_brightness: 1.0,
                    },
                    &stage_side_lengths(config),
                ))
            }
            name => panic!("unhandled visualizer manifest case {name}"),
        }
    }

    fn visualizer_input(
        input: ManifestInput,
        config: &domers_core::DomersConfig,
    ) -> VisualizerInput {
        let palette = std::array::from_fn(|index| {
            config.color_palette.single_color(index, input.palette_slot)
        });
        let palette_entries = std::array::from_fn(|index| {
            config
                .color_palette
                .entry(domers_core::ColorPalette::absolute_index(
                    index,
                    input.palette_slot,
                ))
        });
        VisualizerInput {
            volume: input.volume,
            beat_progress: input.beat_progress,
            animation_frame: 0,
            orientation_override: None,
            flash_active: input.flash_active,
            primary: palette[0],
            secondary: palette[1],
            accent: palette[2],
            palette,
            palette_entries,
        }
    }

    fn stage_side_lengths(config: &domers_core::DomersConfig) -> Vec<usize> {
        config
            .stage
            .side_lengths
            .iter()
            .map(|length| *length as usize)
            .collect()
    }

    #[test]
    fn every_initial_live_dome_visualizer_produces_a_simulator_frame() {
        for visualizer in [
            LiveVisualizer::TvStatic,
            LiveVisualizer::Volume,
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

        assert!(
            render_dome_visualizer(LiveVisualizer::Flash, VisualizerInput::default()).is_empty(),
            "Flash is event-driven and has no first-frame output without an active animation"
        );
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

    #[test]
    fn splat_preview_renders_fading_blobs() {
        let commands = render_dome_visualizer(
            LiveVisualizer::Splat,
            VisualizerInput {
                animation_frame: 120,
                ..VisualizerInput::default()
            },
        );
        let frame = commands
            .iter()
            .find_map(|command| match command {
                DomeCommand::Frame(colors) => Some(colors),
                DomeCommand::Flush | DomeCommand::Pixel { .. } => None,
            })
            .expect("splat visualizer should write a whole preview frame");

        assert_eq!(frame.len(), DOME_PIXELS);
        assert!(
            frame
                .iter()
                .filter(|color| **color != domers_core::Rgb::BLACK)
                .count()
                > 100
        );
    }

    #[test]
    fn tv_static_uses_deterministic_varied_noise() {
        let input = VisualizerInput {
            volume: 0.5,
            beat_progress: 0.1,
            ..VisualizerInput::default()
        };
        let first = render_dome_visualizer(LiveVisualizer::TvStatic, input);
        let second = render_dome_visualizer(LiveVisualizer::TvStatic, input);
        let changed = render_dome_visualizer(
            LiveVisualizer::TvStatic,
            VisualizerInput {
                beat_progress: 0.2,
                ..input
            },
        );

        assert_eq!(first, second);
        assert_ne!(first, changed);
        let frame = first
            .iter()
            .find_map(|command| match command {
                DomeCommand::Frame(colors) => Some(colors),
                DomeCommand::Flush | DomeCommand::Pixel { .. } => None,
            })
            .expect("tv static should write a frame");
        assert!(frame.windows(2).take(100).any(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn quaternion_paintbrush_idle_path_uses_animation_frame() {
        let input = VisualizerInput {
            volume: 0.6,
            beat_progress: 0.25,
            animation_frame: 0,
            ..VisualizerInput::default()
        };
        let later = VisualizerInput {
            animation_frame: 360,
            ..input
        };

        assert_ne!(
            super::frame_hash(&render_dome_visualizer(
                LiveVisualizer::QuaternionPaintbrush,
                input
            )),
            super::frame_hash(&render_dome_visualizer(
                LiveVisualizer::QuaternionPaintbrush,
                later
            )),
            "idle paintbrush should not retrace a constant path when beat phase is unchanged"
        );
    }

    #[test]
    fn quaternion_paintbrush_uses_orientation_override() {
        let input = VisualizerInput {
            volume: 0.6,
            beat_progress: 0.25,
            animation_frame: 120,
            ..VisualizerInput::default()
        };
        let overridden = VisualizerInput {
            orientation_override: Some(OrientationOverride {
                yaw: std::f64::consts::FRAC_PI_2,
                pitch: -std::f64::consts::FRAC_PI_4,
                roll: 0.0,
            }),
            ..input
        };

        assert_ne!(
            super::frame_hash(&render_dome_visualizer(
                LiveVisualizer::QuaternionPaintbrush,
                input
            )),
            super::frame_hash(&render_dome_visualizer(
                LiveVisualizer::QuaternionPaintbrush,
                overridden
            )),
            "manual simulator orientation should steer orientation visualizers"
        );
    }

    #[test]
    fn live_visualizer_frame_hashes_are_stable() {
        let cases = [
            (LiveVisualizer::TvStatic, 14_075_851_066_622_254_809),
            (LiveVisualizer::Volume, 15_270_928_452_629_649_531),
            (LiveVisualizer::Flash, 14_695_981_039_346_656_037),
            (LiveVisualizer::Radial, 8_095_729_372_390_775_204),
            (LiveVisualizer::Splat, 12_459_070_695_921_506_308),
            (LiveVisualizer::Race, 6_816_113_448_421_016_324),
            (LiveVisualizer::Snakes, 2_228_629_276_110_457_077),
            (LiveVisualizer::QuaternionTest, 1_564_991_241_466_880_178),
            (
                LiveVisualizer::QuaternionMultiTest,
                12_459_070_695_921_506_308,
            ),
            (
                LiveVisualizer::QuaternionPaintbrush,
                7_177_837_735_347_917_156,
            ),
        ];
        let actual: Vec<_> = cases
            .iter()
            .map(|(visualizer, _expected)| {
                let commands = render_dome_visualizer(*visualizer, VisualizerInput::default());
                (*visualizer, super::frame_hash(&commands))
            })
            .collect();
        let expected: Vec<_> = cases.into_iter().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn live_visualizers_consume_full_palette_bank() {
        let mut custom = VisualizerInput::default();
        custom.palette[3] = domers_core::Rgb::from_u24(0x11_22_33);
        custom.palette[4] = domers_core::Rgb::from_u24(0x44_55_66);
        custom.palette[5] = domers_core::Rgb::from_u24(0x77_88_99);
        custom.palette[6] = domers_core::Rgb::from_u24(0xaa_bb_cc);
        custom.palette_entries[4] = domers_core::PaletteEntry::solid(0x44_55_66);
        custom.palette_entries[5] = domers_core::PaletteEntry::solid(0x77_88_99);
        custom.palette_entries[6] = domers_core::PaletteEntry::solid(0xaa_bb_cc);

        for visualizer in [
            LiveVisualizer::Volume,
            LiveVisualizer::Radial,
            LiveVisualizer::Snakes,
            LiveVisualizer::QuaternionPaintbrush,
        ] {
            assert_ne!(
                super::frame_hash(&render_dome_visualizer(
                    visualizer,
                    VisualizerInput::default()
                )),
                super::frame_hash(&render_dome_visualizer(visualizer, custom)),
                "{visualizer:?} should use palette entries beyond Color 1-3"
            );
        }
    }

    #[test]
    fn used_dome_diagnostics_produce_frames() {
        for visualizer in [
            DomeDiagnosticVisualizer::FlashColors,
            DomeDiagnosticVisualizer::StrutIteration,
            DomeDiagnosticVisualizer::StrandTest,
            DomeDiagnosticVisualizer::FullColorFlash,
        ] {
            let commands = render_dome_diagnostic(visualizer, DiagnosticInput::default());
            let pixels = commands
                .iter()
                .filter(|command| matches!(command, DomeCommand::Pixel { .. }))
                .count();
            assert!(pixels > 0, "diagnostic should write pixels");
            assert!(commands
                .iter()
                .any(|command| matches!(command, DomeCommand::Flush)));
        }
    }

    #[test]
    fn used_bar_diagnostic_covers_runner_and_infinity() {
        let commands = render_bar_diagnostic(
            BarDiagnosticVisualizer::FlashColors,
            DiagnosticInput::default(),
            4,
            6,
            5,
        );

        assert!(commands.iter().any(|command| matches!(
            command,
            domers_outputs::BarCommand::Pixel {
                is_runner: false,
                ..
            }
        )));
        assert!(commands.iter().any(|command| matches!(
            command,
            domers_outputs::BarCommand::Pixel {
                is_runner: true,
                ..
            }
        )));
        assert!(commands
            .iter()
            .any(|command| matches!(command, domers_outputs::BarCommand::Flush)));
    }

    #[test]
    fn used_stage_visualizers_produce_layered_pixels() {
        for visualizer in [
            StageVisualizer::FlashColorsDiagnostic,
            StageVisualizer::DepthLevel,
        ] {
            let commands =
                render_stage_visualizer(visualizer, DiagnosticInput::default(), &[3, 4, 5]);
            assert!(commands.iter().any(|command| matches!(
                command,
                domers_outputs::StageCommand::Pixel { layer_index: 2, .. }
            )));
            assert!(commands
                .iter()
                .any(|command| matches!(command, domers_outputs::StageCommand::Flush)));
        }
    }

    #[test]
    fn stage_tracer_index_matches_spectrum_side_progression() {
        let side_lengths = [10, 20, 30];

        assert_eq!(super::stage_tracer_led_index(&side_lengths, 0, 0.0), 0);
        assert_eq!(super::stage_tracer_led_index(&side_lengths, 0, 0.25), 7);
        assert_eq!(super::stage_tracer_led_index(&side_lengths, 0, 0.5), 20);
        assert_eq!(super::stage_tracer_led_index(&side_lengths, 0, 0.75), 37);
    }

    #[test]
    fn stage_depth_level_emits_layered_pixels() {
        let commands = render_stage_visualizer(
            StageVisualizer::DepthLevel,
            DiagnosticInput {
                beat_progress: 0.0,
                volume: 1.0,
                ..DiagnosticInput::default()
            },
            &[10, 20, 30],
        );

        assert!(commands.iter().any(|command| matches!(
            command,
            domers_outputs::StageCommand::Pixel {
                side_index: 0,
                led_index: 0,
                layer_index: 0,
                ..
            }
        )));
        assert!(commands
            .iter()
            .any(|command| matches!(command, domers_outputs::StageCommand::Flush)));
    }
}
