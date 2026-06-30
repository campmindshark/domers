//! Visualizer inventory and porting order.

use domers_core::Rgb;
use domers_outputs::{
    topology::{DOME_PIXELS, DOME_STRUTS, STAGE_LAYERS},
    BarCommand, DomeCommand, DomeOutputSink, StageCommand,
};

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
}

impl Default for VisualizerInput {
    fn default() -> Self {
        let primary = Rgb::from_u24(0x00_ff_00);
        let secondary = Rgb::from_u24(0x00_80_ff);
        let accent = Rgb::from_u24(0xff_40_80);
        Self {
            volume: 0.5,
            beat_progress: 0.25,
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

/// Render one used dome diagnostic visualizer frame.
#[must_use]
pub fn render_dome_diagnostic(
    visualizer: DomeDiagnosticVisualizer,
    input: DiagnosticInput,
) -> Vec<DomeCommand> {
    let colors = match visualizer {
        DomeDiagnosticVisualizer::FlashColors => dome_flash_colors_frame(input),
        DomeDiagnosticVisualizer::StrutIteration => dome_strut_iteration_frame(input),
        DomeDiagnosticVisualizer::StrandTest | DomeDiagnosticVisualizer::FullColorFlash => {
            dome_on_off_frame(input.state, white(input.brightness))
        }
    };
    vec![DomeCommand::Frame(colors), DomeCommand::Flush]
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
    match visualizer {
        StageVisualizer::FlashColorsDiagnostic => stage_flash_colors(input, side_lengths),
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

fn dome_on_off_frame(state: u8, color: Rgb) -> Vec<Rgb> {
    if state == 0 {
        vec![Rgb::BLACK; DOME_PIXELS]
    } else {
        vec![color; DOME_PIXELS]
    }
}

fn dome_flash_colors_frame(input: DiagnosticInput) -> Vec<Rgb> {
    if input.state == 0 {
        return vec![Rgb::BLACK; DOME_PIXELS];
    }

    let palette = diagnostic_colors(input.brightness);
    preview_frame(|index| {
        let strut = (index * DOME_STRUTS) / DOME_PIXELS;
        let color = palette[strut % palette.len()];
        if input.state == 2 && index % 40 != 0 {
            Rgb::BLACK
        } else {
            color
        }
    })
}

fn dome_strut_iteration_frame(input: DiagnosticInput) -> Vec<Rgb> {
    let mut frame = vec![Rgb::from_u24(0x00_00_ff).scale(input.brightness); DOME_PIXELS];
    let strut = input.step % DOME_STRUTS;
    let start = (strut * DOME_PIXELS) / DOME_STRUTS;
    let end = ((strut + 1) * DOME_PIXELS) / DOME_STRUTS;
    let color_cycle = [
        Rgb::from_u24(0xff_00_00),
        Rgb::from_u24(0x00_ff_00),
        Rgb::from_u24(0x00_00_ff),
        Rgb::from_u24(0xff_ff_ff),
    ];
    let highlight =
        color_cycle[(input.step / DOME_STRUTS) % color_cycle.len()].scale(input.brightness);
    for color in &mut frame[start..end] {
        *color = highlight;
    }
    frame
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
    let mut color_index = 0;
    for (side_index, side_length) in side_lengths.iter().copied().enumerate() {
        for layer_index in 0..STAGE_LAYERS {
            for led_index in 0..side_length {
                let color = if input.state == 0
                    || (input.state == 2 && led_index != 0 && led_index + 1 != side_length)
                {
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
fn stage_depth_level(input: DiagnosticInput, side_lengths: &[usize]) -> Vec<StageCommand> {
    let mut commands = Vec::new();
    let colors = diagnostic_colors(input.brightness);
    let triangles = side_lengths.len() / 3;
    for triangle_index in 0..triangles {
        let tracer_index =
            stage_tracer_led_index(side_lengths, triangle_index, input.beat_progress);
        let max_triangle_counter = triangle_length(side_lengths, triangle_index);
        let mut triangle_counter = 0;
        for side_offset in 0..3 {
            let side_index = triangle_index * 3 + side_offset;
            let side_length = side_lengths[side_index];
            for led_index in 0..side_length {
                let second_part = stage_second_part(side_index) ^ (input.beat_progress > 0.5);
                let base = if second_part { colors[1] } else { colors[0] };
                let color = stage_depth_color(
                    base,
                    colors[2],
                    triangle_counter,
                    max_triangle_counter,
                    tracer_index,
                    input.volume,
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
fn stage_depth_color(
    base: Rgb,
    tracer: Rgb,
    triangle_counter: usize,
    max_triangle_counter: usize,
    tracer_index: usize,
    volume: f32,
) -> Rgb {
    let pixel_pos = triangle_counter as f64 / max_triangle_counter.max(1) as f64;
    let focus_pos = tracer_index as f64 / max_triangle_counter.max(1) as f64;
    let distance = (pixel_pos - focus_pos).abs().clamp(0.0, 1.0);
    blend(base, tracer, 1.0 - distance).scale(volume)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "RGB blend clamps normalized channels before conversion"
)]
fn blend(a: Rgb, b: Rgb, amount_b: f64) -> Rgb {
    let amount_b = amount_b.clamp(0.0, 1.0);
    let amount_a = 1.0 - amount_b;
    let channel = |left: u8, right: u8| {
        (f64::from(left) * amount_a + f64::from(right) * amount_b)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Rgb {
        r: channel(a.r, b.r),
        g: channel(a.g, b.g),
        b: channel(a.b, b.b),
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
    preview_frame(|index| input.palette[((index + offset) / 89) % input.palette.len()])
}

fn splat_frame(input: VisualizerInput) -> Vec<Rgb> {
    preview_frame(|index| {
        if index % 11 == 0 || index % 17 == 0 {
            input.palette[(index / 11) % input.palette.len()]
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
    preview_frame(|index| {
        if index % 8 < 4 {
            input.palette[1]
        } else {
            input.palette[0].scale(0.3)
        }
    })
}

fn quaternion_multi_test_frame(input: VisualizerInput) -> Vec<Rgb> {
    preview_frame(|index| input.palette[index % input.palette.len()])
}

fn quaternion_paintbrush_frame(input: VisualizerInput) -> Vec<Rgb> {
    let offset = phase_offset(input.beat_progress);
    preview_frame(|index| {
        if (index + offset) % 13 < 6 {
            input.palette[(index / 13 + 2) % input.palette.len()]
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
    use domers_outputs::{topology::DOME_PIXELS, DomeCommand};

    use super::{
        render_bar_diagnostic, render_dome_diagnostic, render_dome_visualizer,
        render_stage_visualizer, BarDiagnosticVisualizer, Classification, DiagnosticInput,
        DomeDiagnosticVisualizer, LiveVisualizer, StageVisualizer, VisualizerInput, INVENTORY,
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
            manifest.matches("\"pending_csharp_execution\"").count(),
            INVENTORY.len()
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
    fn live_visualizer_frame_hashes_are_stable() {
        let cases = [
            (LiveVisualizer::TvStatic, 14_075_851_066_622_254_809),
            (LiveVisualizer::Volume, 5_403_355_765_041_486_106),
            (LiveVisualizer::Flash, 17_092_067_869_950_253_262),
            (LiveVisualizer::Radial, 1_809_576_378_694_742_732),
            (LiveVisualizer::Splat, 6_261_929_961_458_295_948),
            (LiveVisualizer::Race, 12_074_785_084_243_685_636),
            (LiveVisualizer::Snakes, 9_672_234_594_085_961_109),
            (LiveVisualizer::QuaternionTest, 17_270_531_847_863_315_960),
            (
                LiveVisualizer::QuaternionMultiTest,
                5_298_449_737_626_868_325,
            ),
            (
                LiveVisualizer::QuaternionPaintbrush,
                6_740_275_545_131_552_642,
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

        for visualizer in [
            LiveVisualizer::Volume,
            LiveVisualizer::Radial,
            LiveVisualizer::Splat,
            LiveVisualizer::Snakes,
            LiveVisualizer::QuaternionMultiTest,
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
            let frame = commands
                .iter()
                .find_map(|command| match command {
                    DomeCommand::Frame(colors) => Some(colors),
                    DomeCommand::Flush | DomeCommand::Pixel { .. } => None,
                })
                .expect("diagnostic should write a frame");
            assert_eq!(frame.len(), DOME_PIXELS);
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
    fn stage_depth_level_focuses_color_around_tracer_led() {
        let commands = render_stage_visualizer(
            StageVisualizer::DepthLevel,
            DiagnosticInput {
                beat_progress: 0.0,
                volume: 1.0,
                ..DiagnosticInput::default()
            },
            &[10, 20, 30],
        );

        let focused = commands.iter().find_map(|command| match command {
            domers_outputs::StageCommand::Pixel {
                side_index: 0,
                led_index: 0,
                layer_index: 0,
                color,
            } => Some(*color),
            _ => None,
        });
        let distant = commands.iter().find_map(|command| match command {
            domers_outputs::StageCommand::Pixel {
                side_index: 2,
                led_index: 29,
                layer_index: 0,
                color,
            } => Some(*color),
            _ => None,
        });

        assert!(
            focused.expect("focused pixel exists").b > distant.expect("distant pixel exists").b
        );
    }
}
