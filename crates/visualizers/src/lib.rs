//! Visualizer inventory and porting order.

#![allow(
    clippy::large_types_passed_by_value,
    reason = "VisualizerInput is Copy and passed by value throughout the Spectrum port"
)]

use domers_core::{ColorPalette, PaletteEntry, Rgb};
use domers_outputs::{
    dome_strut_index_for_control_box, dome_strut_length,
    topology::{DOME_PIXELS, DOME_STRUTS, STAGE_LAYERS},
    BarCommand, DomeCommand, DomeOutputSink, StageCommand,
};
use serde::Deserialize;
use std::collections::VecDeque;
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

/// Render one deterministic simulator frame for a live visualizer.
#[must_use]
pub fn render_dome_visualizer(
    visualizer: LiveVisualizer,
    input: VisualizerInput,
) -> Vec<DomeCommand> {
    if visualizer == LiveVisualizer::Flash {
        return Vec::new();
    }
    if visualizer == LiveVisualizer::TvStatic {
        return tv_static_commands(input);
    }
    if visualizer == LiveVisualizer::Snakes {
        return snakes_commands(input);
    }
    if visualizer == LiveVisualizer::Race {
        return race_commands(input);
    }
    if visualizer == LiveVisualizer::Volume {
        return volume_commands(input);
    }
    let mut sink = DomeOutputSink::new(false, true);
    sink.write_buffer(match visualizer {
        LiveVisualizer::TvStatic => unreachable!("TV Static writes Spectrum-style pixel commands"),
        LiveVisualizer::Volume => unreachable!("Volume writes Spectrum-style pixel commands"),
        LiveVisualizer::Flash => unreachable!("Flash visualizer is event-driven"),
        LiveVisualizer::Radial => radial_frame(input),
        LiveVisualizer::Splat => splat_frame(input),
        LiveVisualizer::Race => unreachable!("Race writes Spectrum-style pixel commands"),
        LiveVisualizer::Snakes => unreachable!("Snakes writes Spectrum-style pixel commands"),
        LiveVisualizer::QuaternionTest => quaternion_test_frame(input),
        LiveVisualizer::QuaternionMultiTest => quaternion_multi_test_frame(input),
        LiveVisualizer::QuaternionPaintbrush => quaternion_paintbrush_frame(input),
    });
    sink.flush();
    sink.drain_commands()
}

/// Persistent per-visualizer runtime driving the live and sandbox render loops.
///
/// Unlike [`render_dome_visualizer`], which recomputes each frame from
/// [`VisualizerInput::animation_frame`] for deterministic golden tests, this
/// runtime keeps long-lived per-visualizer state (Spectrum-style instance
/// fields) and advances it using wall-clock deltas from
/// [`VisualizerInput::now_ms`]. Switching visualizers resets that state and
/// emits a full black frame so residual pixels from the previous visualizer are
/// cleared on both the hardware channel and the browser preview.
#[derive(Clone, Debug, Default)]
pub struct VisualizerRuntime {
    active: Option<LiveVisualizer>,
    snakes: Option<SnakesRuntime>,
    race: Option<RaceRuntime>,
    radial: Option<RadialRuntime>,
    splat: Option<SplatRuntime>,
    tv_static: Option<TvStaticRuntime>,
    volume: Option<VolumeRuntime>,
    flash: Option<FlashRuntime>,
    paintbrush: Option<PaintbrushRuntime>,
}

impl VisualizerRuntime {
    /// Render the dome commands for `visualizer`, advancing persistent state.
    #[must_use]
    pub fn render_dome(
        &mut self,
        visualizer: LiveVisualizer,
        input: VisualizerInput,
    ) -> Vec<DomeCommand> {
        let switched = self.active != Some(visualizer);
        // Only wipe when replacing a *previous* visualizer; the very first
        // activation has nothing on the dome to clear and must stay bit-for-bit
        // identical to the pure first-frame path used by golden tests.
        let wipe = switched && self.active.is_some();
        if switched {
            self.reset();
            self.active = Some(visualizer);
        }

        let mut commands = Vec::new();
        if wipe {
            commands.push(DomeCommand::Frame(vec![Rgb::BLACK; DOME_PIXELS]));
        }

        match visualizer {
            LiveVisualizer::Snakes => {
                let runtime = self.snakes.get_or_insert_with(SnakesRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::Race => {
                let runtime = self.race.get_or_insert_with(RaceRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::Radial => {
                let runtime = self.radial.get_or_insert_with(RadialRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::Splat => {
                let runtime = self.splat.get_or_insert_with(SplatRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::TvStatic => {
                let runtime = self.tv_static.get_or_insert_with(TvStaticRuntime::new);
                runtime.render(&mut commands);
            }
            LiveVisualizer::Volume => {
                let runtime = self.volume.get_or_insert_with(VolumeRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::Flash => {
                let runtime = self.flash.get_or_insert_with(FlashRuntime::new);
                runtime.render(&input, &mut commands);
            }
            LiveVisualizer::QuaternionPaintbrush => {
                let runtime = self.paintbrush.get_or_insert_with(PaintbrushRuntime::new);
                runtime.render(&input, &mut commands);
            }
            other => commands.extend(render_dome_visualizer(other, input)),
        }

        commands
    }

    /// Drop all persistent per-visualizer state (invoked on visualizer switch).
    fn reset(&mut self) {
        self.snakes = None;
        self.race = None;
        self.radial = None;
        self.splat = None;
        self.tv_static = None;
        self.volume = None;
        self.flash = None;
        self.paintbrush = None;
    }
}

/// Persistent Volume runtime tracking center-offset changes (Spectrum `UpdateCenter`).
#[derive(Clone, Debug)]
struct VolumeRuntime {
    last_center_offset: Option<usize>,
    seen_frame: bool,
}

impl VolumeRuntime {
    fn new() -> Self {
        Self {
            last_center_offset: None,
            seen_frame: false,
        }
    }

    fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        let beat_progress = if input.animation_frame == 0 {
            0.0
        } else {
            input.beat_progress
        };
        let center = volume_center_offset(beat_progress);
        let center_changed = self.last_center_offset.is_some_and(|last| last != center);
        let include_initial_wipe = input.animation_frame == 0 && !self.seen_frame;
        if center_changed && self.seen_frame {
            out.extend(volume_wipe_commands());
        }
        out.extend(volume_commands_with_wipe(
            *input,
            beat_progress,
            include_initial_wipe,
        ));
        self.last_center_offset = Some(center);
        self.seen_frame = true;
    }
}

/// Persistent TV Static runtime mirroring `LEDDomeTVStaticVisualizer`, which
/// advances one long-lived `Random` across frames rather than reseeding.
#[derive(Clone, Debug)]
struct TvStaticRuntime {
    rng: DotNetRandom,
}

impl TvStaticRuntime {
    fn new() -> Self {
        Self {
            rng: DotNetRandom::new(0),
        }
    }

    fn render(&mut self, out: &mut Vec<DomeCommand>) {
        for strut_index in 0..DOME_STRUTS {
            let Some(strut_length) = dome_strut_length(strut_index) else {
                continue;
            };
            for led_index in 0..strut_length {
                out.push(DomeCommand::Pixel {
                    strut_index,
                    led_index,
                    color: self.rng.next_color(255),
                });
            }
        }
        out.push(DomeCommand::Flush);
    }
}

/// One Flash polygon shape built from a concentric strut layout around a hub.
#[derive(Clone, Debug)]
struct FlashShape {
    layout: VolumeStrutLayout,
    struts: Vec<usize>,
    animation: Option<FlashPolygonAnimation>,
}

impl FlashShape {
    const ENABLED: bool = true;

    fn enabled() -> bool {
        Self::ENABLED
    }

    fn available(&self) -> bool {
        Self::enabled() && self.animation.is_none()
    }
}

/// Persistent Flash polygon animation mirroring `PolygonAnimation`.
#[derive(Clone, Debug)]
struct FlashPolygonAnimation {
    pad: u8,
    velocity: f64,
    animation_length: u64,
    starting_time: u64,
    peak_time: u64,
    end_time: u64,
    released: bool,
}

impl FlashPolygonAnimation {
    fn new(pad: u8, velocity: f64, measure_length_ms: u32, now_ms: u64) -> Self {
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

    fn active(&self, now_ms: u64, shape_enabled: bool) -> bool {
        shape_enabled && (!self.released || self.end_time > now_ms)
    }

    fn release(&mut self, now_ms: u64) {
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
    fn intensity(&self, now_ms: u64) -> f64 {
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

/// Persistent Flash runtime mirroring `LEDDomeFlashVisualizer`.
#[derive(Clone, Debug)]
struct FlashRuntime {
    shapes: Vec<FlashShape>,
    pads_to_last_animation: [Option<usize>; 16],
    rng: DotNetRandom,
    last_user_animation_created: u64,
}

impl FlashRuntime {
    fn new() -> Self {
        let mut shapes = Vec::with_capacity(51);
        for starting_point in 20..=70 {
            let layout = concentric_layout_from_point(starting_point, 2);
            let struts = flash_layout_struts(&layout);
            shapes.push(FlashShape {
                layout,
                struts,
                animation: None,
            });
        }
        Self {
            shapes,
            pads_to_last_animation: [None; 16],
            rng: DotNetRandom::new(0),
            last_user_animation_created: 0,
        }
    }

    fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        let now_ms = input.now_ms;

        for (shape_index, shape) in self.shapes.iter_mut().enumerate() {
            let Some(animation) = shape.animation.as_ref() else {
                continue;
            };
            if animation.active(now_ms, FlashShape::enabled()) {
                continue;
            }
            if self.pads_to_last_animation[animation.pad as usize] == Some(shape_index) {
                self.pads_to_last_animation[animation.pad as usize] = None;
            }
            for &strut_index in &shape.struts {
                clear_flash_strut(strut_index, out);
            }
            shape.animation = None;
        }

        let measure_length_ms = input.measure_length_ms.unwrap_or(400);
        'midi: for note in input.midi_notes.into_iter().flatten() {
            if note.index > 15 {
                continue;
            }
            let pad = note.index as usize;
            if let Some(shape_index) = self.pads_to_last_animation[pad] {
                if let Some(animation) = self.shapes[shape_index].animation.as_mut() {
                    animation.release(now_ms);
                }
                if note.value == 0.0 {
                    continue;
                }
            }
            let Some(shape_index) = self.random_available_shape_index() else {
                break 'midi;
            };
            let starting_time = now_ms;
            self.shapes[shape_index].animation = Some(FlashPolygonAnimation::new(
                note.index,
                note.value,
                measure_length_ms,
                starting_time,
            ));
            self.pads_to_last_animation[pad] = Some(shape_index);
            self.last_user_animation_created = starting_time;
        }

        for shape in &self.shapes {
            let Some(animation) = shape.animation.as_ref() else {
                continue;
            };
            if !animation.active(now_ms, FlashShape::enabled()) {
                continue;
            }
            animate_flash_polygon(shape, animation, input, now_ms, out);
        }
    }

    fn random_available_shape_index(&mut self) -> Option<usize> {
        let available: Vec<usize> = self
            .shapes
            .iter()
            .enumerate()
            .filter_map(|(index, shape)| shape.available().then_some(index))
            .collect();
        if available.is_empty() {
            return None;
        }
        let len = i32::try_from(available.len()).unwrap_or(i32::MAX);
        let pick = self.rng.next_int(0, len);
        Some(available[usize::try_from(pick).unwrap_or(0)])
    }
}

/// Wall-clock throttled Snakes runtime wrapping the stateful step machine.
#[derive(Clone, Debug)]
struct SnakesRuntime {
    state: SnakesState,
    last_step_ms: Option<u64>,
}

impl SnakesRuntime {
    fn new() -> Self {
        Self {
            state: SnakesState::new(),
            last_step_ms: None,
        }
    }

    fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        let now = input.now_ms;
        if self.last_step_ms.is_none() {
            // Mirror C# `lastUpdate = DeterministicClock.Now` at construction: the
            // first frame at the same timestamp does not step.
            self.last_step_ms = Some(now);
            return;
        }
        let Some(last) = self.last_step_ms else {
            return;
        };
        if now.saturating_sub(last) < SNAKES_STEP_MS {
            return;
        }
        let steps = u32::try_from(
            (now.saturating_sub(last) / SNAKES_STEP_MS).min(u64::from(SNAKES_MAX_CATCHUP_STEPS)),
        )
        .unwrap_or(SNAKES_MAX_CATCHUP_STEPS);
        for _ in 0..steps {
            self.state.step(input, out);
        }
        self.last_step_ms = Some(now);
    }
}

/// Spectrum `domeVolumeRotationSpeed` default used by Race rotation math.
const VOLUME_ROTATION_SPEED: f64 = 0.25;
/// Spectrum Race band half-width when `domeRadialSize` is 1.0 (see `LEDDomeRaceVisualizer`).
const RACE_RACER_SPACING: f64 = 1.0;

#[derive(Clone, Copy, Debug)]
enum RaceRotation {
    Constant,
    VolumeSquared,
    Beat,
}

#[derive(Clone, Copy, Debug)]
enum RaceColoring {
    Multi,
    FadeExp,
}

#[derive(Clone, Copy, Debug)]
#[allow(
    dead_code,
    reason = "coloring/color_index mirror Spectrum RacerConfig for future wiring"
)]
struct RaceRacerConfig {
    rotation: RaceRotation,
    width: f64,
    coloring: RaceColoring,
    color_index: usize,
}

/// Spectrum `LEDDomeRaceVisualizer.racerConfig`, ground-to-pole order.
const RACE_RACER_CONFIGS: [RaceRacerConfig; 4] = [
    RaceRacerConfig {
        rotation: RaceRotation::VolumeSquared,
        width: 1.0,
        coloring: RaceColoring::Multi,
        color_index: 0,
    },
    RaceRacerConfig {
        rotation: RaceRotation::VolumeSquared,
        width: 0.25,
        coloring: RaceColoring::FadeExp,
        color_index: 1,
    },
    RaceRacerConfig {
        rotation: RaceRotation::Beat,
        width: 0.125,
        coloring: RaceColoring::FadeExp,
        color_index: 2,
    },
    RaceRacerConfig {
        rotation: RaceRotation::Constant,
        width: 1.0,
        coloring: RaceColoring::Multi,
        color_index: 3,
    },
];

#[derive(Clone, Debug)]
#[allow(
    dead_code,
    reason = "radians mirrors Spectrum Racer width; rendering uses shared RACER_WIDTHS table"
)]
struct RaceRacer {
    angle: f64,
    radians: f64,
    accumulated_seconds: f64,
    config: RaceRacerConfig,
}

impl RaceRacer {
    fn new(config: RaceRacerConfig) -> Self {
        Self {
            angle: 0.0,
            radians: std::f64::consts::TAU * config.width,
            accumulated_seconds: 0.0,
            config,
        }
    }

    fn revs_per_second(&self, volume: f64, measure_length_ms: Option<u32>) -> f64 {
        match self.config.rotation {
            RaceRotation::VolumeSquared => volume.mul_add(volume, VOLUME_ROTATION_SPEED / 12.0),
            RaceRotation::Beat => {
                let beats_per_second = match measure_length_ms {
                    Some(measure) if measure > 0 => 1000.0 / f64::from(measure),
                    _ => 1.0,
                };
                beats_per_second / 4.0
            }
            RaceRotation::Constant => VOLUME_ROTATION_SPEED / 4.0,
        }
    }

    fn move_racer(&mut self, num_seconds: f64, volume: f64, measure_length_ms: Option<u32>) {
        let rads_per_second =
            std::f64::consts::TAU * self.revs_per_second(volume, measure_length_ms);
        let rads = (num_seconds + self.accumulated_seconds) * rads_per_second;
        if rads < 0.0001 {
            // Too small to move at f64 precision; bank the time for a later step.
            self.accumulated_seconds += num_seconds;
            return;
        }
        self.angle += rads;
        if self.angle > std::f64::consts::PI {
            self.angle -= std::f64::consts::TAU;
        }
        self.accumulated_seconds = 0.0;
    }
}

/// Persistent Race runtime porting `LEDDomeRaceVisualizer`'s wall-clock racers.
#[derive(Clone, Debug)]
struct RaceRuntime {
    racers: Vec<RaceRacer>,
    last_ms: Option<u64>,
}

impl RaceRuntime {
    fn new() -> Self {
        Self {
            racers: RACE_RACER_CONFIGS
                .iter()
                .copied()
                .map(RaceRacer::new)
                .collect(),
            last_ms: None,
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "Millisecond deltas are small and converted to fractional seconds"
    )]
    fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        if let Some(last) = self.last_ms {
            let num_seconds = input.now_ms.saturating_sub(last) as f64 / 1000.0;
            let volume = f64::from(input.volume.clamp(0.0, 1.0));
            for racer in &mut self.racers {
                racer.move_racer(num_seconds, volume, input.measure_length_ms);
            }
        }
        self.last_ms = Some(input.now_ms);

        let points = DOME_LED_POINTS.get_or_init(build_dome_led_points);
        let mut point_index = 0;
        let start_angles = self.racer_start_angles();
        for strut_index in 0..DOME_STRUTS {
            let Some(strut_length) = dome_strut_length(strut_index) else {
                continue;
            };
            for led_index in 0..strut_length {
                let point = points.get(point_index).copied().unwrap_or(DomeLedPoint {
                    index: point_index,
                    x: 0.5,
                    y: 0.5,
                });
                point_index += 1;
                out.push(DomeCommand::Pixel {
                    strut_index,
                    led_index,
                    color: race_pixel_color(*input, point.x, point.y, Some(start_angles)),
                });
            }
        }
        out.push(DomeCommand::Flush);
    }

    fn racer_start_angles(&self) -> [f64; 4] {
        let mut angles = [0.0; 4];
        for (index, racer) in self.racers.iter().enumerate().take(angles.len()) {
            angles[index] = racer.angle;
        }
        angles
    }
}

/// Spectrum `LEDDomeOutputBuffer` pixel retaining fractional channels so fade
/// accumulation across frames matches the C# reference exactly.
#[derive(Clone, Debug)]
struct DomeBufferPixel {
    x: f64,
    y: f64,
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
fn clamp_byte(value: f64) -> u32 {
    if value <= 0.0 {
        0
    } else if value >= 255.0 {
        255
    } else {
        value as u32
    }
}

impl DomeBufferPixel {
    fn set_color(&mut self, color: Rgb) {
        let packed = color.to_u24();
        self.color = packed;
        self.r = f64::from((packed >> 16) & 0xff);
        self.g = f64::from((packed >> 8) & 0xff);
        self.b = f64::from(packed & 0xff);
    }

    fn blend_light_paint(&mut self, paint: Rgb) {
        self.set_color(light_paint(self.rgb(), paint));
    }

    fn update_color(&mut self) {
        self.color = (clamp_byte(self.r) << 16) | (clamp_byte(self.g) << 8) | clamp_byte(self.b);
    }

    fn fade(&mut self, mul: f64, sub: f64) {
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
    fn hue_rotate(&mut self, rate: f64) {
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

    fn rgb(&self) -> Rgb {
        Rgb::from_u24(self.color)
    }
}

/// Persistent full-dome buffer mirroring `LEDDomeOutputBuffer`.
#[derive(Clone, Debug)]
struct DomeBuffer {
    pixels: Vec<DomeBufferPixel>,
}

impl DomeBuffer {
    fn new() -> Self {
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

    fn fade(&mut self, mul: f64, sub: f64) {
        for pixel in &mut self.pixels {
            pixel.fade(mul, sub);
        }
    }

    fn hue_rotate(&mut self, rate: f64) {
        for pixel in &mut self.pixels {
            pixel.hue_rotate(rate);
        }
    }

    fn frame_commands(&self) -> Vec<DomeCommand> {
        vec![
            DomeCommand::Frame(self.pixels.iter().map(DomeBufferPixel::rgb).collect()),
            DomeCommand::Flush,
        ]
    }
}

// Spectrum dome config defaults (`SpectrumConfiguration`) hardcoded here because
// domers pins Spectrum's default preset; wiring them from DomersConfig is the
// diagnostics-cadence follow-up.
const DOME_GLOBAL_FADE_SPEED: f64 = 0.0;
const DOME_GLOBAL_HUE_SPEED: f64 = 1.0;
const DOME_RADIAL_CENTER_SPEED: f64 = 0.0;
const DOME_RADIAL_CENTER_ANGLE: f64 = 0.0;
const DOME_RADIAL_CENTER_DISTANCE: f64 = 0.0;
const DOME_RADIAL_EFFECT: i32 = 0;
const DOME_RADIAL_FREQUENCY: f64 = 1.0;
/// Spectrum `domeRadialSize` from `spectrum_default_config.xml` used for goldens.
const DOME_RADIAL_SIZE: f64 = 1.0;
const SPLAT_FADE: f64 = 0.96;

fn polar_to_cartesian(angle: f64, distance: f64) -> (f64, f64) {
    (angle.cos() * distance, angle.sin() * distance)
}

/// Compute `(val, gradient_val)` for a radial effect, porting the C# switch.
fn radial_effect(effect: i32, angle: f64, dist: f64, current_angle: f64) -> (f64, f64) {
    let freq = DOME_RADIAL_FREQUENCY;
    match effect {
        1 => {
            let mut val = map_wrap(dist, current_angle, 1.0 + current_angle, 0.0, 1.0);
            val = wrap(val * freq, 0.0, 1.0);
            val = map_value(val, 0.0, 1.0, -1.0, 1.0).abs();
            let gradient_val = map_value(angle, 0.0, 1.0, -1.0, 1.0).abs();
            (val, gradient_val)
        }
        2 => {
            let mut val = map_wrap(
                angle + dist / freq,
                current_angle,
                1.0 + current_angle,
                0.0,
                1.0,
            );
            val = wrap(val * freq, 0.0, 1.0);
            val = map_value(val, 0.0, 1.0, -1.0, 1.0).abs();
            (val, dist)
        }
        3 => {
            let mut a = map_wrap(angle, current_angle, 1.0 + current_angle, 0.0, 1.0);
            a = wrap(a * freq, 0.0, 1.0);
            a = map_value(a, 0.0, 1.0, -1.0, 1.0).abs();
            ((dist - a).clamp(0.0, 1.0), dist)
        }
        _ => {
            let mut val = map_wrap(angle, current_angle, 1.0 + current_angle, 0.0, 1.0);
            val = wrap(val * freq, 0.0, 1.0);
            val = map_value(val, 0.0, 1.0, -1.0, 1.0).abs();
            (val, dist)
        }
    }
}

/// Persistent Radial runtime porting `LEDDomeRadialVisualizer`.
#[derive(Clone, Debug)]
struct RadialRuntime {
    buffer: DomeBuffer,
    current_angle: f64,
    current_gradient: f64,
    current_center_angle: f64,
    last_progress: f64,
}

impl RadialRuntime {
    fn new() -> Self {
        Self {
            buffer: DomeBuffer::new(),
            current_angle: 0.0,
            current_gradient: 0.0,
            current_center_angle: 0.0,
            last_progress: 0.0,
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Spectrum picks the radial gradient by truncating normalized volume times 8"
    )]
    fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        self.buffer
            .fade(1.0 - 10f64.powf(-DOME_GLOBAL_FADE_SPEED), 0.0);
        self.buffer.hue_rotate(10f64.powf(-DOME_GLOBAL_HUE_SPEED));

        let level = f64::from(input.volume.clamp(0.0, 1.0));
        let adjusted_level = level.sqrt().clamp(0.1, 1.0);
        let progress = input.beat_progress;
        let delta = wrap(progress - self.last_progress, 0.0, 1.0);
        self.current_angle = wrap(
            self.current_angle + VOLUME_ROTATION_SPEED * delta * 0.25,
            0.0,
            1.0,
        );
        self.current_gradient = wrap(
            self.current_gradient + VOLUME_GRADIENT_SPEED * delta,
            0.0,
            1.0,
        );
        self.current_center_angle = wrap(
            self.current_center_angle + DOME_RADIAL_CENTER_SPEED * delta * 0.25,
            0.0,
            1.0,
        );
        self.last_progress = progress;

        let center = polar_to_cartesian(
            DOME_RADIAL_CENTER_ANGLE + self.current_center_angle * std::f64::consts::TAU,
            DOME_RADIAL_CENTER_DISTANCE,
        );
        let which_gradient = ((level * 8.0) as usize).min(7);
        let size_limit = DOME_RADIAL_SIZE * adjusted_level;

        for pixel in &mut self.buffer.pixels {
            let px = (pixel.x + center.0) * 2.0 - 1.0;
            let py = (pixel.y + center.1) * 2.0 - 1.0;
            let angle = map_wrap(
                py.atan2(px),
                -std::f64::consts::PI,
                std::f64::consts::PI,
                0.0,
                1.0,
            );
            let dist = (px * px + py * py).sqrt();
            let (val, gradient_val) =
                radial_effect(DOME_RADIAL_EFFECT, angle, dist, self.current_angle);
            if val <= size_limit {
                let color = input.palette_entries[which_gradient].gradient_color(
                    gradient_val,
                    self.current_gradient,
                    true,
                );
                pixel.set_color(color);
            }
        }

        out.extend(self.buffer.frame_commands());
    }
}

/// Persistent Splat runtime porting `LEDDomeSplatVisualizer`.
#[derive(Clone, Debug)]
struct SplatRuntime {
    buffer: DomeBuffer,
    rng: DotNetRandom,
    last_progress: f64,
    seen_first: bool,
}

impl SplatRuntime {
    fn new() -> Self {
        Self {
            buffer: DomeBuffer::new(),
            rng: DotNetRandom::new(0),
            last_progress: 0.0,
            seen_first: false,
        }
    }

    #[allow(
        clippy::cast_sign_loss,
        reason = "Spectrum indexes the palette with a non-negative Random.Next() modulo 8"
    )]
    fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        let level = f64::from(input.volume.clamp(0.0, 1.0));
        let adjusted_level = level.sqrt().clamp(0.1, 1.0);
        let progress = input.beat_progress;

        self.buffer.fade(SPLAT_FADE, 0.0);

        if self.seen_first && progress < self.last_progress {
            let cx = map_value(self.rng.next_double(), 0.0, 1.0, 0.1, 0.9);
            let cy = map_value(self.rng.next_double(), 0.0, 1.0, 0.1, 0.9);
            let radius = adjusted_level * 0.25;
            let color_index = (self.rng.next().rem_euclid(8)) as usize;
            for pixel in &mut self.buffer.pixels {
                let dx = pixel.x - cx;
                let dy = pixel.y - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < radius {
                    let color =
                        input.palette_entries[color_index].gradient_color(dist / radius, 0.0, true);
                    pixel.set_color(color);
                }
            }
        }

        self.last_progress = progress;
        self.seen_first = true;
        out.extend(self.buffer.frame_commands());
    }
}

const DOME_TWINKLE_DENSITY: f64 = 0.0;
const DOME_RIPPLE_CD_STEP: f64 = 1.0;
const DOME_RIPPLE_STEP: f64 = 1.0;

/// Persistent Paintbrush runtime mirroring `LEDDomeQuaternionPaintbrushVisualizer`.
#[derive(Clone, Debug)]
struct PaintbrushRuntime {
    buffer: DomeBuffer,
    rng: DotNetRandom,
    yaw: f64,
    pitch: f64,
    roll: f64,
    yaw_momentum: f64,
    pitch_momentum: f64,
    roll_momentum: f64,
    counter: u64,
    cooldown: i32,
    stamp_fired: bool,
    stamp_effect: i32,
    last_progress: f64,
    ripple_counter: f64,
    ripple_firing: bool,
    ripple_cooldown: f64,
    ripple_type: i32,
    contour_counter: f64,
    stamp_center: Quaternion,
    ripple_center: Quaternion,
}

impl PaintbrushRuntime {
    fn new() -> Self {
        Self {
            buffer: DomeBuffer::new(),
            rng: DotNetRandom::new(0),
            yaw: 0.0,
            pitch: -0.25,
            roll: 0.0,
            yaw_momentum: 0.0,
            pitch_momentum: 0.0005,
            roll_momentum: 0.0,
            counter: 0,
            cooldown: 7,
            stamp_fired: false,
            stamp_effect: 0,
            last_progress: 0.0,
            ripple_counter: 0.0,
            ripple_firing: false,
            ripple_cooldown: 100.0,
            ripple_type: 0,
            contour_counter: 0.0,
            stamp_center: Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            ripple_center: Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
        }
    }

    fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        let progress = input.beat_progress;
        let level = f64::from(input.volume.clamp(0.0, 1.0));

        self.buffer
            .fade(1.0 - 5f64.powf(-DOME_GLOBAL_FADE_SPEED), 0.0);
        let hue_rate =
            (3.0 * progress * progress - 3.0 * progress + 1.0) * 10f64.powf(-DOME_GLOBAL_HUE_SPEED);
        self.buffer.hue_rotate(hue_rate);
        self.counter += 1;

        let orientation = self.idle_orientation(input);
        self.update_stamp_and_ripple(level, progress, orientation);
        self.contour_counter += 4.0 * level;
        if self.contour_counter >= 100.0 {
            self.contour_counter = 0.0;
        }

        let threshold_factor = DOME_RADIAL_SIZE / 4.0 + level + 0.01;
        let threshold = 2.0 / threshold_factor;
        let saturation = (1.3 / level.max(0.01) - 1.0).clamp(0.2, 1.0);
        let metaball_hue = (1.0 + orientation.w) / 2.0;

        for pixel in &mut self.buffer.pixels {
            let (x, y, z) = hemisphere_point(pixel.x, pixel.y);
            let (rx, ry, rz) = orientation.transform_vector(x, y, z);
            let distance = distance3(rx, ry, rz, -1.0, 0.0, 0.0);
            let neg_distance = distance3(rx, ry, rz, 1.0, 0.0, 0.0);
            let potential = 1.0 / (distance * neg_distance);
            let strength = potential - threshold;

            let twinkle_roll = self.rng.next_double();
            if twinkle_roll < DOME_TWINKLE_DENSITY && z > 0.2 {
                pixel.set_color(Rgb::from_u24(0xff_ff_ff));
            }

            if strength > 0.0 {
                pixel.blend_light_paint(hsv_to_rgb(metaball_hue, saturation, 1.0));
            }

            if self.ripple_firing && self.ripple_counter > 0.0 {
                let (tx, ty, tz) = self.ripple_center.transform_vector(x, y, z);
                let ripple_radius = self.ripple_counter / 300.0;
                let distance_to_spot = distance3(tx, ty, tz, -1.0, 0.0, 0.0);
                if (distance_to_spot - ripple_radius).abs() < 0.012 {
                    let ripple_saturation = (1.0 - self.ripple_counter / 600.0).clamp(0.0, 1.0);
                    let ripple_value = (1.0 - self.ripple_counter / 800.0).clamp(0.0, 1.0);
                    pixel.blend_light_paint(hsv_to_rgb(
                        metaball_hue,
                        ripple_saturation,
                        ripple_value,
                    ));
                }
            }

            if self.stamp_fired {
                let (sx, sy, sz) = self.stamp_center.transform_vector(x, y, z);
                let distance_to_spot = distance3(sx, sy, sz, -1.0, 0.0, 0.0);
                if self.stamp_effect == 1 && distance_to_spot.rem_euclid(0.4) < 0.05 {
                    pixel.set_color(hsv_to_rgb(metaball_hue, 0.2, 1.0));
                } else if self.stamp_effect == 2 {
                    let ring_distance =
                        2.4 - (1.8 / (4.0 - f64::from(self.cooldown) / 2.0)).clamp(0.0, 2.4);
                    let half_width = 0.003 * f64::from(self.cooldown * self.cooldown);
                    if (ring_distance - half_width..=ring_distance + half_width)
                        .contains(&distance_to_spot)
                    {
                        pixel.set_color(hsv_to_rgb(metaball_hue, 0.2, 1.0));
                    }
                }
            }
        }

        if self.cooldown < 7 && self.stamp_effect == 1 {
            self.stamp_fired = false;
        }
        self.last_progress = progress;
        out.extend(self.buffer.frame_commands());
    }

    fn idle_orientation(&mut self, input: &VisualizerInput) -> Quaternion {
        if let Some(orientation) = input.orientation_override {
            return Quaternion::from_yaw_pitch_roll(
                orientation.yaw,
                orientation.pitch,
                orientation.roll,
            );
        }
        if let Some(device) = input.orientation_devices.iter().find_map(|device| *device) {
            return device.rotation;
        }

        let noise = 0.0001;
        self.yaw_momentum =
            (self.yaw_momentum + spectrum_nudge(&mut self.rng, noise)).clamp(-0.001, 0.001);
        self.roll_momentum =
            (self.roll_momentum + spectrum_nudge(&mut self.rng, noise)).clamp(-0.001, 0.001);
        self.pitch_momentum =
            (self.pitch_momentum + spectrum_nudge(&mut self.rng, noise)).clamp(-0.001, 0.001);

        let motion_scale = 4.0 * (f64::from(input.volume.clamp(0.0, 1.0)) + 0.25);
        self.yaw += motion_scale * self.yaw_momentum;
        self.pitch += motion_scale * self.pitch_momentum;
        self.roll += motion_scale * self.roll_momentum;

        Quaternion::from_unitless_yaw_pitch_roll(self.yaw, self.pitch, self.roll)
    }

    fn update_stamp_and_ripple(&mut self, level: f64, progress: f64, orientation: Quaternion) {
        if self.cooldown > 0 && self.last_progress > progress {
            self.cooldown -= 1;
            if self.cooldown <= 0 {
                self.stamp_fired = false;
            }
        }
        if self.counter > 1_000 && level > 0.3 {
            self.stamp_fired = true;
            self.counter = 0;
            self.cooldown = 10;
            let mut effect = self.stamp_effect;
            if effect == 0 {
                effect = 1;
            }
            if effect == 1 {
                effect = 2;
            }
            if effect == 2 {
                effect = 1;
            }
            self.stamp_effect = effect;
            self.stamp_center = orientation;
        }

        if self.ripple_counter > 1_000.0 {
            self.ripple_counter = 0.0;
            self.ripple_firing = false;
        }
        if !self.ripple_firing {
            self.ripple_cooldown -= DOME_RIPPLE_CD_STEP;
        }
        if self.ripple_cooldown < 0.0 {
            self.ripple_firing = true;
            self.ripple_type = (self.ripple_type + 1) % 2;
            self.ripple_center = orientation;
            self.ripple_cooldown = 100.0;
        }
        if self.ripple_firing {
            self.ripple_counter += DOME_RIPPLE_STEP;
            if self.ripple_type == 1 {
                self.ripple_center = orientation;
            }
        }
    }
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

fn tv_static_commands(input: VisualizerInput) -> Vec<DomeCommand> {
    let seed = i32::try_from(input.animation_frame % i32::MAX as u64)
        .expect("TV static frame seed fits in i32");
    let mut random = DotNetRandom::new(seed);
    let mut commands = Vec::with_capacity(DOME_PIXELS + 1);
    for strut_index in 0..DOME_STRUTS {
        let Some(strut_length) = dome_strut_length(strut_index) else {
            continue;
        };
        for led_index in 0..strut_length {
            commands.push(DomeCommand::Pixel {
                strut_index,
                led_index,
                color: random.next_color(255),
            });
        }
    }
    commands.push(DomeCommand::Flush);
    commands
}

#[derive(Clone, Debug)]
struct DotNetRandom {
    seed_array: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl DotNetRandom {
    const MBIG: i32 = 2_147_483_647;
    const MSEED: i32 = 161_803_398;

    fn new(seed: i32) -> Self {
        let subtraction = if seed == i32::MIN {
            i32::MAX
        } else {
            seed.abs()
        };
        let mut seed_array = [0; 56];
        let mut mj = Self::MSEED - subtraction;
        if mj < 0 {
            mj += Self::MBIG;
        }
        seed_array[55] = mj;
        let mut mk = 1;
        for i in 1..55 {
            let ii = (21 * i) % 55;
            seed_array[ii] = mk;
            mk = mj - mk;
            if mk < 0 {
                mk += Self::MBIG;
            }
            mj = seed_array[ii];
        }
        for _ in 0..4 {
            for i in 1..56 {
                seed_array[i] -= seed_array[1 + (i + 30) % 55];
                if seed_array[i] < 0 {
                    seed_array[i] += Self::MBIG;
                }
            }
        }
        Self {
            seed_array,
            inext: 0,
            inextp: 21,
        }
    }

    fn internal_sample(&mut self) -> i32 {
        self.inext += 1;
        if self.inext >= 56 {
            self.inext = 1;
        }
        self.inextp += 1;
        if self.inextp >= 56 {
            self.inextp = 1;
        }
        let mut ret = self.seed_array[self.inext] - self.seed_array[self.inextp];
        if ret == Self::MBIG {
            ret -= 1;
        }
        if ret < 0 {
            ret += Self::MBIG;
        }
        self.seed_array[self.inext] = ret;
        ret
    }

    fn next_double(&mut self) -> f64 {
        f64::from(self.internal_sample()) * (1.0 / f64::from(Self::MBIG))
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Spectrum truncates Random.NextDouble multiplied by the byte brightness cap"
    )]
    fn next_color(&mut self, brightness_byte: i32) -> Rgb {
        let blue = (self.next_double() * f64::from(brightness_byte)) as u8;
        let green = (self.next_double() * f64::from(brightness_byte)) as u8;
        let red = (self.next_double() * f64::from(brightness_byte)) as u8;
        Rgb {
            r: red,
            g: green,
            b: blue,
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "Spectrum .NET Random.Next(min,max) truncates Sample()*range to an int"
    )]
    fn next_int(&mut self, min_value: i32, max_value: i32) -> i32 {
        let range = i64::from(max_value) - i64::from(min_value);
        (self.next_double() * range as f64) as i32 + min_value
    }

    /// Mirrors C# `Random.Next()`, returning a value in `[0, i32::MAX)`.
    fn next(&mut self) -> i32 {
        self.internal_sample()
    }
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

const VOLUME_ANIMATION_SIZE: usize = 4;
const VOLUME_GRADIENT_SPEED: f64 = 0.25;
const VOLUME_STARTING_POINTS: [usize; 6] = [22, 26, 30, 34, 38, 70];

#[allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    reason = "Volume port mirrors Spectrum's small integer layout ratios and exact filled-section checks"
)]
fn volume_commands(input: VisualizerInput) -> Vec<DomeCommand> {
    let beat_progress = if input.animation_frame == 0 {
        0.0
    } else {
        input.beat_progress
    };
    volume_commands_with_wipe(input, beat_progress, input.animation_frame == 0)
}

#[allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    reason = "Volume port mirrors Spectrum's small integer layout ratios and exact filled-section checks"
)]
fn volume_commands_with_wipe(
    input: VisualizerInput,
    beat_progress: f64,
    include_wipe: bool,
) -> Vec<DomeCommand> {
    let layouts = volume_layouts(volume_center_offset(beat_progress));
    let total_parts = VOLUME_ANIMATION_SIZE;
    let volume_split_into = 2 * ((total_parts - 1) / 2 + 1);
    let level = f64::from(input.volume.clamp(0.0, 1.0));
    let gradient_focus = progress_through_beat(beat_progress, VOLUME_GRADIENT_SPEED);
    let mut commands = if include_wipe {
        volume_wipe_commands()
    } else {
        Vec::new()
    };

    for part in (0..total_parts).step_by(2) {
        let start_range = part as f64 / volume_split_into as f64;
        let end_range = (part + 2) as f64 / volume_split_into as f64;
        let scaled = if end_range == start_range {
            0.0
        } else {
            ((level - start_range) / (end_range - start_range)).clamp(0.0, 1.0)
        };
        let start_lit_range = if level == 0.0 {
            1.0
        } else {
            (start_range / level).min(1.0)
        };
        let end_lit_range = if level == 0.0 {
            1.0
        } else {
            (end_range / level).min(1.0)
        };

        for strut in &layouts.part.segments[part].struts {
            update_volume_strut(
                &mut commands,
                &layouts.part,
                input,
                *strut,
                scaled,
                start_lit_range,
                end_lit_range,
                gradient_focus,
            );
        }

        if part + 1 == total_parts {
            break;
        }

        for section_index in 0..6 {
            let segment = &layouts.section.segments[section_index + part * 3];
            let gradient_step = 1.0 / segment.struts.len() as f64;
            let mut gradient_start_pos = 0.0;
            for strut in &segment.struts {
                let gradient_end_pos = gradient_start_pos + gradient_step;
                update_volume_strut(
                    &mut commands,
                    &layouts.part,
                    input,
                    *strut,
                    if scaled == 1.0 { 1.0 } else { 0.0 },
                    gradient_start_pos,
                    gradient_end_pos,
                    gradient_focus,
                );
                gradient_start_pos = gradient_end_pos;
            }
        }
    }

    commands.push(DomeCommand::Flush);
    commands
}

fn volume_wipe_commands() -> Vec<DomeCommand> {
    let mut commands = Vec::with_capacity(DOME_PIXELS);
    for strut_index in 0..DOME_STRUTS {
        let Some(length) = dome_strut_length(strut_index) else {
            continue;
        };
        for led_index in 0..length {
            commands.push(DomeCommand::Pixel {
                strut_index,
                led_index,
                color: Rgb::BLACK,
            });
        }
    }
    commands
}

#[allow(
    clippy::too_many_arguments,
    reason = "Mirrors Spectrum LEDDomeVolumeVisualizer.UpdateStrut without hiding the layout inputs"
)]
fn update_volume_strut(
    commands: &mut Vec<DomeCommand>,
    part_layout: &VolumeStrutLayout,
    input: VisualizerInput,
    strut: VolumeStrut,
    percentage_lit: f64,
    start_lit_range: f64,
    end_lit_range: f64,
    gradient_focus: f64,
) {
    let Some(length) = dome_strut_length(strut.index) else {
        return;
    };
    for led_index in 0..length {
        let color = volume_gradient_pos(
            strut,
            length,
            percentage_lit,
            start_lit_range,
            end_lit_range,
            led_index,
        )
        .map_or(Rgb::BLACK, |gradient_pos| {
            volume_color_from_part(
                part_layout,
                input,
                strut.index,
                gradient_pos,
                gradient_focus,
            )
        });
        commands.push(DomeCommand::Pixel {
            strut_index: strut.index,
            led_index,
            color,
        });
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Volume strut lengths and LED indexes are small Spectrum topology constants"
)]
fn volume_gradient_pos(
    strut: VolumeStrut,
    length: usize,
    percentage_lit: f64,
    start_lit_range: f64,
    end_lit_range: f64,
    led_index: usize,
) -> Option<f64> {
    if percentage_lit == 0.0 {
        return None;
    }
    let led = if strut.reversed {
        length.saturating_sub(led_index)
    } else {
        led_index
    };
    let step = (end_lit_range - start_lit_range) / (length as f64 * percentage_lit);
    let gradient_pos = start_lit_range + led as f64 * step;
    (gradient_pos <= 1.0).then_some(gradient_pos)
}

fn volume_color_from_part(
    part_layout: &VolumeStrutLayout,
    input: VisualizerInput,
    strut_index: usize,
    pixel_pos: f64,
    gradient_focus: f64,
) -> Rgb {
    let color_index = match part_layout.segment_index_of_strut(strut_index) {
        Some(0) => 1,
        Some(1) => 2,
        Some(2) => 3,
        _ => 0,
    };
    input.palette_entries[color_index].gradient_color(pixel_pos, gradient_focus, true)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Spectrum truncates ProgressThroughBeat times four to choose the volume center"
)]
fn volume_center_offset(beat_progress: f64) -> usize {
    (progress_through_beat(beat_progress, VOLUME_ROTATION_SPEED) * 4.0) as usize
}

/// Mirror `BeatBroadcaster.ProgressThroughBeat` for injected measure progress.
fn progress_through_beat(beat_progress: f64, factor: f64) -> f64 {
    if factor == 0.0 {
        return 0.0;
    }
    let beat_length = 1.0 / factor;
    let progress_in_beat = beat_progress % beat_length;
    progress_in_beat / beat_length
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VolumeStrut {
    index: usize,
    reversed: bool,
}

#[derive(Clone, Debug)]
struct VolumeStrutLayoutSegment {
    struts: Vec<VolumeStrut>,
}

#[derive(Clone, Debug)]
struct VolumeStrutLayout {
    segments: Vec<VolumeStrutLayoutSegment>,
    strut_to_segment: [Option<usize>; DOME_STRUTS],
}

impl VolumeStrutLayout {
    fn new(segments: Vec<VolumeStrutLayoutSegment>) -> Self {
        let mut strut_to_segment = [None; DOME_STRUTS];
        for (segment_index, segment) in segments.iter().enumerate() {
            for strut in &segment.struts {
                strut_to_segment[strut.index] = Some(segment_index);
            }
        }
        Self {
            segments,
            strut_to_segment,
        }
    }

    fn segment_index_of_strut(&self, strut_index: usize) -> Option<usize> {
        self.strut_to_segment.get(strut_index).copied().flatten()
    }
}

#[derive(Clone, Debug)]
struct VolumeLayouts {
    part: VolumeStrutLayout,
    section: VolumeStrutLayout,
}

fn volume_layouts(center_offset: usize) -> VolumeLayouts {
    let mut points = VOLUME_STARTING_POINTS;
    for point in points.iter_mut().take(5) {
        *point += center_offset;
    }
    if points[4] >= 40 {
        points[4] -= 20;
    }

    let edge_dictionary = volume_edge_dictionary();
    let mut cur_points_by_group: Vec<Vec<usize>> =
        points.iter().copied().map(|point| vec![point]).collect();
    let mut spoke_segments = Vec::new();
    let mut struts_by_group: [Vec<VolumeStrut>; 6] = std::array::from_fn(|_| Vec::new());
    let mut circle_segments = Vec::new();
    let mut used_struts = [false; DOME_STRUTS];
    let mut layers_left = VOLUME_ANIMATION_SIZE;

    while layers_left > 0 {
        let mut layer1 = Vec::new();
        let mut next_points_by_group = Vec::new();
        for (group_index, group) in cur_points_by_group.iter().enumerate() {
            let mut new_points = Vec::new();
            for &point in group {
                for edge in &edge_dictionary[point] {
                    if used_struts[edge.strut.index] {
                        continue;
                    }
                    used_struts[edge.strut.index] = true;
                    push_unique_strut(&mut layer1, edge.strut);
                    push_unique_strut(&mut struts_by_group[group_index], edge.strut);
                    push_unique_usize(&mut new_points, edge.connected_point);
                }
            }
            next_points_by_group.push(new_points);
        }
        spoke_segments.push(VolumeStrutLayoutSegment { struts: layer1 });
        layers_left -= 1;
        if layers_left == 0 {
            break;
        }

        cur_points_by_group = next_points_by_group;
        let mut layer2 = Vec::new();
        for (group_index, group) in cur_points_by_group.iter().enumerate() {
            let Some(mut current_point) = group.first().copied() else {
                circle_segments.push(VolumeStrutLayoutSegment { struts: Vec::new() });
                continue;
            };
            for &point in group {
                let connected_count = edge_dictionary[point]
                    .iter()
                    .filter(|edge| group.contains(&edge.connected_point))
                    .count();
                if connected_count == 1 {
                    current_point = point;
                    break;
                }
            }

            let mut points_left = group.clone();
            let mut circle_struts = Vec::new();
            loop {
                let mut next_point_in_loop = None;
                for edge in &edge_dictionary[current_point] {
                    if !group.contains(&edge.connected_point) || used_struts[edge.strut.index] {
                        continue;
                    }
                    used_struts[edge.strut.index] = true;
                    push_unique_strut(&mut layer2, edge.strut);
                    push_unique_strut(&mut circle_struts, edge.strut);
                    push_unique_strut(&mut struts_by_group[group_index], edge.strut);
                    if points_left.contains(&edge.connected_point) {
                        next_point_in_loop = Some(edge.connected_point);
                    }
                    break;
                }
                points_left.retain(|point| *point != current_point);
                if let Some(next_point) = next_point_in_loop {
                    current_point = next_point;
                } else {
                    break;
                }
            }
            circle_segments.push(VolumeStrutLayoutSegment {
                struts: circle_struts,
            });
        }
        spoke_segments.push(VolumeStrutLayoutSegment { struts: layer2 });
        layers_left -= 1;
    }

    VolumeLayouts {
        part: VolumeStrutLayout::new(spoke_segments),
        section: VolumeStrutLayout::new(circle_segments),
    }
}

fn concentric_layout_from_point(starting_point: usize, num_layers: usize) -> VolumeStrutLayout {
    concentric_layout_from_starting_points(&[starting_point], num_layers)
}

fn concentric_layout_from_starting_points(
    starting_points: &[usize],
    num_layers: usize,
) -> VolumeStrutLayout {
    let edge_dictionary = volume_edge_dictionary();
    let mut cur_points_by_group: Vec<Vec<usize>> = starting_points
        .iter()
        .copied()
        .map(|point| vec![point])
        .collect();
    let mut segments = Vec::new();
    let mut used_struts = [false; DOME_STRUTS];
    let mut layers_left = num_layers;

    while layers_left > 0 {
        let mut layer1 = Vec::new();
        let mut next_points_by_group = Vec::new();
        for group in &cur_points_by_group {
            let mut new_points = Vec::new();
            for &point in group {
                for edge in &edge_dictionary[point] {
                    if used_struts[edge.strut.index] {
                        continue;
                    }
                    used_struts[edge.strut.index] = true;
                    push_unique_strut(&mut layer1, edge.strut);
                    push_unique_usize(&mut new_points, edge.connected_point);
                }
            }
            next_points_by_group.push(new_points);
        }
        segments.push(VolumeStrutLayoutSegment { struts: layer1 });
        layers_left -= 1;
        if layers_left == 0 {
            break;
        }

        cur_points_by_group = next_points_by_group;
        let mut layer2 = Vec::new();
        for group in &cur_points_by_group {
            let Some(mut current_point) = group.first().copied() else {
                continue;
            };
            for &point in group {
                let connected_count = edge_dictionary[point]
                    .iter()
                    .filter(|edge| group.contains(&edge.connected_point))
                    .count();
                if connected_count == 1 {
                    current_point = point;
                    break;
                }
            }

            let mut points_left = group.clone();
            loop {
                let mut next_point_in_loop = None;
                for edge in &edge_dictionary[current_point] {
                    if !group.contains(&edge.connected_point) || used_struts[edge.strut.index] {
                        continue;
                    }
                    used_struts[edge.strut.index] = true;
                    push_unique_strut(&mut layer2, edge.strut);
                    if points_left.contains(&edge.connected_point) {
                        next_point_in_loop = Some(edge.connected_point);
                    }
                    break;
                }
                points_left.retain(|point| *point != current_point);
                if let Some(next_point) = next_point_in_loop {
                    current_point = next_point;
                } else {
                    break;
                }
            }
        }
        segments.push(VolumeStrutLayoutSegment { struts: layer2 });
        layers_left -= 1;
    }

    VolumeStrutLayout::new(segments)
}

fn flash_layout_struts(layout: &VolumeStrutLayout) -> Vec<usize> {
    let mut struts = Vec::new();
    for segment in &layout.segments {
        for strut in &segment.struts {
            push_unique_usize(&mut struts, strut.index);
        }
    }
    struts
}

fn clear_flash_strut(strut_index: usize, out: &mut Vec<DomeCommand>) {
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
fn animate_flash_polygon(
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

fn flash_pad_single_color(input: &VisualizerInput, pad: u8) -> Rgb {
    scale_rgb_f64(
        input.palette_entries[pad as usize % input.palette_entries.len()].single_color(),
        input.dome_brightness,
    )
}

fn flash_pad_gradient_color(input: &VisualizerInput, pad: u8, pixel_pos: f64) -> Rgb {
    scale_rgb_f64(
        input.palette_entries[pad as usize % input.palette_entries.len()]
            .gradient_color(pixel_pos, 0.0, false),
        input.dome_brightness,
    )
}

#[derive(Clone, Copy, Debug)]
struct VolumeEdge {
    connected_point: usize,
    strut: VolumeStrut,
}

fn volume_edge_dictionary() -> Vec<Vec<VolumeEdge>> {
    let mut edges = vec![Vec::new(); 71];
    for (strut_index, [point0, point1]) in VOLUME_LINES.iter().copied().enumerate() {
        edges[point0].push(VolumeEdge {
            connected_point: point1,
            strut: VolumeStrut {
                index: strut_index,
                reversed: false,
            },
        });
        edges[point1].push(VolumeEdge {
            connected_point: point0,
            strut: VolumeStrut {
                index: strut_index,
                reversed: true,
            },
        });
    }
    edges
}

fn push_unique_usize(values: &mut Vec<usize>, value: usize) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn push_unique_strut(values: &mut Vec<VolumeStrut>, value: VolumeStrut) {
    if !values.iter().any(|strut| strut.index == value.index) {
        values.push(value);
    }
}

const VOLUME_LINES: [[usize; 2]; DOME_STRUTS] = [
    [0, 1],
    [1, 2],
    [3, 2],
    [3, 4],
    [4, 5],
    [5, 6],
    [7, 6],
    [7, 8],
    [8, 9],
    [9, 10],
    [11, 10],
    [11, 12],
    [12, 13],
    [13, 14],
    [15, 14],
    [15, 16],
    [16, 17],
    [17, 18],
    [19, 18],
    [19, 0],
    [20, 21],
    [22, 21],
    [23, 22],
    [24, 23],
    [24, 25],
    [26, 25],
    [27, 26],
    [28, 27],
    [28, 29],
    [30, 29],
    [31, 30],
    [32, 31],
    [32, 33],
    [34, 33],
    [35, 34],
    [36, 35],
    [36, 37],
    [38, 37],
    [39, 38],
    [20, 39],
    [41, 40],
    [42, 41],
    [43, 42],
    [44, 43],
    [45, 44],
    [46, 45],
    [47, 46],
    [48, 47],
    [49, 48],
    [50, 49],
    [51, 50],
    [52, 51],
    [53, 52],
    [54, 53],
    [40, 54],
    [56, 55],
    [57, 56],
    [58, 57],
    [59, 58],
    [60, 59],
    [61, 60],
    [62, 61],
    [63, 62],
    [64, 63],
    [55, 64],
    [65, 66],
    [66, 67],
    [67, 68],
    [68, 69],
    [69, 65],
    [20, 0],
    [0, 21],
    [21, 1],
    [1, 22],
    [2, 22],
    [23, 2],
    [23, 3],
    [24, 3],
    [24, 4],
    [4, 25],
    [25, 5],
    [5, 26],
    [6, 26],
    [27, 6],
    [27, 7],
    [28, 7],
    [28, 8],
    [8, 29],
    [29, 9],
    [9, 30],
    [10, 30],
    [31, 10],
    [31, 11],
    [32, 11],
    [32, 12],
    [12, 33],
    [33, 13],
    [13, 34],
    [14, 34],
    [35, 14],
    [35, 15],
    [36, 15],
    [36, 16],
    [16, 37],
    [37, 17],
    [17, 38],
    [18, 38],
    [39, 18],
    [39, 19],
    [20, 19],
    [20, 40],
    [21, 40],
    [21, 41],
    [22, 41],
    [41, 23],
    [42, 23],
    [24, 42],
    [24, 43],
    [25, 43],
    [25, 44],
    [26, 44],
    [44, 27],
    [45, 27],
    [28, 45],
    [28, 46],
    [29, 46],
    [29, 47],
    [30, 47],
    [47, 31],
    [48, 31],
    [32, 48],
    [32, 49],
    [33, 49],
    [33, 50],
    [34, 50],
    [50, 35],
    [51, 35],
    [36, 51],
    [36, 52],
    [37, 52],
    [37, 53],
    [38, 53],
    [53, 39],
    [54, 39],
    [20, 54],
    [40, 55],
    [40, 56],
    [41, 56],
    [56, 42],
    [42, 57],
    [43, 57],
    [43, 58],
    [44, 58],
    [58, 45],
    [45, 59],
    [46, 59],
    [46, 60],
    [47, 60],
    [60, 48],
    [48, 61],
    [49, 61],
    [49, 62],
    [50, 62],
    [62, 51],
    [51, 63],
    [52, 63],
    [52, 64],
    [53, 64],
    [64, 54],
    [54, 55],
    [55, 65],
    [56, 65],
    [57, 65],
    [57, 66],
    [58, 66],
    [59, 66],
    [59, 67],
    [60, 67],
    [61, 67],
    [61, 68],
    [62, 68],
    [63, 68],
    [63, 69],
    [64, 69],
    [55, 69],
    [65, 70],
    [66, 70],
    [67, 70],
    [68, 70],
    [69, 70],
];

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
        runtime_visualizer_progress_unwrapped(input, 200)
    };
    let current_angle = wrap(progress * VOLUME_ROTATION_SPEED * 0.25, 0.0, 1.0);
    let current_gradient = wrap(progress * VOLUME_GRADIENT_SPEED, 0.0, 1.0);
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

    let progress = runtime_visualizer_progress(input, 240);
    points
        .iter()
        .map(|point| {
            let mut color = Rgb::BLACK;
            for splat in splats {
                let age = (progress + splat.phase_offset).rem_euclid(1.0);
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

fn race_commands(input: VisualizerInput) -> Vec<DomeCommand> {
    let points = DOME_LED_POINTS.get_or_init(build_dome_led_points);
    let mut commands = Vec::with_capacity(DOME_PIXELS + 1);
    let mut point_index = 0;
    for strut_index in 0..DOME_STRUTS {
        let Some(strut_length) = dome_strut_length(strut_index) else {
            continue;
        };
        for led_index in 0..strut_length {
            let point = points.get(point_index).copied().unwrap_or(DomeLedPoint {
                index: point_index,
                x: 0.5,
                y: 0.5,
            });
            point_index += 1;
            commands.push(DomeCommand::Pixel {
                strut_index,
                led_index,
                color: race_pixel_color(input, point.x, point.y, None),
            });
        }
    }
    commands.push(DomeCommand::Flush);
    commands
}

fn race_pixel_color(
    input: VisualizerInput,
    projected_x: f64,
    projected_y: f64,
    start_angles: Option<[f64; 4]>,
) -> Rgb {
    let px = projected_x * 2.0 - 1.0;
    let py = projected_y * 2.0 - 1.0;
    let y = 1.0 - (px * px + py * py).sqrt();
    let angle = py.atan2(px);
    let Some((racer_index, loc_ang)) = race_location(input, y, angle, start_angles) else {
        return Rgb::BLACK;
    };
    match racer_index {
        0 | 3 => race_multi_color(input, loc_ang),
        1 => scale_rgb_f64(input.palette[1], 1.0 / (1.0 + (4.0 - 4.0 * loc_ang).exp())),
        2 => scale_rgb_f64(input.palette[2], 1.0 / (1.0 + (4.0 - 4.0 * loc_ang).exp())),
        _ => Rgb::BLACK,
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "Spectrum truncates floating racer positions to integer band indexes"
)]
fn race_location(
    input: VisualizerInput,
    mut y: f64,
    mut angle: f64,
    start_angles: Option<[f64; 4]>,
) -> Option<(usize, f64)> {
    const RACER_COUNT: f64 = 4.0;
    const RACER_WIDTHS: [f64; 4] = [1.0, 0.25, 0.125, 1.0];
    if y > 0.9999 {
        y = 0.9999;
    }
    let racer_loc_y = y * RACER_COUNT;
    let racer_index = usize::try_from(racer_loc_y as isize).ok()?;
    let local_y = (racer_loc_y - racer_index as f64 - 0.5).abs();
    if local_y > RACE_RACER_SPACING {
        return None;
    }
    if racer_index >= RACER_WIDTHS.len() {
        return None;
    }
    if angle < 0.0 {
        angle += std::f64::consts::PI * 2.0;
    }
    let start_angle = match start_angles {
        Some(angles) => angles[racer_index].rem_euclid(std::f64::consts::TAU),
        None => race_start_angle(input, racer_index),
    };
    let mut offset = angle - start_angle;
    if offset < 0.0 {
        offset += std::f64::consts::PI * 2.0;
    }
    let radians = std::f64::consts::PI * 2.0 * RACER_WIDTHS[racer_index];
    if offset < std::f64::consts::PI * 2.0 - radians {
        return None;
    }
    Some((
        racer_index,
        1.0 - (std::f64::consts::PI * 2.0 - offset) / radians,
    ))
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Runtime preview frame counter is small and used as elapsed Spectrum seconds"
)]
fn race_start_angle(input: VisualizerInput, racer_index: usize) -> f64 {
    if input.animation_frame == 0 {
        return 0.0;
    }
    let seconds = input.animation_frame as f64 / 100.0;
    let volume = f64::from(input.volume.clamp(0.0, 1.0));
    let revs_per_second = match racer_index {
        0 | 1 => volume.mul_add(volume, VOLUME_ROTATION_SPEED / 12.0),
        2 => 0.25,
        3 => VOLUME_ROTATION_SPEED / 4.0,
        _ => 0.0,
    };
    (seconds * revs_per_second * std::f64::consts::TAU).rem_euclid(std::f64::consts::TAU)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "Spectrum truncates palette segment selection from normalized racer location"
)]
fn race_multi_color(input: VisualizerInput, loc_ang: f64) -> Rgb {
    let scaled = loc_ang.clamp(0.0, 1.0) * 4.0;
    let min_color_index = (scaled as usize).min(4);
    let max_color_index = min_color_index + 1;
    let scaled_pixel_pos = (loc_ang - min_color_index as f64 / 4.0) * 4.0;
    blend_spectrum_rgb(
        input.palette[min_color_index],
        input.palette[max_color_index],
        scaled_pixel_pos,
    )
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Spectrum LEDColor.GradientColor truncates blended double channels to bytes"
)]
fn blend_spectrum_rgb(color1: Rgb, color2: Rgb, distance: f64) -> Rgb {
    let distance = distance.clamp(0.0, 1.0);
    let inverse = 1.0 - distance;
    Rgb {
        r: (distance * f64::from(color1.r) + inverse * f64::from(color2.r)) as u8,
        g: (distance * f64::from(color1.g) + inverse * f64::from(color2.g)) as u8,
        b: (distance * f64::from(color1.b) + inverse * f64::from(color2.b)) as u8,
    }
}

const SNAKE_LENGTH: usize = 7;
const SNAKES_COLOR_PALETTE_COUNT: i32 = 8;
/// Preview frames per Spectrum 50 ms Snakes throttle step (10 ms preview cadence).
const SNAKES_STEP_FRAMES: u64 = 5;
/// Spectrum Snakes wall-clock throttle interval in milliseconds.
const SNAKES_STEP_MS: u64 = 50;
/// Upper bound on Snakes catch-up steps per render to avoid runaway after stalls.
const SNAKES_MAX_CATCHUP_STEPS: u32 = 8;

/// One dome triangle: three clockwise struts plus directional neighbors, ported
/// from Spectrum's `TriangleSegmentFactory`.
#[derive(Clone, Copy)]
struct TriangleSeg {
    struts: [usize; 3],
    points_up: bool,
    left: Option<usize>,
    above: Option<usize>,
    right: Option<usize>,
    below: Option<usize>,
}

static SNAKE_TRIANGLES: OnceLock<Vec<TriangleSeg>> = OnceLock::new();

fn snake_triangles() -> &'static [TriangleSeg] {
    SNAKE_TRIANGLES.get_or_init(build_snake_triangles)
}

/// (first, second, third, `points_up`) in Spectrum `LoadSegments` order.
const SNAKE_TRIANGLE_DEFS: &[(usize, usize, usize, bool)] = &[
    // Layer 1
    (72, 71, 0, true),
    (73, 21, 72, false),
    (74, 73, 1, true),
    (75, 22, 74, false),
    (76, 75, 2, true),
    (77, 23, 76, false),
    (78, 77, 3, true),
    (79, 24, 78, false),
    (80, 79, 4, true),
    (81, 25, 80, false),
    (82, 81, 5, true),
    (83, 26, 82, false),
    (84, 83, 6, true),
    (85, 27, 84, false),
    (86, 85, 7, true),
    (87, 28, 86, false),
    (88, 87, 8, true),
    (89, 29, 74, false),
    (90, 89, 9, true),
    (91, 30, 74, false),
    (92, 91, 10, true),
    (93, 31, 74, false),
    (94, 93, 11, true),
    (95, 32, 74, false),
    (96, 95, 12, true),
    (97, 33, 74, false),
    (98, 97, 13, true),
    (99, 34, 74, false),
    (100, 99, 14, true),
    (101, 35, 74, false),
    (102, 101, 15, true),
    (103, 36, 74, false),
    (104, 103, 16, true),
    (105, 37, 74, false),
    (106, 105, 17, true),
    (107, 38, 74, false),
    (108, 107, 18, true),
    (109, 39, 108, false),
    (70, 109, 19, true),
    (71, 20, 70, false),
    // Layer 2
    (111, 110, 20, true),
    (112, 40, 111, false),
    (113, 112, 21, true),
    (114, 113, 22, true),
    (115, 41, 114, false),
    (116, 115, 23, true),
    (117, 42, 116, false),
    (118, 117, 24, true),
    (119, 43, 118, false),
    (120, 119, 25, true),
    (121, 120, 26, true),
    (122, 44, 121, false),
    (123, 122, 27, true),
    (124, 45, 123, false),
    (125, 124, 28, true),
    (126, 46, 125, false),
    (127, 126, 29, true),
    (128, 127, 30, true),
    (129, 47, 128, false),
    (130, 129, 31, true),
    (131, 48, 130, false),
    (132, 131, 32, true),
    (133, 49, 132, false),
    (134, 133, 33, true),
    (135, 134, 34, true),
    (136, 50, 135, false),
    (137, 136, 35, true),
    (138, 51, 137, false),
    (139, 138, 36, true),
    (140, 52, 139, false),
    (141, 140, 37, true),
    (142, 141, 38, true),
    (143, 53, 142, false),
    (144, 143, 39, true),
    (110, 54, 144, false),
    // Layer 3
    (147, 146, 40, true),
    (148, 147, 41, true),
    (149, 56, 148, false),
    (150, 149, 42, true),
    (151, 57, 150, false),
    (152, 151, 43, true),
    (153, 152, 44, true),
    (154, 58, 153, false),
    (155, 154, 45, true),
    (156, 59, 155, false),
    (157, 156, 46, true),
    (158, 157, 47, true),
    (159, 60, 158, false),
    (160, 159, 48, true),
    (161, 61, 160, false),
    (162, 161, 49, true),
    (163, 162, 50, true),
    (164, 62, 163, false),
    (165, 164, 51, true),
    (166, 63, 165, false),
    (167, 166, 52, true),
    (168, 167, 53, true),
    (169, 64, 168, false),
    (145, 169, 54, true),
    (146, 55, 145, false),
    // Layer 4
    (171, 170, 55, true),
    (172, 171, 56, true),
    (173, 65, 172, false),
    (174, 173, 57, true),
    (175, 174, 58, true),
    (176, 66, 175, false),
    (177, 176, 59, true),
    (178, 177, 60, true),
    (179, 67, 178, false),
    (180, 179, 61, true),
    (181, 180, 62, true),
    (182, 68, 181, false),
    (183, 182, 63, true),
    (184, 183, 64, true),
    (170, 69, 184, false),
    // Layer 5
    (186, 185, 65, true),
    (187, 186, 66, true),
    (188, 187, 67, true),
    (189, 188, 68, true),
    (185, 189, 69, true),
];

fn build_snake_triangles() -> Vec<TriangleSeg> {
    let mut tris: Vec<TriangleSeg> = Vec::with_capacity(SNAKE_TRIANGLE_DEFS.len());
    for &(first, second, third, points_up) in SNAKE_TRIANGLE_DEFS {
        snake_add_triangle(&mut tris, first, second, third, points_up);
    }
    tris
}

fn snake_add_triangle(
    tris: &mut Vec<TriangleSeg>,
    first: usize,
    second: usize,
    third: usize,
    points_up: bool,
) {
    let new_index = tris.len();
    tris.push(TriangleSeg {
        struts: [first, second, third],
        points_up,
        left: None,
        above: None,
        right: None,
        below: None,
    });

    let find_left = |tris: &[TriangleSeg]| {
        tris.iter().position(|t| {
            (t.struts[1] == first && t.points_up) || (t.struts[2] == first && !t.points_up)
        })
    };

    if points_up {
        if let Some(i) = tris.iter().position(|t| t.struts[1] == third) {
            tris[new_index].below = Some(i);
            tris[i].above = Some(new_index);
        }
        if let Some(i) = find_left(tris) {
            tris[new_index].left = Some(i);
            tris[i].right = Some(new_index);
        }
        if let Some(i) = tris.iter().position(|t| t.struts[0] == second) {
            tris[new_index].right = Some(i);
            tris[i].left = Some(new_index);
        }
    } else {
        if let Some(i) = tris.iter().position(|t| t.struts[2] == second) {
            tris[new_index].above = Some(i);
            tris[i].below = Some(new_index);
        }
        if let Some(i) = find_left(tris) {
            tris[new_index].left = Some(i);
            tris[i].right = Some(new_index);
        }
        if let Some(i) = tris.iter().position(|t| t.struts[0] == third) {
            tris[new_index].right = Some(i);
            tris[i].left = Some(new_index);
        }
    }
}

/// Persistent Snakes state mirroring the Spectrum visualizer instance fields.
#[derive(Clone, Debug)]
struct SnakesState {
    rng: DotNetRandom,
    snakes: [VecDeque<usize>; 2],
    color_palette_index: i32,
}

impl SnakesState {
    fn new() -> Self {
        Self {
            rng: DotNetRandom::new(0),
            snakes: [VecDeque::new(), VecDeque::new()],
            color_palette_index: 0,
        }
    }

    /// Advance one throttled Spectrum update, emitting the delta commands.
    fn step(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        let trailing_color = scale_rgb_f64(
            input.palette[self.color_palette_index.unsigned_abs() as usize % 8],
            input.dome_brightness,
        );
        for snake in &mut self.snakes {
            progress_snake(snake, &mut self.rng, trailing_color, out);
        }
        self.color_palette_index =
            (self.color_palette_index + 1) % (SNAKES_COLOR_PALETTE_COUNT - 1);
        out.push(DomeCommand::Flush);
    }
}

fn progress_snake(
    snake: &mut VecDeque<usize>,
    rng: &mut DotNetRandom,
    trailing_color: Rgb,
    out: &mut Vec<DomeCommand>,
) {
    if snake.is_empty() {
        snake.push_back(0);
    }

    let mut next: Option<usize> = None;
    let mut attempt_count: i32 = 0;
    loop {
        let contains = next.is_some_and(|n| snake.iter().any(|&t| t == n));
        if !(next.is_none() || contains) {
            break;
        }
        let prev = attempt_count;
        attempt_count += 1;
        if prev > 10 {
            next = Some(0);
            break;
        }
        let last = *snake.back().expect("snake is non-empty after seeding");
        next = get_next_triangle(last, rng);
    }

    let next_index = next.expect("snake next triangle resolves to a fallback");
    if snake.len() > SNAKE_LENGTH {
        if let Some(tail) = snake.pop_front() {
            set_triangle_color(tail, trailing_color, out);
        }
    }
    snake.push_back(next_index);
    set_triangle_color(next_index, Rgb::BLACK, out);
}

fn get_next_triangle(current: usize, rng: &mut DotNetRandom) -> Option<usize> {
    let tris = snake_triangles();
    let starting = rng.next_int(0, 4);
    let mut direction = starting;
    loop {
        let neighbor = snake_directional(&tris[current], direction);
        direction += 1;
        if neighbor.is_some() {
            return neighbor;
        }
        if direction > 3 {
            direction = 0;
        }
        if direction == starting {
            return None;
        }
    }
}

fn snake_directional(triangle: &TriangleSeg, direction: i32) -> Option<usize> {
    match direction {
        0 => triangle.left,
        1 => triangle.above,
        3 => triangle.below,
        _ => triangle.right,
    }
}

fn set_triangle_color(triangle: usize, color: Rgb, out: &mut Vec<DomeCommand>) {
    let tris = snake_triangles();
    for &strut_index in &tris[triangle].struts {
        let Some(length) = dome_strut_length(strut_index) else {
            continue;
        };
        for led_index in 0..length {
            out.push(DomeCommand::Pixel {
                strut_index,
                led_index,
                color,
            });
        }
    }
}

fn snakes_commands(input: VisualizerInput) -> Vec<DomeCommand> {
    let steps = input.animation_frame / SNAKES_STEP_FRAMES + 1;
    let mut state = SnakesState::new();
    let mut out = Vec::new();
    for _ in 0..steps {
        out.clear();
        state.step(&input, &mut out);
    }
    out
}

fn quaternion_test_frame(input: VisualizerInput) -> Vec<Rgb> {
    let orientation = input.orientation_override.map_or_else(
        || Quaternion {
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

fn quaternion_multi_test_frame(input: VisualizerInput) -> Vec<Rgb> {
    let Some(orientation_override) = input.orientation_override else {
        return vec![Rgb::BLACK; DOME_PIXELS];
    };
    let orientation = Quaternion::from_yaw_pitch_roll(
        orientation_override.yaw,
        orientation_override.pitch,
        orientation_override.roll,
    );
    let spot = (0.0, 1.0, 0.0);
    DOME_LED_POINTS
        .get_or_init(build_dome_led_points)
        .iter()
        .map(|point| {
            let (x, y, z) = spectrum_quaternion_test_point(point.x, point.y);
            let (rx, ry, rz) = orientation.transform_vector(x, y, z);
            let distance = distance3(rx, ry, rz, spot.0, spot.1, spot.2);
            if distance < 0.2 {
                hsv_to_rgb(0.0, 1.0, ((0.2 - distance) / 0.2).clamp(0.0, 1.0))
            } else {
                Rgb::BLACK
            }
        })
        .collect()
}

fn quaternion_paintbrush_frame(input: VisualizerInput) -> Vec<Rgb> {
    let orientation = input.orientation_override.map_or_else(
        || idle_paintbrush_orientation(input),
        |orientation| {
            Quaternion::from_yaw_pitch_roll(orientation.yaw, orientation.pitch, orientation.roll)
        },
    );
    let frame_in_cycle = u64::from(paintbrush_frame_in_cycle(input));
    let trail_orientations = paintbrush_trail_orientations(input, frame_in_cycle);
    let ripple_counter = paintbrush_ripple_counter(frame_in_cycle);
    let stamp_frame = paintbrush_stamp_frame(frame_in_cycle);
    let threshold_factor = 0.25 + f64::from(input.volume.clamp(0.0, 1.0)) + 0.01;
    let threshold = 2.0 / threshold_factor;
    let saturation = (1.3 / f64::from(input.volume.max(0.01)) - 1.0).clamp(0.2, 1.0);

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

            let mut color = paintbrush_twinkle(input, point.index, z);
            if strength > 0.0 {
                color = light_paint(color, hsv_to_rgb(hue, saturation, 1.0));
            }

            for (trail_orientation, fade) in &trail_orientations {
                let (tx, ty, tz) = trail_orientation.transform_vector(x, y, z);
                let trail_distance = distance3(tx, ty, tz, -1.0, 0.0, 0.0).max(0.001);
                let trail_neg_distance = distance3(tx, ty, tz, 1.0, 0.0, 0.0).max(0.001);
                let trail_potential = 1.0 / (trail_distance * trail_neg_distance);
                if trail_potential > threshold {
                    let trail_hue = (1.0 + trail_orientation.w) / 2.0;
                    color = light_paint(color, hsv_to_rgb(trail_hue, saturation, *fade));
                }
            }

            if ripple_counter > 0.0 {
                let ripple_radius = ripple_counter / 300.0;
                let distance_to_spot = distance3(rx, ry, rz, -1.0, 0.0, 0.0);
                if (distance_to_spot - ripple_radius).abs() < 0.012 {
                    let ripple_saturation = (1.0 - ripple_counter / 600.0).clamp(0.0, 1.0);
                    let ripple_value = (1.0 - ripple_counter / 800.0).clamp(0.0, 1.0);
                    color = light_paint(color, hsv_to_rgb(hue, ripple_saturation, ripple_value));
                }
            }

            if let Some(stamp_frame) = stamp_frame {
                let distance_to_spot = distance3(rx, ry, rz, -1.0, 0.0, 0.0);
                if paintbrush_stamp_ring(distance_to_spot, stamp_frame) {
                    color = hsv_to_rgb(hue, 0.2, 1.0);
                }
            }

            color
        })
        .collect()
}

/// Unit quaternion used for orientation device input.
#[derive(Clone, Copy, Debug)]
pub struct Quaternion {
    x: f64,
    y: f64,
    z: f64,
    w: f64,
}

impl Quaternion {
    fn from_unitless_yaw_pitch_roll(yaw: f64, pitch: f64, roll: f64) -> Self {
        Self::from_yaw_pitch_roll_f32(
            (std::f64::consts::TAU * yaw) as f32,
            (std::f64::consts::TAU * pitch) as f32,
            (std::f64::consts::TAU * roll) as f32,
        )
    }

    fn from_yaw_pitch_roll(yaw: f64, pitch: f64, roll: f64) -> Self {
        Self::from_yaw_pitch_roll_f32(yaw as f32, pitch as f32, roll as f32)
    }

    fn from_yaw_pitch_roll_f32(yaw: f32, pitch: f32, roll: f32) -> Self {
        let (sy, cy) = (yaw * 0.5).sin_cos();
        let (sp, cp) = (pitch * 0.5).sin_cos();
        let (sr, cr) = (roll * 0.5).sin_cos();
        Self {
            x: f64::from(cy.mul_add(sp * cr, sy * cp * sr)),
            y: f64::from(sy.mul_add(cp * cr, -cy * sp * sr)),
            z: f64::from(cy.mul_add(cp * sr, -sy * sp * cr)),
            w: f64::from(cy.mul_add(cp * cr, sy * sp * sr)),
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
    idle_paintbrush_orientation_at(input.volume, paintbrush_frame_in_cycle(input))
}

fn idle_paintbrush_orientation_at(volume: f32, frame_in_cycle: u32) -> Quaternion {
    let level = f64::from(volume.clamp(0.0, 1.0));
    let mut random = DotNetRandom::new(0);
    let mut yaw = 0.0;
    let mut pitch = -0.25;
    let mut roll = 0.0;
    let mut yaw_momentum = 0.0;
    let mut pitch_momentum = 0.0005;
    let mut roll_momentum = 0.0;

    for _ in 0..=frame_in_cycle {
        yaw_momentum = (yaw_momentum + spectrum_nudge(&mut random, 0.0001)).clamp(-0.001, 0.001);
        roll_momentum = (roll_momentum + spectrum_nudge(&mut random, 0.0001)).clamp(-0.001, 0.001);
        pitch_momentum =
            (pitch_momentum + spectrum_nudge(&mut random, 0.0001)).clamp(-0.001, 0.001);

        let motion_scale = 4.0 * (level + 0.25);
        yaw += motion_scale * yaw_momentum;
        pitch += motion_scale * pitch_momentum;
        roll += motion_scale * roll_momentum;
    }

    let yaw = std::f64::consts::TAU * yaw;
    let pitch = std::f64::consts::TAU * pitch;
    let roll = std::f64::consts::TAU * roll;
    Quaternion::from_yaw_pitch_roll(yaw, pitch, roll)
}

fn paintbrush_trail_orientations(
    input: VisualizerInput,
    frame_in_cycle: u64,
) -> Vec<(Quaternion, f64)> {
    if input.orientation_override.is_some() || frame_in_cycle == 0 {
        return Vec::new();
    }

    [8_u64, 18, 32, 56, 88, 128]
        .into_iter()
        .filter(|offset| frame_in_cycle >= *offset)
        .map(|offset| {
            let frame = (frame_in_cycle - offset)
                .try_into()
                .expect("paintbrush trail frame fits in u32");
            let offset_f64 = f64::from(u32::try_from(offset).expect("trail offset fits in u32"));
            let fade = (1.0 - offset_f64 / 150.0).clamp(0.12, 0.75);
            (idle_paintbrush_orientation_at(input.volume, frame), fade)
        })
        .collect()
}

fn paintbrush_ripple_counter(frame_in_cycle: u64) -> f64 {
    const RIPPLE_COOLDOWN_FRAMES: u64 = 100;
    if frame_in_cycle <= RIPPLE_COOLDOWN_FRAMES {
        return 0.0;
    }
    let frame = frame_in_cycle - RIPPLE_COOLDOWN_FRAMES;
    if frame >= 1_000 {
        0.0
    } else {
        f64::from(u32::try_from(frame).expect("ripple frame fits in u32"))
    }
}

fn paintbrush_stamp_frame(frame_in_cycle: u64) -> Option<u64> {
    const STAMP_START_FRAMES: u64 = 1_001;
    if frame_in_cycle < STAMP_START_FRAMES {
        return None;
    }
    let frame = frame_in_cycle - STAMP_START_FRAMES;
    (frame < 90).then_some(frame)
}

fn paintbrush_stamp_ring(distance_to_spot: f64, stamp_frame: u64) -> bool {
    if stamp_frame < 45 {
        distance_to_spot.rem_euclid(0.4) < 0.05
    } else {
        let cooldown = 10.0
            - f64::from(u32::try_from(stamp_frame - 45).expect("stamp frame fits in u32")) / 4.5;
        let ring_distance = 2.4 - (1.8 / (4.0 - cooldown / 2.0)).clamp(0.0, 2.4);
        let half_width = 0.003 * cooldown * cooldown;
        (ring_distance - half_width..=ring_distance + half_width).contains(&distance_to_spot)
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Runtime visualizer animation periods are small preview-frame counters"
)]
fn runtime_visualizer_progress(input: VisualizerInput, period_frames: u64) -> f64 {
    if input.animation_frame == 0 || period_frames == 0 {
        return 0.0;
    }
    let frame = input.animation_frame % period_frames;
    (input.beat_progress + frame as f64 / period_frames as f64).rem_euclid(1.0)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Runtime visualizer animation uses small preview-frame counters"
)]
fn runtime_visualizer_progress_unwrapped(input: VisualizerInput, frames_per_cycle: u64) -> f64 {
    if input.animation_frame == 0 || frames_per_cycle == 0 {
        return 0.0;
    }
    input.animation_frame as f64 / frames_per_cycle as f64
}

fn spectrum_nudge(random: &mut DotNetRandom, scale: f64) -> f64 {
    (random.next_double() - 0.5) * 2.0 * scale
}

fn paintbrush_frame_in_cycle(input: VisualizerInput) -> u32 {
    input
        .animation_frame
        .min(u64::from(u32::MAX))
        .try_into()
        .expect("paintbrush animation frame fits in u32")
}

fn paintbrush_twinkle(input: VisualizerInput, point_index: usize, z: f64) -> Rgb {
    if input.animation_frame == 0 || z <= 0.2 {
        return Rgb::BLACK;
    }
    let frame_bucket = input.animation_frame / 3;
    let seed = i32::try_from(
        ((frame_bucket.wrapping_mul(1_103_515_245))
            ^ u64::try_from(point_index).expect("point index fits in u64"))
            % i32::MAX as u64,
    )
    .expect("twinkle seed fits in i32");
    let mut random = DotNetRandom::new(seed);
    if random.next_double() < 0.001 {
        Rgb::from_u24(0xff_ff_ff)
    } else {
        Rgb::BLACK
    }
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
        DomeDiagnosticVisualizer, LiveVisualizer, MidiNoteInput, OrientationOverride,
        StageVisualizer, StageVisualizerInput, VisualizerInput, VisualizerRuntime, INVENTORY,
        MAX_FRAME_MIDI_NOTES, MAX_ORIENTATION_DEVICES,
    };

    fn frame_colors(commands: &[DomeCommand]) -> &[domers_core::Rgb] {
        commands
            .iter()
            .find_map(|command| match command {
                DomeCommand::Frame(colors) => Some(colors.as_slice()),
                DomeCommand::Flush | DomeCommand::Pixel { .. } => None,
            })
            .expect("visualizer should write a whole preview frame")
    }

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

    #[derive(Clone, Deserialize)]
    struct ManifestInput {
        volume: f32,
        beat_progress: f64,
        flash_active: bool,
        diagnostic_state: u8,
        diagnostic_step: usize,
        palette_slot: u8,
        #[serde(default)]
        midi: Vec<MidiNoteInput>,
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
    fn spectrum_visualizer_sequence_manifest_covers_live_dome_visualizers() {
        let manifest =
            include_str!("../../../fixtures/spectrum-csharp/visualizer_sequence_cases.json");
        for visualizer in INVENTORY
            .iter()
            .filter(|visualizer| visualizer.classification == Classification::Live)
        {
            assert!(
                manifest.contains(&format!("\"name\": \"{}\"", visualizer.name)),
                "{} should have a multi-frame fixture sequence",
                visualizer.name
            );
        }
        assert_eq!(
            manifest
                .matches("\"kind\": \"frame_sequence_hashes\"")
                .count(),
            11
        );
        assert_eq!(manifest.matches("\"input_sequence\"").count(), 11);
        // Sequence goldens are captured incrementally, so the split between
        // captured and still-pending cases shifts over time; only the total
        // must stay complete.
        let captured = manifest.matches("\"status\": \"captured\"").count();
        let pending = manifest
            .matches("\"status\": \"pending_csharp_execution\"")
            .count();
        assert_eq!(captured + pending, 11);
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

    #[derive(Deserialize)]
    struct SequenceManifest {
        capture_metadata: SequenceMeta,
        cases: Vec<SequenceCase>,
    }

    #[derive(Deserialize)]
    struct SequenceMeta {
        frame_delta_ticks: i64,
        clock_base_ticks: i64,
        beat_measure_ms: u32,
    }

    #[derive(Deserialize)]
    struct SequenceCase {
        case: String,
        name: String,
        expected: SequenceExpected,
        input_sequence: Vec<ManifestInput>,
        #[serde(default)]
        frame_delta_ticks: Option<i64>,
    }

    #[derive(Deserialize)]
    struct SequenceExpected {
        status: String,
        #[serde(default)]
        frames: Vec<String>,
    }

    /// C# `TimeSpan.TicksPerMillisecond`, used to convert the capture clock's
    /// tick counter into the wall-clock milliseconds the runtime consumes.
    const TICKS_PER_MS: i64 = 10_000;

    fn live_visualizer_for_manifest_name(name: &str) -> Option<LiveVisualizer> {
        Some(match name {
            "LEDDomeVolumeVisualizer" => LiveVisualizer::Volume,
            "LEDDomeRadialVisualizer" => LiveVisualizer::Radial,
            "LEDDomeRaceVisualizer" => LiveVisualizer::Race,
            "LEDDomeSnakesVisualizer" => LiveVisualizer::Snakes,
            "LEDDomeSplatVisualizer" => LiveVisualizer::Splat,
            "LEDDomeQuaternionTestVisualizer" => LiveVisualizer::QuaternionTest,
            "LEDDomeQuaternionMultiTestVisualizer" => LiveVisualizer::QuaternionMultiTest,
            "LEDDomeQuaternionPaintbrushVisualizer" => LiveVisualizer::QuaternionPaintbrush,
            "LEDDomeTVStaticVisualizer" => LiveVisualizer::TvStatic,
            "LEDDomeFlashVisualizer" => LiveVisualizer::Flash,
            _ => return None,
        })
    }

    /// Replay one captured multi-frame Spectrum sequence through the persistent
    /// `VisualizerRuntime` and compare each frame's FNV-1a hash to the C# golden.
    /// Ignored by default: run explicitly while converging exactness.
    #[test]
    #[ignore = "run explicitly while converging Spectrum sequence goldens"]
    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "capture clock ticks and frame counts stay well within range"
    )]
    fn rust_visualizer_sequences_match_spectrum_csharp_goldens() {
        let manifest: SequenceManifest = serde_json::from_str(include_str!(
            "../../../fixtures/spectrum-csharp/visualizer_sequence_cases.json"
        ))
        .expect("sequence manifest parses");
        let config = import_spectrum_xml(include_str!(
            "../../../fixtures/config/spectrum_default_config.xml"
        ))
        .config;
        let meta = &manifest.capture_metadata;
        let mut mismatches = Vec::new();
        let mut checked = 0usize;

        for test_case in &manifest.cases {
            if test_case.expected.status != "captured" {
                continue;
            }
            let Some(visualizer) = live_visualizer_for_manifest_name(&test_case.name) else {
                // Stage/bar sequences are hashed on their own outputs, not the
                // dome runtime; they are covered elsewhere.
                continue;
            };
            checked += 1;

            let mut runtime = VisualizerRuntime::default();
            let frame_delta = test_case
                .frame_delta_ticks
                .unwrap_or(meta.frame_delta_ticks);
            for (frame_index, frame_input) in test_case.input_sequence.iter().enumerate() {
                let now_ticks = meta.clock_base_ticks + (frame_index as i64) * frame_delta;
                let now_ms = (now_ticks / TICKS_PER_MS) as u64;

                let mut input = visualizer_input(frame_input, &config);
                input.now_ms = now_ms;
                input.measure_length_ms = Some(meta.beat_measure_ms);
                input.animation_frame = u64::try_from(frame_index).expect("frame index fits");
                input.midi_notes = [None; MAX_FRAME_MIDI_NOTES];
                for (slot, note) in frame_input
                    .midi
                    .iter()
                    .enumerate()
                    .take(MAX_FRAME_MIDI_NOTES)
                {
                    input.midi_notes[slot] = Some(*note);
                }

                let commands = runtime.render_dome(visualizer, input);
                let actual = frame_hash(&commands);
                let expected = test_case
                    .expected
                    .frames
                    .get(frame_index)
                    .and_then(|value| value.parse::<u64>().ok());
                if expected != Some(actual) {
                    mismatches.push(format!(
                        "{} / {} frame {frame_index}: expected {expected:?}, got {actual}",
                        test_case.case, test_case.name
                    ));
                }
            }
        }

        assert!(checked > 0, "no captured dome sequences to verify");
        assert!(
            mismatches.is_empty(),
            "Rust sequence hashes differ from Spectrum C# goldens ({} of them):\n{}",
            mismatches.len(),
            mismatches.join("\n")
        );
    }

    fn render_manifest_case_hash(
        test_case: &VisualizerCase,
        config: &domers_core::DomersConfig,
    ) -> u64 {
        let live_input = visualizer_input(&test_case.input, config);
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
        input: &ManifestInput,
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
            now_ms: 0,
            measure_length_ms: None,
            orientation_override: None,
            orientation_devices: [None; MAX_ORIENTATION_DEVICES],
            midi_notes: [None; MAX_FRAME_MIDI_NOTES],
            flash_active: input.flash_active,
            primary: palette[0],
            secondary: palette[1],
            accent: palette[2],
            palette,
            palette_entries,
            dome_brightness: 1.0,
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
        let pixel_count = commands
            .iter()
            .filter(|command| matches!(command, DomeCommand::Pixel { .. }))
            .count();
        let lit_count = commands
            .iter()
            .filter(|command| match command {
                DomeCommand::Pixel { color, .. } => *color != domers_core::Rgb::BLACK,
                DomeCommand::Flush | DomeCommand::Frame(_) => false,
            })
            .count();

        assert!(pixel_count >= DOME_PIXELS);
        assert!(
            lit_count > 1_000,
            "volume visualizer should light a substantial part of the dome"
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
        let first = render_dome_visualizer(LiveVisualizer::TvStatic, VisualizerInput::default());
        let second = render_dome_visualizer(LiveVisualizer::TvStatic, VisualizerInput::default());

        assert_eq!(first, second);
        let pixels: Vec<_> = first
            .iter()
            .filter_map(|command| match command {
                DomeCommand::Pixel { color, .. } => Some(*color),
                DomeCommand::Flush | DomeCommand::Frame(_) => None,
            })
            .collect();
        assert_eq!(pixels.len(), DOME_PIXELS);
        assert!(pixels.windows(2).take(100).any(|pair| pair[0] != pair[1]));
        assert!(matches!(first.last(), Some(DomeCommand::Flush)));
    }

    #[test]
    fn runtime_visualizers_animate_after_captured_first_frame() {
        for visualizer in [
            LiveVisualizer::TvStatic,
            LiveVisualizer::Radial,
            LiveVisualizer::Splat,
            LiveVisualizer::Race,
        ] {
            let first_runtime = render_dome_visualizer(
                visualizer,
                VisualizerInput {
                    animation_frame: 1,
                    ..VisualizerInput::default()
                },
            );
            let later_runtime = render_dome_visualizer(
                visualizer,
                VisualizerInput {
                    animation_frame: 120,
                    ..VisualizerInput::default()
                },
            );
            assert_ne!(
                super::frame_hash(&first_runtime),
                super::frame_hash(&later_runtime),
                "{visualizer:?} should animate during live preview"
            );
        }
    }

    #[test]
    fn snakes_animate_across_throttle_steps() {
        let first = render_dome_visualizer(
            LiveVisualizer::Snakes,
            VisualizerInput {
                animation_frame: 0,
                ..VisualizerInput::default()
            },
        );
        // Same throttle window (< 50 ms) repeats the same first update.
        let same_window = render_dome_visualizer(
            LiveVisualizer::Snakes,
            VisualizerInput {
                animation_frame: super::SNAKES_STEP_FRAMES - 1,
                ..VisualizerInput::default()
            },
        );
        let next_step = render_dome_visualizer(
            LiveVisualizer::Snakes,
            VisualizerInput {
                animation_frame: super::SNAKES_STEP_FRAMES,
                ..VisualizerInput::default()
            },
        );
        assert_eq!(super::frame_hash(&first), super::frame_hash(&same_window));
        assert_ne!(
            super::frame_hash(&first),
            super::frame_hash(&next_step),
            "Snakes should advance to the next triangle after the throttle window"
        );
        // Each snake writes one triangle (3 struts) of pixels plus a trailing flush.
        assert!(matches!(first.last(), Some(DomeCommand::Flush)));
        assert!(first
            .iter()
            .any(|command| matches!(command, DomeCommand::Pixel { .. })));
    }

    #[test]
    fn snakes_move_between_connected_triangles() {
        let triangles = super::snake_triangles();
        assert_eq!(triangles.len(), super::SNAKE_TRIANGLE_DEFS.len());
        // Triangle 0 (72,71,0) must have at least one directional neighbor so the
        // snake can leave the seed triangle.
        let seed = triangles[0];
        assert!(
            seed.left.is_some()
                || seed.above.is_some()
                || seed.right.is_some()
                || seed.below.is_some(),
            "seed triangle must connect to the graph"
        );
        // Every triangle indexes valid struts and most connect to neighbors.
        let connected = triangles
            .iter()
            .filter(|t| {
                t.left.is_some() || t.above.is_some() || t.right.is_some() || t.below.is_some()
            })
            .count();
        assert!(
            connected >= triangles.len() - 1,
            "snake graph should be almost fully connected, got {connected}/{}",
            triangles.len()
        );
    }

    fn pixel_commands(commands: &[DomeCommand]) -> Vec<(usize, usize, domers_core::Rgb)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DomeCommand::Pixel {
                    strut_index,
                    led_index,
                    color,
                } => Some((*strut_index, *led_index, *color)),
                DomeCommand::Flush | DomeCommand::Frame(_) => None,
            })
            .collect()
    }

    #[test]
    fn race_racers_advance_with_wall_clock_time() {
        let mut runtime = VisualizerRuntime::default();
        let base = VisualizerInput {
            volume: 0.8,
            beat_progress: 0.25,
            now_ms: 0,
            ..VisualizerInput::default()
        };
        let first = runtime.render_dome(LiveVisualizer::Race, base);
        // A second later the volume-driven racers must have rotated.
        let later = runtime.render_dome(
            LiveVisualizer::Race,
            VisualizerInput {
                now_ms: 1_000,
                ..base
            },
        );
        assert_ne!(
            pixel_commands(&first),
            pixel_commands(&later),
            "Race racers should rotate as wall-clock time advances"
        );
    }

    #[test]
    fn tv_static_advances_persistent_rng_across_frames() {
        let mut runtime = VisualizerRuntime::default();
        let first = pixel_commands(
            &runtime.render_dome(LiveVisualizer::TvStatic, VisualizerInput::default()),
        );
        let second = pixel_commands(
            &runtime.render_dome(LiveVisualizer::TvStatic, VisualizerInput::default()),
        );
        assert_eq!(first.len(), second.len());
        assert_ne!(
            first, second,
            "TV static should keep advancing one RNG, not repeat the same frame"
        );
    }

    #[test]
    fn switch_wipes_previous_visualizer() {
        let mut runtime = VisualizerRuntime::default();
        // Snakes emits pixel deltas; the very first activation must not prepend a
        // clearing frame (nothing is on the dome yet).
        let first = runtime.render_dome(LiveVisualizer::Snakes, VisualizerInput::default());
        assert!(
            !matches!(first.first(), Some(DomeCommand::Frame(_))),
            "first activation should not emit a clearing frame"
        );
        // Switching visualizers clears the dome with a leading all-black frame.
        let switched = runtime.render_dome(LiveVisualizer::Radial, VisualizerInput::default());
        match switched.first() {
            Some(DomeCommand::Frame(colors)) => {
                assert_eq!(colors.len(), DOME_PIXELS);
                assert!(colors.iter().all(|color| *color == domers_core::Rgb::BLACK));
            }
            _ => panic!("visualizer switch should begin with a black clearing frame"),
        }
    }

    #[test]
    fn splat_spawns_on_beat_wrap() {
        let mut runtime = VisualizerRuntime::default();
        let non_black = |commands: &[DomeCommand]| {
            frame_colors(commands)
                .iter()
                .any(|color| *color != domers_core::Rgb::BLACK)
        };
        // Rising progress: fade only, no spawn yet.
        let _ = runtime.render_dome(
            LiveVisualizer::Splat,
            VisualizerInput {
                beat_progress: 0.2,
                ..VisualizerInput::default()
            },
        );
        let before_wrap = runtime.render_dome(
            LiveVisualizer::Splat,
            VisualizerInput {
                beat_progress: 0.9,
                ..VisualizerInput::default()
            },
        );
        assert!(!non_black(&before_wrap), "no splat before the beat wraps");
        // Progress wraps downward (0.9 -> 0.1): a splat spawns.
        let after_wrap = runtime.render_dome(
            LiveVisualizer::Splat,
            VisualizerInput {
                beat_progress: 0.1,
                ..VisualizerInput::default()
            },
        );
        assert!(
            non_black(&after_wrap),
            "splat should spawn when beat progress wraps"
        );
    }

    #[test]
    fn quaternion_multi_uses_orientation_devices_not_fake_idle_motion() {
        let idle = render_dome_visualizer(
            LiveVisualizer::QuaternionMultiTest,
            VisualizerInput {
                animation_frame: 120,
                ..VisualizerInput::default()
            },
        );
        let oriented = render_dome_visualizer(
            LiveVisualizer::QuaternionMultiTest,
            VisualizerInput {
                animation_frame: 120,
                orientation_override: Some(OrientationOverride {
                    yaw: 0.0,
                    pitch: 0.0,
                    roll: 0.0,
                }),
                ..VisualizerInput::default()
            },
        );

        assert!(frame_colors(&idle)
            .iter()
            .all(|color| *color == domers_core::Rgb::BLACK));
        assert!(frame_colors(&oriented)
            .iter()
            .any(|color| *color != domers_core::Rgb::BLACK));
    }

    #[test]
    fn volume_animation_uses_beat_progress_like_spectrum() {
        let first_runtime = render_dome_visualizer(
            LiveVisualizer::Volume,
            VisualizerInput {
                animation_frame: 1,
                beat_progress: 0.10,
                ..VisualizerInput::default()
            },
        );
        let later_runtime = render_dome_visualizer(
            LiveVisualizer::Volume,
            VisualizerInput {
                animation_frame: 120,
                beat_progress: 0.65,
                ..VisualizerInput::default()
            },
        );
        assert_ne!(
            super::frame_hash(&first_runtime),
            super::frame_hash(&later_runtime),
            "Volume should follow beat progress instead of a synthetic rotating shape"
        );
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
    fn quaternion_paintbrush_accumulates_spectrum_style_paint_layers() {
        let first = render_dome_visualizer(
            LiveVisualizer::QuaternionPaintbrush,
            VisualizerInput {
                animation_frame: 0,
                ..VisualizerInput::default()
            },
        );
        let later = render_dome_visualizer(
            LiveVisualizer::QuaternionPaintbrush,
            VisualizerInput {
                animation_frame: 360,
                ..VisualizerInput::default()
            },
        );
        let first_lit = frame_colors(&first)
            .iter()
            .filter(|color| **color != domers_core::Rgb::BLACK)
            .count();
        let later_lit = frame_colors(&later)
            .iter()
            .filter(|color| **color != domers_core::Rgb::BLACK)
            .count();

        assert!(
            later_lit > first_lit,
            "paintbrush should retain trailing paint and ripple layers after the captured first frame"
        );
    }

    #[test]
    fn quaternion_paintbrush_event_layers_do_not_loop_reset() {
        let early = render_dome_visualizer(
            LiveVisualizer::QuaternionPaintbrush,
            VisualizerInput {
                animation_frame: 360,
                ..VisualizerInput::default()
            },
        );
        let later = render_dome_visualizer(
            LiveVisualizer::QuaternionPaintbrush,
            VisualizerInput {
                animation_frame: 1_460,
                ..VisualizerInput::default()
            },
        );

        assert_ne!(
            super::frame_hash(&early),
            super::frame_hash(&later),
            "paintbrush ripple/stamp event layers must not loop back into an obvious reset"
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
            (LiveVisualizer::TvStatic, 7_938_821_499_849_451_788),
            (LiveVisualizer::Volume, 3_360_946_268_713_528_047),
            (LiveVisualizer::Flash, 14_695_981_039_346_656_037),
            (LiveVisualizer::Radial, 8_095_729_372_390_775_204),
            (LiveVisualizer::Splat, 12_459_070_695_921_506_308),
            (LiveVisualizer::Race, 7_871_414_923_077_219_675),
            (LiveVisualizer::Snakes, 3_377_082_443_979_724_166),
            (LiveVisualizer::QuaternionTest, 1_564_991_241_466_880_178),
            (
                LiveVisualizer::QuaternionMultiTest,
                12_459_070_695_921_506_308,
            ),
            (
                LiveVisualizer::QuaternionPaintbrush,
                5_139_703_606_261_245_084,
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

        let visualizer = LiveVisualizer::Radial;
        assert_ne!(
            super::frame_hash(&render_dome_visualizer(
                visualizer,
                VisualizerInput::default()
            )),
            super::frame_hash(&render_dome_visualizer(visualizer, custom)),
            "{visualizer:?} should use palette entries beyond Color 1-3"
        );
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

    #[test]
    fn flash_frame2_hash_for_shape_57() {
        let config = import_spectrum_xml(include_str!(
            "../../../fixtures/config/spectrum_default_config.xml"
        ))
        .config;
        let manifest: SequenceManifest = serde_json::from_str(include_str!(
            "../../../fixtures/spectrum-csharp/visualizer_sequence_cases.json"
        ))
        .unwrap();
        let case = manifest
            .cases
            .iter()
            .find(|c| c.case == "dome_flash_idle_and_active_placeholder")
            .unwrap();
        let expected = case.expected.frames[2].parse::<u64>().unwrap();
        let meta = &manifest.capture_metadata;
        let frame_input = &case.input_sequence[2];
        let start_ms = ((meta.clock_base_ticks + 500_000) / 10_000) as u64;
        let now_ms = ((meta.clock_base_ticks + 1_000_000) / 10_000) as u64;
        let mut input = visualizer_input(frame_input, &config);
        input.now_ms = now_ms;
        input.measure_length_ms = Some(meta.beat_measure_ms);

        let layout = super::concentric_layout_from_point(57, 2);
        let struts = super::flash_layout_struts(&layout);
        let shape = super::FlashShape {
            layout,
            struts,
            animation: None,
        };
        let animation =
            super::FlashPolygonAnimation::new(0, 0.8, meta.beat_measure_ms, start_ms);
        let mut commands = Vec::new();
        super::animate_flash_polygon(&shape, &animation, &input, now_ms, &mut commands);
        let actual = frame_hash(&commands);
        assert_eq!(Some(actual), Some(expected));
    }

    #[test]
    fn flash_runtime_replay_matches_direct_frame2() {
        let config = import_spectrum_xml(include_str!(
            "../../../fixtures/config/spectrum_default_config.xml"
        ))
        .config;
        let manifest: SequenceManifest = serde_json::from_str(include_str!(
            "../../../fixtures/spectrum-csharp/visualizer_sequence_cases.json"
        ))
        .unwrap();
        let case = manifest
            .cases
            .iter()
            .find(|c| c.case == "dome_flash_idle_and_active_placeholder")
            .unwrap();
        let expected = case.expected.frames[2].parse::<u64>().unwrap();
        let meta = &manifest.capture_metadata;
        let frame_delta = case.frame_delta_ticks.unwrap_or(meta.frame_delta_ticks);
        let mut runtime = VisualizerRuntime::default();
        for (frame_index, frame_input) in case.input_sequence.iter().enumerate().take(3) {
            let now_ticks = meta.clock_base_ticks + (frame_index as i64) * frame_delta;
            let now_ms = (now_ticks / 10_000) as u64;
            let mut input = visualizer_input(frame_input, &config);
            input.now_ms = now_ms;
            input.measure_length_ms = Some(meta.beat_measure_ms);
            input.midi_notes = [None; MAX_FRAME_MIDI_NOTES];
            for (slot, note) in frame_input
                .midi
                .iter()
                .enumerate()
                .take(MAX_FRAME_MIDI_NOTES)
            {
                input.midi_notes[slot] = Some(*note);
            }
            let commands = runtime.render_dome(LiveVisualizer::Flash, input);
            if frame_index == 2 {
                assert_eq!(Some(frame_hash(&commands)), Some(expected));
            }
        }
    }
}
