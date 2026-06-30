//! Runnable Domers server contract and HTTP/WebSocket adapter.

use std::{collections::VecDeque, net::SocketAddr, process::Stdio, sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::header,
    response::{Html, IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use domers_core::{
    BeatBroadcaster, ColorPalette, DomersConfig, EngineConfig, MidiBindingAction,
    MidiBindingCommandKind, MidiBindingConfig, PaletteEntry, Rgb, TempoSource,
};
use domers_engine::{schedule_operator_frame, FullVisualizerSpec, InputSpec, OutputSpec};
use domers_inputs::{
    parse_beat_line, parse_midi_payload, parse_volume_payload, MadmomLaunchConfig, MidiCommand,
    MidiCommandKind, OrientationDevice, OrientationInputState, OrientationQuaternion,
};
use domers_outputs::{
    apply_bar_commands, apply_dome_commands, apply_stage_commands, BarCommand, DomeCommand,
    OpcAddress, OpcClient, PersistentChannel, StageCommand,
};
use domers_visualizers::{
    render_bar_diagnostic, render_dome_diagnostic, render_dome_visualizer, render_stage_visualizer,
    BarDiagnosticVisualizer, DiagnosticInput, DomeDiagnosticVisualizer, LiveVisualizer,
    StageVisualizer, VisualizerInput,
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{TcpListener, UdpSocket},
    process::Command,
    sync::{broadcast, Mutex},
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};

/// Engine frame interval for the 400 Hz compute cap.
pub const ENGINE_FRAME_INTERVAL: Duration = Duration::from_micros(2_500);

/// Emit a browser simulator frame roughly every 32.5 ms while the engine runs.
pub const SIMULATOR_FRAME_STRIDE: u64 = 13;

const DOME_CONTROL_BOX_PIXEL_COUNT: usize = 214 * 8;

/// Health status returned by the early API.
#[must_use]
pub const fn health() -> &'static str {
    "ok"
}

/// Runtime metrics exposed by the server contract.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Metrics {
    /// Operator frames produced.
    pub frames: u64,
    /// Simulator frames produced.
    pub simulator_frames: u64,
}

/// Serializable server snapshot returned by the HTTP API.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ServerSnapshot {
    /// Whether the engine loop is currently running.
    pub running: bool,
    /// Active engine config.
    pub config: EngineConfig,
    /// Runtime counters.
    pub metrics: Metrics,
    /// Simulator input and palette controls.
    pub simulator: SimulatorControls,
    /// Runtime diagnostic/test-pattern controls.
    pub diagnostics: DiagnosticControls,
    /// Hardware output connection status.
    pub hardware: HardwareStatus,
    /// Live input status and latest runtime values.
    pub inputs: InputStatus,
}

/// Diagnostic/test-pattern controls exposed through `/api/state`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DiagnosticControls {
    /// Active dome diagnostic pattern.
    pub dome_test_pattern: u8,
    /// Active bar diagnostic pattern.
    pub bar_test_pattern: u8,
    /// Active stage diagnostic pattern.
    pub stage_test_pattern: u8,
}

/// Live input status exposed through `/api/state`.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct InputStatus {
    /// Last live audio volume, if any.
    pub volume: Option<f32>,
    /// Current beat length in milliseconds, if known.
    pub beat_ms: Option<u64>,
    /// Current beat progress in `[0.0, 1.0)`.
    pub beat_progress: f64,
    /// Number of tap-tempo taps accepted.
    pub taps: u64,
    /// Number of parsed Madmom beat lines accepted.
    pub madmom_beats: u64,
    /// Number of MIDI commands applied.
    pub midi_commands: u64,
    /// Recent MIDI command/action log.
    pub midi_log: Vec<MidiLogEntry>,
    /// Last recognized orientation datagram kind.
    pub last_orientation: Option<String>,
    /// Active orientation devices.
    pub orientation_devices: Vec<OrientationDeviceStatus>,
    /// Live audio adapter status.
    pub audio_adapter: InputAdapterStatus,
    /// Live MIDI adapter status.
    pub midi_adapter: InputAdapterStatus,
    /// Live orientation adapter status.
    pub orientation_adapter: InputAdapterStatus,
    /// Managed Madmom sidecar status.
    pub madmom_adapter: InputAdapterStatus,
}

/// Browser-facing MIDI log entry.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MidiLogEntry {
    /// Runtime timestamp when the command was applied.
    pub timestamp_ms: u64,
    /// Command kind.
    pub kind: String,
    /// Note/controller/program index.
    pub index: u8,
    /// Normalized command value.
    pub value: f32,
    /// Actions triggered by this command.
    pub actions: Vec<String>,
}

/// Browser-facing orientation quaternion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct OrientationQuaternionStatus {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
    /// Z component.
    pub z: f32,
    /// W component.
    pub w: f32,
}

impl From<OrientationQuaternion> for OrientationQuaternionStatus {
    fn from(value: OrientationQuaternion) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
            w: value.w,
        }
    }
}

/// Browser-facing active orientation device state.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OrientationDeviceStatus {
    /// Device id from the datagram.
    pub device_id: u8,
    /// Last accepted device timestamp.
    pub timestamp: i32,
    /// Device kind name.
    pub kind: String,
    /// Raw Spectrum device type.
    pub device_type: u8,
    /// Current orientation.
    pub current_orientation: OrientationQuaternionStatus,
    /// Calibration origin.
    pub calibration_origin: OrientationQuaternionStatus,
    /// Calibrated current rotation.
    pub current_rotation: OrientationQuaternionStatus,
    /// Current action flag.
    pub action_flag: u8,
    /// Whether this device reports angular speed.
    pub has_speed: bool,
    /// Last short-window angular speed value.
    pub avg_distance_short: f64,
}

impl From<OrientationDevice> for OrientationDeviceStatus {
    fn from(value: OrientationDevice) -> Self {
        Self {
            device_id: value.device_id,
            timestamp: value.timestamp,
            kind: format!("{:?}", value.kind),
            device_type: value.device_type,
            current_orientation: value.current_orientation.into(),
            calibration_origin: value.calibration_origin.into(),
            current_rotation: value.current_rotation().into(),
            action_flag: value.action_flag,
            has_speed: value.has_speed,
            avg_distance_short: value.avg_distance_short,
        }
    }
}

/// Browser-facing live input adapter status.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct InputAdapterStatus {
    /// Whether this adapter is configured.
    pub enabled: bool,
    /// Bind address or command used by the adapter.
    pub target: Option<String>,
    /// Number of accepted events from this adapter.
    pub events: u64,
    /// Last lifecycle or parse error.
    pub last_error: Option<String>,
}

/// Hardware output status exposed through `/api/state`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct HardwareStatus {
    /// Dome/bar OPC target status.
    pub dome: HardwareTargetStatus,
    /// Stage OPC target status.
    pub stage: HardwareTargetStatus,
}

/// One hardware target status.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct HardwareTargetStatus {
    /// Whether this target is enabled by config.
    pub enabled: bool,
    /// Last configured address.
    pub address: Option<String>,
    /// Whether a TCP client is currently connected.
    pub connected: bool,
    /// Number of frames successfully written to the TCP target.
    pub frames_sent: u64,
    /// Last connection or write error, if any.
    pub last_error: Option<String>,
}

/// Browser-facing simulator frame.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SimulatorFrame {
    /// Metrics after this frame was produced.
    pub metrics: Metrics,
    /// Dome simulator commands for the frame.
    pub commands: Vec<SimulatorCommand>,
    /// Bar simulator commands for the frame.
    pub bar_commands: Vec<BarSimulatorCommand>,
    /// Stage simulator commands for the frame.
    pub stage_commands: Vec<StageSimulatorCommand>,
}

/// Full operator frame produced by the runtime scheduler.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OperatorCommandFrame {
    /// Active input names selected for update.
    pub active_inputs: Vec<&'static str>,
    /// Active visualizer names selected for update.
    pub active_visualizers: Vec<&'static str>,
    /// Active output names selected for update.
    pub active_outputs: Vec<&'static str>,
    /// Dome command stream for this frame.
    pub dome: Vec<DomeCommand>,
    /// Bar command stream for this frame.
    pub bar: Vec<BarCommand>,
    /// Stage command stream for this frame.
    pub stage: Vec<StageCommand>,
}

/// Browser-facing simulator command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SimulatorCommand {
    /// Flush/redraw marker.
    Flush,
    /// Whole dome frame encoded as `0xRRGGBB` colors.
    Frame {
        /// Colors in canonical strut-major order.
        colors: Vec<u32>,
    },
    /// Single logical dome pixel write.
    Pixel {
        /// Strut index.
        strut_index: usize,
        /// LED index within the strut.
        led_index: usize,
        /// Color encoded as `0xRRGGBB`.
        color: u32,
    },
}

/// Browser-facing bar simulator command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BarSimulatorCommand {
    /// Flush marker.
    Flush,
    /// Bar pixel write.
    Pixel {
        /// Whether the pixel is on the runner strip.
        is_runner: bool,
        /// Logical LED index.
        led_index: usize,
        /// RGB color encoded as `0xRRGGBB`.
        color: u32,
    },
}

/// Browser-facing stage simulator command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StageSimulatorCommand {
    /// Flush marker.
    Flush,
    /// Stage pixel write.
    Pixel {
        /// Side index.
        side_index: usize,
        /// LED index.
        led_index: usize,
        /// Layer index.
        layer_index: usize,
        /// RGB color encoded as `0xRRGGBB`.
        color: u32,
    },
}

/// Operator-controlled simulator inputs.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SimulatorControls {
    /// Normalized audio volume preview.
    pub volume: f32,
    /// Beat phase preview in `[0.0, 1.0)`.
    pub beat_progress: f64,
    /// Whether the flash overlay is active.
    pub flash_active: bool,
}

impl Default for SimulatorControls {
    fn default() -> Self {
        Self {
            volume: 0.7,
            beat_progress: 0.25,
            flash_active: false,
        }
    }
}

impl SimulatorControls {
    fn visualizer_input(self, config: &EngineConfig) -> VisualizerInput {
        let palette = std::array::from_fn(|index| {
            config
                .color_palette
                .single_color(index, config.color_palette_index)
        });
        VisualizerInput {
            volume: self.volume,
            beat_progress: self.beat_progress,
            flash_active: self.flash_active,
            primary: palette[0],
            secondary: palette[1],
            accent: palette[2],
            palette,
        }
    }
}

/// In-process server state shared by HTTP handlers and the engine task.
#[derive(Clone, Debug, Default)]
pub struct ServerState {
    config: DomersConfig,
    simulator: SimulatorControls,
    inputs: InputRuntime,
    metrics: Metrics,
    running: bool,
}

impl ServerState {
    /// Create server state from an engine config.
    #[must_use]
    pub fn new(config: DomersConfig) -> Self {
        Self {
            config,
            simulator: SimulatorControls {
                volume: 0.7,
                beat_progress: 0.25,
                flash_active: false,
            },
            inputs: InputRuntime::default(),
            metrics: Metrics {
                frames: 0,
                simulator_frames: 0,
            },
            running: false,
        }
    }

    /// Return a config snapshot.
    #[must_use]
    pub fn config(&self) -> EngineConfig {
        EngineConfig::from(&self.config)
    }

    /// Return a full config snapshot.
    #[must_use]
    pub fn full_config(&self) -> DomersConfig {
        self.config.clone()
    }

    /// Replace the full native config.
    pub fn replace_full_config(&mut self, config: DomersConfig) {
        self.config = config;
    }

    /// Patch dome runtime configuration.
    pub fn patch_dome_config(&mut self, patch: DomeConfigPatch) {
        if let Some(active_visualizer) = patch.active_visualizer {
            self.config.dome.active_visualizer = active_visualizer;
        }
        if let Some(flash_speed) = patch.flash_speed {
            self.config.tempo.flash_speed = flash_speed.clamp(0.0, 32.0);
        }
        if let Some(color_palette_index) = patch.color_palette_index {
            self.config.color_palette_index = color_palette_index.min(7);
        }
    }

    /// Patch runtime diagnostic/test-pattern controls.
    pub fn patch_diagnostics(&mut self, patch: DiagnosticConfigPatch) {
        if let Some(dome_test_pattern) = patch.dome_test_pattern {
            self.config.dome.test_pattern = dome_test_pattern.min(4);
        }
        if let Some(bar_test_pattern) = patch.bar_test_pattern {
            self.config.bar.test_pattern = bar_test_pattern.min(1);
        }
        if let Some(stage_test_pattern) = patch.stage_test_pattern {
            self.config.stage.test_pattern = stage_test_pattern.min(1);
        }
    }

    /// Patch one runtime color palette entry.
    pub fn patch_palette_entry(&mut self, patch: PaletteEntryPatch) {
        let color_palette_index = patch
            .color_palette_index
            .unwrap_or(self.config.color_palette_index)
            .min(7);
        let absolute_index = ColorPalette::absolute_index(
            usize::from(patch.relative_index.min(7)),
            color_palette_index,
        );
        if self.config.color_palette.colors.len() <= absolute_index {
            self.config
                .color_palette
                .colors
                .resize(absolute_index + 1, PaletteEntry::default());
        }
        self.config.color_palette.colors[absolute_index] = if patch.color2_enabled.unwrap_or(false)
        {
            PaletteEntry::gradient(patch.color1, patch.color2.unwrap_or(patch.color1))
        } else {
            PaletteEntry::solid(patch.color1)
        };
    }

    /// Patch simulator input controls.
    pub fn patch_simulator_controls(&mut self, patch: SimulatorControlsPatch) {
        if let Some(volume) = patch.volume {
            self.simulator.volume = volume.clamp(0.0, 1.0);
        }
        if let Some(beat_progress) = patch.beat_progress {
            self.simulator.beat_progress = beat_progress.clamp(0.0, 1.0);
        }
        if let Some(flash_active) = patch.flash_active {
            self.simulator.flash_active = flash_active;
        }
    }

    /// Start the engine loop.
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Stop the engine loop.
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Whether the engine is running.
    #[must_use]
    pub const fn running(&self) -> bool {
        self.running
    }

    /// Current metrics snapshot.
    #[must_use]
    pub const fn metrics(&self) -> Metrics {
        self.metrics
    }

    /// Record one engine compute frame without emitting a simulator frame.
    pub fn engine_frame(&mut self) {
        self.metrics.frames = self.metrics.frames.saturating_add(1);
    }

    /// Apply one live audio volume sample.
    pub fn apply_audio_volume(&mut self, volume: f32) {
        self.inputs.volume = Some(volume.clamp(0.0, 1.0));
        self.inputs.audio_adapter.events = self.inputs.audio_adapter.events.saturating_add(1);
        self.inputs.audio_adapter.last_error = None;
    }

    /// Record a human tap-tempo event at the current runtime timestamp.
    pub fn tap_tempo(&mut self) {
        self.inputs.taps = self.inputs.taps.saturating_add(1);
        self.inputs.beat.add_tap(self.now_ms());
    }

    /// Parse and record a Madmom `BEAT:` stdout line.
    pub fn report_madmom_line(&mut self, line: &str) -> bool {
        if let Some(beat_ms) = parse_beat_line(line) {
            self.inputs.madmom_beats = self.inputs.madmom_beats.saturating_add(1);
            self.inputs.madmom_adapter.events = self.inputs.madmom_adapter.events.saturating_add(1);
            self.inputs.madmom_adapter.last_error = None;
            self.inputs.beat.report_madmom_beat(beat_ms, self.now_ms());
            true
        } else {
            self.inputs.madmom_adapter.last_error = Some("malformed BEAT line".to_string());
            false
        }
    }

    /// Apply MIDI commands to runtime state.
    pub fn apply_midi_commands(&mut self, commands: &[MidiCommand]) {
        self.inputs.midi_commands = self
            .inputs
            .midi_commands
            .saturating_add(commands.len() as u64);
        if !commands.is_empty() {
            self.inputs.midi_adapter.events = self
                .inputs
                .midi_adapter
                .events
                .saturating_add(commands.len() as u64);
            self.inputs.midi_adapter.last_error = None;
        }
        let bindings = self.config.inputs.midi.bindings.clone();
        for command in commands {
            let mut actions = Vec::new();
            for binding in &bindings {
                if midi_binding_matches(binding, *command) {
                    let action = self.apply_midi_binding(binding, *command);
                    actions.push(action);
                }
            }
            self.record_midi_log(*command, actions);
        }
    }

    fn apply_midi_binding(&mut self, binding: &MidiBindingConfig, command: MidiCommand) -> String {
        match binding.action {
            MidiBindingAction::Flash => {
                self.simulator.flash_active = command.value > 0.0;
                "flash".to_string()
            }
            MidiBindingAction::Volume => {
                self.simulator.volume = command.value.clamp(0.0, 1.0);
                "volume".to_string()
            }
            MidiBindingAction::TapTempo => {
                if command.value > 0.0 {
                    self.tap_tempo();
                }
                "tap_tempo".to_string()
            }
            MidiBindingAction::Palette => {
                let index = binding
                    .target_index
                    .unwrap_or_else(|| scaled_index(command.value, 8));
                self.config.color_palette_index = index.min(7);
                format!("palette:{}", self.config.color_palette_index)
            }
            MidiBindingAction::Visualizer => {
                let index = binding
                    .target_index
                    .unwrap_or_else(|| scaled_index(command.value, 9));
                self.config.dome.active_visualizer = index.min(8);
                format!("visualizer:{}", self.config.dome.active_visualizer)
            }
        }
    }

    fn record_midi_log(&mut self, command: MidiCommand, actions: Vec<String>) {
        const MAX_MIDI_LOG_ENTRIES: usize = 32;
        if self.inputs.midi_log.len() == MAX_MIDI_LOG_ENTRIES {
            self.inputs.midi_log.pop_front();
        }
        self.inputs.midi_log.push_back(MidiLogEntry {
            timestamp_ms: self.now_ms(),
            kind: format!("{:?}", command.kind),
            index: command.index,
            value: command.value,
            actions,
        });
    }

    /// Classify and record one orientation datagram.
    pub fn apply_orientation_datagram(&mut self, bytes: &[u8]) -> bool {
        if let Some(kind) = self
            .inputs
            .orientation
            .process_datagram(bytes, self.now_ms())
        {
            self.inputs.last_orientation = Some(format!("{kind:?}"));
            self.inputs.orientation_adapter.events =
                self.inputs.orientation_adapter.events.saturating_add(1);
            self.inputs.orientation_adapter.last_error = None;
            true
        } else {
            self.inputs.orientation_adapter.last_error =
                Some("unrecognized orientation datagram".to_string());
            false
        }
    }

    /// Calibrate all active orientation devices.
    pub fn calibrate_orientation_devices(&mut self) {
        self.inputs.orientation.calibrate_all();
    }

    /// Configure live input adapter targets for status reporting.
    pub fn configure_input_adapters(&mut self, config: &DomersConfig) {
        self.inputs.audio_adapter.enabled = config.inputs.audio.bind.is_some();
        self.inputs
            .audio_adapter
            .target
            .clone_from(&config.inputs.audio.bind);
        self.inputs.midi_adapter.enabled = config.inputs.midi.bind.is_some();
        self.inputs
            .midi_adapter
            .target
            .clone_from(&config.inputs.midi.bind);
        self.inputs.orientation_adapter.enabled = config.inputs.orientation.bind.is_some();
        self.inputs
            .orientation_adapter
            .target
            .clone_from(&config.inputs.orientation.bind);
        self.inputs.madmom_adapter.enabled = matches!(config.tempo.source, TempoSource::Madmom);
        self.inputs.madmom_adapter.target = if self.inputs.madmom_adapter.enabled {
            Some(config.madmom.command.clone())
        } else {
            None
        };
    }

    fn record_input_adapter_error(&mut self, adapter: InputAdapter, error: impl Into<String>) {
        let status = match adapter {
            InputAdapter::Audio => &mut self.inputs.audio_adapter,
            InputAdapter::Midi => &mut self.inputs.midi_adapter,
            InputAdapter::Orientation => &mut self.inputs.orientation_adapter,
            InputAdapter::Madmom => &mut self.inputs.madmom_adapter,
        };
        status.enabled = true;
        status.last_error = Some(error.into());
    }

    /// Produce one deterministic simulator frame for the selected visualizer.
    pub fn simulator_frame(&mut self) -> OperatorCommandFrame {
        self.metrics.frames = self.metrics.frames.saturating_add(1);
        self.metrics.simulator_frames = self.metrics.simulator_frames.saturating_add(1);
        self.operator_frame()
    }

    fn record_simulator_frame(&mut self) {
        self.metrics.simulator_frames = self.metrics.simulator_frames.saturating_add(1);
    }

    /// Produce one scheduled operator frame for all outputs.
    #[must_use]
    pub fn operator_frame(&self) -> OperatorCommandFrame {
        render_operator_frame(
            &self.config,
            self.visualizer_controls(),
            self.metrics.frames,
        )
    }

    /// Return a serializable snapshot.
    #[must_use]
    pub fn snapshot(&self) -> ServerSnapshot {
        ServerSnapshot {
            running: self.running,
            config: EngineConfig::from(&self.config),
            metrics: self.metrics,
            simulator: self.simulator,
            diagnostics: self.diagnostic_controls(),
            hardware: HardwareStatus::default(),
            inputs: self.input_status(),
        }
    }

    fn diagnostic_controls(&self) -> DiagnosticControls {
        DiagnosticControls {
            dome_test_pattern: self.config.dome.test_pattern,
            bar_test_pattern: self.config.bar.test_pattern,
            stage_test_pattern: self.config.stage.test_pattern,
        }
    }

    fn now_ms(&self) -> u64 {
        self.metrics.frames.saturating_mul(2_500) / 1_000
    }

    fn prune_input_state(&mut self) {
        self.inputs.orientation.remove_stale_devices(self.now_ms());
    }

    fn visualizer_controls(&self) -> SimulatorControls {
        let mut controls = self.simulator;
        if let Some(volume) = self.inputs.volume {
            controls.volume = volume;
        } else if self.running {
            controls.volume = animated_volume(self.now_ms());
        }
        if self.inputs.beat.beat_ms().is_some() {
            controls.beat_progress = self.inputs.beat.progress(self.now_ms(), 1.0);
        } else if self.running {
            controls.beat_progress = animated_beat_progress(self.now_ms());
        }
        controls
    }

    fn input_status(&self) -> InputStatus {
        InputStatus {
            volume: self.inputs.volume,
            beat_ms: self.inputs.beat.beat_ms(),
            beat_progress: self.inputs.beat.progress(self.now_ms(), 1.0),
            taps: self.inputs.taps,
            madmom_beats: self.inputs.madmom_beats,
            midi_commands: self.inputs.midi_commands,
            midi_log: self.inputs.midi_log.iter().cloned().collect(),
            last_orientation: self.inputs.last_orientation.clone(),
            orientation_devices: self
                .inputs
                .orientation
                .devices()
                .into_iter()
                .map(OrientationDeviceStatus::from)
                .collect(),
            audio_adapter: self.inputs.audio_adapter.clone(),
            midi_adapter: self.inputs.midi_adapter.clone(),
            orientation_adapter: self.inputs.orientation_adapter.clone(),
            madmom_adapter: self.inputs.madmom_adapter.clone(),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct InputRuntime {
    volume: Option<f32>,
    beat: BeatBroadcaster,
    taps: u64,
    madmom_beats: u64,
    midi_commands: u64,
    midi_log: VecDeque<MidiLogEntry>,
    last_orientation: Option<String>,
    orientation: OrientationInputState,
    audio_adapter: InputAdapterStatus,
    midi_adapter: InputAdapterStatus,
    orientation_adapter: InputAdapterStatus,
    madmom_adapter: InputAdapterStatus,
}

#[derive(Clone, Copy)]
enum InputAdapter {
    Audio,
    Midi,
    Orientation,
    Madmom,
}

fn midi_binding_matches(binding: &MidiBindingConfig, command: MidiCommand) -> bool {
    binding.index == command.index && midi_kind_matches(binding.command_kind, command.kind)
}

fn midi_kind_matches(binding: MidiBindingCommandKind, command: MidiCommandKind) -> bool {
    matches!(
        (binding, command),
        (MidiBindingCommandKind::Note, MidiCommandKind::Note)
            | (
                MidiBindingCommandKind::ControlChange,
                MidiCommandKind::ControlChange
            )
            | (MidiBindingCommandKind::Program, MidiCommandKind::Program)
    )
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "MIDI values are clamped to a small fixed UI index range"
)]
fn scaled_index(value: f32, count: u8) -> u8 {
    if count == 0 {
        return 0;
    }
    let last = count - 1;
    (value.clamp(0.0, 1.0) * f32::from(last)).round() as u8
}

/// Shared runnable app runtime.
#[derive(Clone)]
pub struct AppRuntime {
    state: Arc<Mutex<ServerState>>,
    hardware: Arc<Mutex<HardwareOutputs>>,
    frames: broadcast::Sender<SimulatorFrame>,
    engine_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    input_tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl Default for AppRuntime {
    fn default() -> Self {
        Self::new(DomersConfig::default())
    }
}

impl AppRuntime {
    /// Create a runtime from an engine config.
    #[must_use]
    pub fn new(config: DomersConfig) -> Self {
        let (frames, _) = broadcast::channel(32);
        let mut state = ServerState::new(config);
        let config = state.full_config();
        state.configure_input_adapters(&config);
        Self {
            state: Arc::new(Mutex::new(state)),
            hardware: Arc::new(Mutex::new(HardwareOutputs::default())),
            frames,
            engine_task: Arc::new(Mutex::new(None)),
            input_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Build the HTTP/WebSocket router.
    pub fn router(self) -> Router {
        Router::new()
            .route("/", get(index_html))
            .route("/simulator", get(simulator_html))
            .route("/main.mjs", get(main_js))
            .route("/api/health", get(health_json))
            .route("/api/state", get(get_state))
            .route("/api/start", post(start_engine))
            .route("/api/stop", post(stop_engine))
            .route(
                "/api/config",
                get(get_full_config).patch(replace_full_config),
            )
            .route("/api/config/dome", patch(patch_dome_config))
            .route("/api/config/diagnostics", patch(patch_diagnostics))
            .route("/api/config/palette", patch(patch_palette_entry))
            .route("/api/input/tap", post(tap_tempo))
            .route(
                "/api/input/orientation/calibrate",
                post(calibrate_orientation),
            )
            .route("/api/dome/geometry", get(dome_geometry))
            .route("/api/dome/mapping", get(dome_mapping))
            .route("/api/simulator", patch(patch_simulator_controls))
            .route("/api/simulator/frame", get(simulator_preview_frame))
            .route(
                "/api/simulator/sandbox-frame",
                post(simulator_sandbox_frame),
            )
            .route("/ws/simulator", get(simulator_websocket))
            .with_state(self)
    }

    /// Subscribe to simulator frames.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<SimulatorFrame> {
        self.frames.subscribe()
    }

    /// Return a server snapshot.
    pub async fn snapshot(&self) -> ServerSnapshot {
        let (mut snapshot, config) = {
            let mut state = self.state.lock().await;
            state.prune_input_state();
            (state.snapshot(), state.full_config())
        };
        snapshot.hardware = self.hardware.lock().await.status_for_config(&config);
        snapshot
    }

    /// Start the engine task if it is not already running.
    pub async fn start(&self) {
        let config = {
            let mut state = self.state.lock().await;
            state.start();
            let config = state.full_config();
            state.configure_input_adapters(&config);
            config
        };

        let mut task = self.engine_task.lock().await;
        if task.as_ref().is_some_and(|handle| !handle.is_finished()) {
            return;
        }

        self.start_input_tasks(&config).await;

        let runtime = self.clone();
        *task = Some(tokio::spawn(async move {
            runtime.run_engine_loop().await;
        }));
    }

    /// Stop the engine task.
    pub async fn stop(&self) {
        let config = {
            let mut state = self.state.lock().await;
            let config = state.full_config();
            state.stop();
            config
        };
        self.hardware.lock().await.blackout(&config).await;
        if let Some(task) = self.engine_task.lock().await.take() {
            task.abort();
        }
        self.stop_input_tasks().await;
    }

    async fn start_input_tasks(&self, config: &DomersConfig) {
        self.stop_input_tasks().await;
        let mut tasks = self.input_tasks.lock().await;
        if let Some(bind) = &config.inputs.audio.bind {
            tasks.push(spawn_audio_udp_task(self.state.clone(), bind.clone()));
        }
        if let Some(bind) = &config.inputs.midi.bind {
            tasks.push(spawn_midi_udp_task(self.state.clone(), bind.clone()));
        }
        if let Some(bind) = &config.inputs.orientation.bind {
            tasks.push(spawn_orientation_udp_task(self.state.clone(), bind.clone()));
        }
        if matches!(config.tempo.source, TempoSource::Madmom) {
            tasks.push(spawn_madmom_task(self.state.clone(), config.madmom.clone()));
        }
    }

    async fn stop_input_tasks(&self) {
        for task in self.input_tasks.lock().await.drain(..) {
            task.abort();
        }
    }

    /// Patch dome runtime configuration.
    pub async fn patch_dome_config(&self, patch: DomeConfigPatch) {
        self.state.lock().await.patch_dome_config(patch);
    }

    /// Patch runtime diagnostic/test-pattern controls.
    pub async fn patch_diagnostics(&self, patch: DiagnosticConfigPatch) {
        self.state.lock().await.patch_diagnostics(patch);
    }

    /// Patch one runtime color palette entry.
    pub async fn patch_palette_entry(&self, patch: PaletteEntryPatch) {
        self.state.lock().await.patch_palette_entry(patch);
    }

    /// Patch simulator input controls.
    pub async fn patch_simulator_controls(&self, patch: SimulatorControlsPatch) {
        self.state.lock().await.patch_simulator_controls(patch);
    }

    /// Record a tap-tempo input.
    pub async fn tap_tempo(&self) -> ServerSnapshot {
        let mut state = self.state.lock().await;
        state.tap_tempo();
        state.snapshot()
    }

    /// Return the full native config.
    pub async fn full_config(&self) -> DomersConfig {
        self.state.lock().await.full_config()
    }

    /// Replace the full native config and restart input adapters if needed.
    pub async fn replace_full_config(&self, config: DomersConfig) -> ServerSnapshot {
        let was_running = {
            let state = self.state.lock().await;
            state.running()
        };
        if was_running {
            self.stop_input_tasks().await;
        }
        {
            let mut state = self.state.lock().await;
            state.replace_full_config(config.clone());
            state.configure_input_adapters(&config);
        }
        if was_running {
            self.start_input_tasks(&config).await;
        }
        self.snapshot().await
    }

    /// Calibrate all active orientation devices.
    pub async fn calibrate_orientation_devices(&self) -> ServerSnapshot {
        let mut state = self.state.lock().await;
        state.calibrate_orientation_devices();
        state.snapshot()
    }

    /// Produce one simulator frame immediately.
    pub async fn simulator_frame(&self) -> SimulatorFrame {
        let mut state = self.state.lock().await;
        let frame = state.simulator_frame();
        SimulatorFrame {
            metrics: state.metrics(),
            commands: serialize_commands(frame.dome),
            bar_commands: serialize_bar_commands(frame.bar),
            stage_commands: serialize_stage_commands(frame.stage),
        }
    }

    /// Produce one simulator-only frame without mutating runtime config or controls.
    pub async fn simulator_sandbox_frame(
        &self,
        request: SimulatorSandboxRequest,
    ) -> SimulatorFrame {
        let state = self.state.lock().await;
        let commands = render_dome_visualizer(
            visualizer_from_index(request.active_visualizer.unwrap_or(0)),
            request.visualizer_input(),
        );
        SimulatorFrame {
            metrics: state.metrics(),
            commands: serialize_commands(commands),
            bar_commands: Vec::new(),
            stage_commands: Vec::new(),
        }
    }

    async fn run_engine_loop(self) {
        let mut interval = time::interval(ENGINE_FRAME_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut frame_count = 0_u64;

        loop {
            interval.tick().await;
            frame_count = frame_count.saturating_add(1);

            let (config, operator_frame, maybe_frame) = {
                let mut state = self.state.lock().await;
                if !state.running() {
                    return;
                }
                state.engine_frame();
                let config = state.full_config();
                let operator_frame = state.operator_frame();

                #[allow(
                    clippy::manual_is_multiple_of,
                    reason = "The is_multiple_of method is newer than the workspace MSRV"
                )]
                let maybe_frame = if frame_count % SIMULATOR_FRAME_STRIDE == 0 {
                    state.record_simulator_frame();
                    Some(SimulatorFrame {
                        metrics: state.metrics(),
                        commands: serialize_commands(operator_frame.dome.clone()),
                        bar_commands: serialize_bar_commands(operator_frame.bar.clone()),
                        stage_commands: serialize_stage_commands(operator_frame.stage.clone()),
                    })
                } else {
                    None
                };
                (config, operator_frame, maybe_frame)
            };

            #[allow(
                clippy::manual_is_multiple_of,
                reason = "The is_multiple_of method is newer than the workspace MSRV"
            )]
            if frame_count % 2 == 0 {
                self.hardware
                    .lock()
                    .await
                    .send_operator_frame(&config, &operator_frame)
                    .await;
            }

            if let Some(frame) = maybe_frame {
                let _ = self.frames.send(frame);
            }
        }
    }
}

fn spawn_audio_udp_task(state: Arc<Mutex<ServerState>>, bind: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        let socket = match UdpSocket::bind(&bind).await {
            Ok(socket) => socket,
            Err(error) => {
                state
                    .lock()
                    .await
                    .record_input_adapter_error(InputAdapter::Audio, error.to_string());
                return;
            }
        };
        let mut buffer = [0_u8; 128];
        loop {
            match socket.recv_from(&mut buffer).await {
                Ok((len, _)) => {
                    if let Some(volume) = parse_volume_payload(&buffer[..len]) {
                        state.lock().await.apply_audio_volume(volume);
                    } else {
                        state.lock().await.record_input_adapter_error(
                            InputAdapter::Audio,
                            "malformed audio volume payload",
                        );
                    }
                }
                Err(error) => {
                    state
                        .lock()
                        .await
                        .record_input_adapter_error(InputAdapter::Audio, error.to_string());
                }
            }
        }
    })
}

fn spawn_midi_udp_task(state: Arc<Mutex<ServerState>>, bind: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        let socket = match UdpSocket::bind(&bind).await {
            Ok(socket) => socket,
            Err(error) => {
                state
                    .lock()
                    .await
                    .record_input_adapter_error(InputAdapter::Midi, error.to_string());
                return;
            }
        };
        let mut buffer = [0_u8; 128];
        loop {
            match socket.recv_from(&mut buffer).await {
                Ok((len, _)) => {
                    if let Some(command) = parse_midi_payload(&buffer[..len]) {
                        state.lock().await.apply_midi_commands(&[command]);
                    } else {
                        state.lock().await.record_input_adapter_error(
                            InputAdapter::Midi,
                            "malformed MIDI payload",
                        );
                    }
                }
                Err(error) => {
                    state
                        .lock()
                        .await
                        .record_input_adapter_error(InputAdapter::Midi, error.to_string());
                }
            }
        }
    })
}

fn spawn_orientation_udp_task(state: Arc<Mutex<ServerState>>, bind: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        let socket = match UdpSocket::bind(&bind).await {
            Ok(socket) => socket,
            Err(error) => {
                state
                    .lock()
                    .await
                    .record_input_adapter_error(InputAdapter::Orientation, error.to_string());
                return;
            }
        };
        let mut buffer = [0_u8; 512];
        loop {
            match socket.recv_from(&mut buffer).await {
                Ok((len, _)) => {
                    state
                        .lock()
                        .await
                        .apply_orientation_datagram(&buffer[..len]);
                }
                Err(error) => {
                    state
                        .lock()
                        .await
                        .record_input_adapter_error(InputAdapter::Orientation, error.to_string());
                }
            }
        }
    })
}

fn spawn_madmom_task(
    state: Arc<Mutex<ServerState>>,
    config: domers_core::MadmomConfig,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let launch = MadmomLaunchConfig {
            command: config.command,
            tracker: config.tracker,
            audio_input_index: config.audio_input_index,
        };
        let mut child = match Command::new(&launch.command)
            .args(launch.args())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                state
                    .lock()
                    .await
                    .record_input_adapter_error(InputAdapter::Madmom, error.to_string());
                return;
            }
        };
        let Some(stdout) = child.stdout.take() else {
            state
                .lock()
                .await
                .record_input_adapter_error(InputAdapter::Madmom, "Madmom stdout unavailable");
            let _ = child.kill().await;
            return;
        };
        let mut lines = BufReader::new(stdout).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    state.lock().await.report_madmom_line(&line);
                }
                Ok(None) => break,
                Err(error) => {
                    state
                        .lock()
                        .await
                        .record_input_adapter_error(InputAdapter::Madmom, error.to_string());
                    break;
                }
            }
        }
        if let Err(error) = child.wait().await {
            state
                .lock()
                .await
                .record_input_adapter_error(InputAdapter::Madmom, error.to_string());
        }
    })
}
#[derive(Debug, Default)]
struct HardwareOutputs {
    dome: HardwareTarget,
    stage: HardwareTarget,
    dome_channel: PersistentChannel,
    stage_channel: PersistentChannel,
}

impl HardwareOutputs {
    fn status(&self) -> HardwareStatus {
        HardwareStatus {
            dome: self.dome.status.clone(),
            stage: self.stage.status.clone(),
        }
    }

    fn status_for_config(&self, config: &DomersConfig) -> HardwareStatus {
        let mut status = self.status();
        status.dome.enabled = config.dome.enabled || config.bar.enabled;
        status.dome.address = Some(config.dome.opc_address.clone());
        status.stage.enabled = config.stage.enabled;
        status.stage.address = Some(config.stage.opc_address.clone());
        status
    }

    async fn send_operator_frame(&mut self, config: &DomersConfig, frame: &OperatorCommandFrame) {
        if config.dome.enabled {
            apply_dome_commands(&mut self.dome_channel, &frame.dome);
            if config.bar.enabled {
                apply_bar_commands(
                    &mut self.dome_channel,
                    &frame.bar,
                    config.bar.infinity_width as usize,
                    config.bar.infinity_length as usize,
                    config.bar.runner_length as usize,
                );
            }
            self.dome
                .send(
                    config.dome.opc_address.clone(),
                    self.dome_channel.current_pixels(),
                )
                .await;
        } else {
            self.dome.disable();
        }

        if config.stage.enabled {
            let side_lengths: Vec<_> = config
                .stage
                .side_lengths
                .iter()
                .map(|length| *length as usize)
                .collect();
            apply_stage_commands(&mut self.stage_channel, &frame.stage, &side_lengths);
            self.stage
                .send(
                    config.stage.opc_address.clone(),
                    self.stage_channel.current_pixels(),
                )
                .await;
        } else {
            self.stage.disable();
        }
    }

    async fn blackout(&mut self, config: &DomersConfig) {
        if config.dome.enabled {
            self.dome_channel.blackout(dome_pixel_count(config));
            self.dome
                .send(
                    config.dome.opc_address.clone(),
                    self.dome_channel.current_pixels(),
                )
                .await;
        }
        if config.stage.enabled {
            self.stage_channel.blackout(stage_pixel_count(config));
            self.stage
                .send(
                    config.stage.opc_address.clone(),
                    self.stage_channel.current_pixels(),
                )
                .await;
        }
    }
}

#[derive(Debug, Default)]
struct HardwareTarget {
    client: Option<OpcClient>,
    status: HardwareTargetStatus,
}

impl HardwareTarget {
    async fn send(&mut self, address: String, pixels: &[Rgb]) {
        self.status.enabled = true;
        if self.status.address.as_deref() != Some(address.as_str()) {
            self.client = None;
            self.status.address = Some(address.clone());
        }

        if self.client.is_none() {
            match OpcAddress::parse(&address) {
                Ok(parsed) => match OpcClient::connect(parsed).await {
                    Ok(client) => {
                        self.client = Some(client);
                        self.status.connected = true;
                        self.status.last_error = None;
                    }
                    Err(error) => {
                        self.status.connected = false;
                        self.status.last_error = Some(error.to_string());
                        return;
                    }
                },
                Err(error) => {
                    self.status.connected = false;
                    self.status.last_error = Some(error);
                    return;
                }
            }
        }

        if let Some(client) = &mut self.client {
            if let Err(error) = client.send_frame(pixels).await {
                self.client = None;
                self.status.connected = false;
                self.status.last_error = Some(error.to_string());
            } else {
                self.status.connected = true;
                self.status.frames_sent = self.status.frames_sent.saturating_add(1);
                self.status.last_error = None;
            }
        }
    }

    fn disable(&mut self) {
        self.client = None;
        self.status.enabled = false;
        self.status.connected = false;
    }
}

fn dome_pixel_count(config: &DomersConfig) -> usize {
    let control_boxes = if config.bar.enabled { 6 } else { 5 };
    control_boxes * DOME_CONTROL_BOX_PIXEL_COUNT
}

fn stage_pixel_count(config: &DomersConfig) -> usize {
    config
        .stage
        .side_lengths
        .chunks(3)
        .map(|triangle| triangle.iter().sum::<u32>() as usize)
        .max()
        .unwrap_or(0)
        * 3
        * 16
}

/// Serve Domers on the provided socket address.
///
/// # Errors
///
/// Returns an error if the TCP listener cannot bind or the HTTP server fails.
pub async fn serve(addr: SocketAddr, config: DomersConfig) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    serve_listener(listener, config).await
}

/// Serve Domers from an existing listener.
///
/// # Errors
///
/// Returns an error if the HTTP server fails.
pub async fn serve_listener(listener: TcpListener, config: DomersConfig) -> std::io::Result<()> {
    axum::serve(listener, AppRuntime::new(config).router()).await
}

#[derive(Clone, Copy, Debug, Deserialize)]
/// Runtime dome config patch payload.
pub struct DomeConfigPatch {
    /// Active dome visualizer index.
    pub active_visualizer: Option<u8>,
    /// Beat flash blackout speed.
    pub flash_speed: Option<f64>,
    /// Active color palette slot.
    pub color_palette_index: Option<u8>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
/// Runtime diagnostic/test-pattern patch payload.
pub struct DiagnosticConfigPatch {
    /// Dome diagnostic pattern.
    pub dome_test_pattern: Option<u8>,
    /// Bar diagnostic pattern: 0 off, 1 flash colors.
    pub bar_test_pattern: Option<u8>,
    /// Stage diagnostic pattern: 0 off, 1 flash colors.
    pub stage_test_pattern: Option<u8>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
/// Runtime color palette patch payload.
pub struct PaletteEntryPatch {
    /// Optional palette bank to patch. Defaults to the active runtime palette.
    pub color_palette_index: Option<u8>,
    /// Color index within the selected palette bank.
    pub relative_index: u8,
    /// First color encoded as `0xRRGGBB`.
    pub color1: u32,
    /// Optional second color encoded as `0xRRGGBB`.
    pub color2: Option<u32>,
    /// Whether to enable gradient blending for this entry.
    pub color2_enabled: Option<bool>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
/// Simulator control patch payload.
pub struct SimulatorControlsPatch {
    /// Normalized audio volume preview.
    pub volume: Option<f32>,
    /// Beat phase preview in `[0.0, 1.0)`.
    pub beat_progress: Option<f64>,
    /// Whether the flash overlay is active.
    pub flash_active: Option<bool>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
/// Dedicated simulator frame payload that never patches live runtime state.
pub struct SimulatorSandboxRequest {
    /// Active dome visualizer index for this simulator page only.
    pub active_visualizer: Option<u8>,
    /// Normalized audio volume preview.
    pub volume: Option<f32>,
    /// Beat phase preview in `[0.0, 1.0)`.
    pub beat_progress: Option<f64>,
    /// Whether the flash overlay is active.
    pub flash_active: Option<bool>,
    /// Primary preview color encoded as `0xRRGGBB`.
    pub primary: Option<u32>,
    /// Secondary preview color encoded as `0xRRGGBB`.
    pub secondary: Option<u32>,
    /// Accent/flash preview color encoded as `0xRRGGBB`.
    pub accent: Option<u32>,
}

impl SimulatorSandboxRequest {
    fn visualizer_input(self) -> VisualizerInput {
        VisualizerInput {
            volume: self.volume.unwrap_or(0.7).clamp(0.0, 1.0),
            beat_progress: self.beat_progress.unwrap_or(0.25).clamp(0.0, 1.0),
            flash_active: self.flash_active.unwrap_or(true),
            primary: domers_core::Rgb::from_u24(self.primary.unwrap_or(0x00_ff_00)),
            secondary: domers_core::Rgb::from_u24(self.secondary.unwrap_or(0x00_80_ff)),
            accent: domers_core::Rgb::from_u24(self.accent.unwrap_or(0xff_40_80)),
            palette: [
                domers_core::Rgb::from_u24(self.primary.unwrap_or(0x00_ff_00)),
                domers_core::Rgb::from_u24(self.secondary.unwrap_or(0x00_80_ff)),
                domers_core::Rgb::from_u24(self.accent.unwrap_or(0xff_40_80)),
                domers_core::Rgb::from_u24(0xff_ff_00),
                domers_core::Rgb::from_u24(0xff_00_ff),
                domers_core::Rgb::from_u24(0x00_ff_ff),
                domers_core::Rgb::from_u24(0xff_ff_ff),
                domers_core::Rgb::BLACK,
            ],
        }
    }
}

async fn index_html() -> Html<&'static str> {
    Html(include_str!("../../../ui/index.html"))
}

async fn simulator_html() -> Html<&'static str> {
    Html(include_str!("../../../ui/simulator.html"))
}

async fn main_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../ui/main.mjs"),
    )
}

async fn health_json() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": health() }))
}

async fn dome_geometry() -> Json<serde_json::Value> {
    Json(
        serde_json::from_str(include_str!(
            "../../../fixtures/spectrum-csharp/dome_geometry.json"
        ))
        .expect("dome geometry fixture is valid JSON"),
    )
}

async fn dome_mapping() -> Json<serde_json::Value> {
    Json(
        serde_json::from_str(include_str!(
            "../../../fixtures/spectrum-csharp/dome_mapping.json"
        ))
        .expect("dome mapping fixture is valid JSON"),
    )
}

async fn get_state(State(runtime): State<AppRuntime>) -> Json<ServerSnapshot> {
    Json(runtime.snapshot().await)
}

async fn start_engine(State(runtime): State<AppRuntime>) -> Json<ServerSnapshot> {
    runtime.start().await;
    Json(runtime.snapshot().await)
}

async fn stop_engine(State(runtime): State<AppRuntime>) -> Json<ServerSnapshot> {
    runtime.stop().await;
    Json(runtime.snapshot().await)
}

async fn get_full_config(State(runtime): State<AppRuntime>) -> Json<DomersConfig> {
    Json(runtime.full_config().await)
}

async fn replace_full_config(
    State(runtime): State<AppRuntime>,
    Json(config): Json<DomersConfig>,
) -> Json<ServerSnapshot> {
    Json(runtime.replace_full_config(config).await)
}

async fn patch_dome_config(
    State(runtime): State<AppRuntime>,
    Json(patch): Json<DomeConfigPatch>,
) -> Json<ServerSnapshot> {
    runtime.patch_dome_config(patch).await;
    Json(runtime.snapshot().await)
}

async fn patch_diagnostics(
    State(runtime): State<AppRuntime>,
    Json(patch): Json<DiagnosticConfigPatch>,
) -> Json<ServerSnapshot> {
    runtime.patch_diagnostics(patch).await;
    Json(runtime.snapshot().await)
}

async fn patch_palette_entry(
    State(runtime): State<AppRuntime>,
    Json(patch): Json<PaletteEntryPatch>,
) -> Json<ServerSnapshot> {
    runtime.patch_palette_entry(patch).await;
    Json(runtime.snapshot().await)
}

async fn tap_tempo(State(runtime): State<AppRuntime>) -> Json<ServerSnapshot> {
    runtime.tap_tempo().await;
    Json(runtime.snapshot().await)
}

async fn calibrate_orientation(State(runtime): State<AppRuntime>) -> Json<ServerSnapshot> {
    runtime.calibrate_orientation_devices().await;
    Json(runtime.snapshot().await)
}

async fn patch_simulator_controls(
    State(runtime): State<AppRuntime>,
    Json(patch): Json<SimulatorControlsPatch>,
) -> Json<ServerSnapshot> {
    runtime.patch_simulator_controls(patch).await;
    Json(runtime.snapshot().await)
}

async fn simulator_preview_frame(State(runtime): State<AppRuntime>) -> Json<SimulatorFrame> {
    Json(runtime.simulator_frame().await)
}

async fn simulator_sandbox_frame(
    State(runtime): State<AppRuntime>,
    Json(request): Json<SimulatorSandboxRequest>,
) -> Json<SimulatorFrame> {
    Json(runtime.simulator_sandbox_frame(request).await)
}

async fn simulator_websocket(
    websocket: WebSocketUpgrade,
    State(runtime): State<AppRuntime>,
) -> Response {
    websocket.on_upgrade(move |socket| simulator_socket(socket, runtime))
}

async fn simulator_socket(mut socket: WebSocket, runtime: AppRuntime) {
    let mut receiver = runtime.subscribe();
    while let Ok(frame) = receiver.recv().await {
        let Ok(payload) = serde_json::to_string(&frame) else {
            continue;
        };
        if socket.send(Message::Text(payload.into())).await.is_err() {
            break;
        }
    }
}

fn serialize_commands(commands: Vec<DomeCommand>) -> Vec<SimulatorCommand> {
    commands
        .into_iter()
        .map(|command| match command {
            DomeCommand::Flush => SimulatorCommand::Flush,
            DomeCommand::Frame(colors) => SimulatorCommand::Frame {
                colors: colors.into_iter().map(domers_core::Rgb::to_u24).collect(),
            },
            DomeCommand::Pixel {
                strut_index,
                led_index,
                color,
            } => SimulatorCommand::Pixel {
                strut_index,
                led_index,
                color: color.to_u24(),
            },
        })
        .collect()
}

fn serialize_bar_commands(commands: Vec<BarCommand>) -> Vec<BarSimulatorCommand> {
    commands
        .into_iter()
        .map(|command| match command {
            BarCommand::Flush => BarSimulatorCommand::Flush,
            BarCommand::Pixel {
                is_runner,
                led_index,
                color,
            } => BarSimulatorCommand::Pixel {
                is_runner,
                led_index,
                color: color.to_u24(),
            },
        })
        .collect()
}

fn serialize_stage_commands(commands: Vec<StageCommand>) -> Vec<StageSimulatorCommand> {
    commands
        .into_iter()
        .map(|command| match command {
            StageCommand::Flush => StageSimulatorCommand::Flush,
            StageCommand::Pixel {
                side_index,
                led_index,
                layer_index,
                color,
            } => StageSimulatorCommand::Pixel {
                side_index,
                led_index,
                layer_index,
                color: color.to_u24(),
            },
        })
        .collect()
}

fn render_operator_frame(
    config: &DomersConfig,
    simulator: SimulatorControls,
    frame_index: u64,
) -> OperatorCommandFrame {
    let engine = EngineConfig::from(config);
    let inputs = input_specs(simulator);
    let outputs = output_specs(config);
    let schedule = schedule_operator_frame(&inputs, &outputs);
    let visualizer_input = simulator.visualizer_input(&engine);
    let diagnostic_input = DiagnosticInput {
        state: diagnostic_state(frame_index),
        step: diagnostic_step(frame_index),
        brightness: brightness_f32(config.dome.brightness),
        volume: simulator.volume,
        beat_progress: simulator.beat_progress,
    };

    let mut frame = OperatorCommandFrame {
        active_inputs: schedule.active_inputs,
        active_visualizers: schedule.active_visualizers.clone(),
        active_outputs: schedule.active_outputs,
        ..OperatorCommandFrame::default()
    };

    for visualizer in &schedule.active_visualizers {
        render_scheduled_visualizer(
            visualizer,
            config,
            diagnostic_input,
            visualizer_input,
            &mut frame,
        );
    }

    frame
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "Diagnostic state intentionally wraps to a three-state animation cycle"
)]
fn diagnostic_state(frame_index: u64) -> u8 {
    ((frame_index / 20) % 3) as u8
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "Diagnostic step intentionally wraps to the displayable fixture range"
)]
fn diagnostic_step(frame_index: u64) -> usize {
    (frame_index / 4) as usize
}

#[allow(
    clippy::too_many_lines,
    reason = "This dispatch table keeps Spectrum visualizer names explicit at the runtime boundary"
)]
fn render_scheduled_visualizer(
    visualizer: &str,
    config: &DomersConfig,
    diagnostic_input: DiagnosticInput,
    visualizer_input: VisualizerInput,
    frame: &mut OperatorCommandFrame,
) {
    match visualizer {
        "LEDDomeVolumeVisualizer" => frame.dome.extend(render_dome_visualizer(
            LiveVisualizer::Volume,
            visualizer_input,
        )),
        "LEDDomeRadialVisualizer" => frame.dome.extend(render_dome_visualizer(
            LiveVisualizer::Radial,
            visualizer_input,
        )),
        "LEDDomeRaceVisualizer" => frame.dome.extend(render_dome_visualizer(
            LiveVisualizer::Race,
            visualizer_input,
        )),
        "LEDDomeSnakesVisualizer" => frame.dome.extend(render_dome_visualizer(
            LiveVisualizer::Snakes,
            visualizer_input,
        )),
        "LEDDomeSplatVisualizer" => frame.dome.extend(render_dome_visualizer(
            LiveVisualizer::Splat,
            visualizer_input,
        )),
        "LEDDomeQuaternionTestVisualizer" => frame.dome.extend(render_dome_visualizer(
            LiveVisualizer::QuaternionTest,
            visualizer_input,
        )),
        "LEDDomeQuaternionMultiTestVisualizer" => frame.dome.extend(render_dome_visualizer(
            LiveVisualizer::QuaternionMultiTest,
            visualizer_input,
        )),
        "LEDDomeQuaternionPaintbrushVisualizer" => frame.dome.extend(render_dome_visualizer(
            LiveVisualizer::QuaternionPaintbrush,
            visualizer_input,
        )),
        "LEDDomeFlashVisualizer" => frame.dome.extend(render_dome_visualizer(
            LiveVisualizer::Flash,
            visualizer_input,
        )),
        "LEDDomeTVStaticVisualizer" => frame.dome.extend(render_dome_visualizer(
            LiveVisualizer::TvStatic,
            visualizer_input,
        )),
        "LEDDomeFlashColorsDiagnosticVisualizer" => frame.dome.extend(render_dome_diagnostic(
            DomeDiagnosticVisualizer::FlashColors,
            diagnostic_input,
        )),
        "LEDDomeStrutIterationDiagnosticVisualizer" => {
            frame.dome.extend(render_dome_diagnostic(
                DomeDiagnosticVisualizer::StrutIteration,
                diagnostic_input,
            ));
        }
        "LEDDomeStrandTestDiagnosticVisualizer" => frame.dome.extend(render_dome_diagnostic(
            DomeDiagnosticVisualizer::StrandTest,
            diagnostic_input,
        )),
        "LEDDomeFullColorFlashDiagnosticVisualizer" => frame.dome.extend(render_dome_diagnostic(
            DomeDiagnosticVisualizer::FullColorFlash,
            diagnostic_input,
        )),
        "LEDBarFlashColorsDiagnosticVisualizer" => frame.bar.extend(render_bar_diagnostic(
            BarDiagnosticVisualizer::FlashColors,
            DiagnosticInput {
                brightness: brightness_f32(config.bar.brightness),
                ..diagnostic_input
            },
            config.bar.infinity_width as usize,
            config.bar.infinity_length as usize,
            config.bar.runner_length as usize,
        )),
        "LEDStageFlashColorsDiagnosticVisualizer" => frame.stage.extend(render_stage_visualizer(
            StageVisualizer::FlashColorsDiagnostic,
            DiagnosticInput {
                brightness: brightness_f32(config.stage.brightness),
                ..diagnostic_input
            },
            &stage_side_lengths(config),
        )),
        "LEDStageDepthLevelVisualizer" => frame.stage.extend(render_stage_visualizer(
            StageVisualizer::DepthLevel,
            DiagnosticInput {
                brightness: brightness_f32(config.stage.brightness),
                ..diagnostic_input
            },
            &stage_side_lengths(config),
        )),
        _ => {}
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "Brightness is user/config bounded and visualizer inputs use f32 channels"
)]
fn brightness_f32(brightness: f64) -> f32 {
    brightness.clamp(0.0, 1.0) as f32
}

fn input_specs(simulator: SimulatorControls) -> [InputSpec; 3] {
    [
        InputSpec {
            name: "audio",
            enabled: true,
            always_active: true,
        },
        InputSpec {
            name: "midi",
            enabled: simulator.flash_active,
            always_active: false,
        },
        InputSpec {
            name: "orientation",
            enabled: true,
            always_active: true,
        },
    ]
}

fn output_specs(config: &DomersConfig) -> [OutputSpec; 3] {
    [
        OutputSpec {
            name: "dome",
            enabled: config.dome.enabled || config.dome.simulation_enabled,
            visualizers: dome_visualizers(config),
        },
        OutputSpec {
            name: "bar",
            enabled: config.bar.enabled || config.bar.simulation_enabled,
            visualizers: bar_visualizers(config),
        },
        OutputSpec {
            name: "stage",
            enabled: config.stage.enabled || config.stage.simulation_enabled,
            visualizers: stage_visualizers(config),
        },
    ]
}

fn dome_visualizers(config: &DomersConfig) -> Vec<FullVisualizerSpec> {
    let mut visualizers = vec![
        FullVisualizerSpec {
            name: active_dome_visualizer_name(config.dome.active_visualizer),
            priority: 2,
            inputs: active_dome_visualizer_inputs(config.dome.active_visualizer),
        },
        FullVisualizerSpec {
            name: "LEDDomeFlashVisualizer",
            priority: 2,
            inputs: &["audio", "midi"],
        },
        FullVisualizerSpec {
            name: "LEDDomeTVStaticVisualizer",
            priority: 1,
            inputs: &[],
        },
    ];
    if let Some(name) = dome_diagnostic_name(config.dome.test_pattern) {
        visualizers.push(FullVisualizerSpec {
            name,
            priority: 1000,
            inputs: &[],
        });
    }
    visualizers
}

fn bar_visualizers(config: &DomersConfig) -> Vec<FullVisualizerSpec> {
    if config.bar.test_pattern == 1 {
        vec![FullVisualizerSpec {
            name: "LEDBarFlashColorsDiagnosticVisualizer",
            priority: 1000,
            inputs: &[],
        }]
    } else {
        Vec::new()
    }
}

fn stage_visualizers(config: &DomersConfig) -> Vec<FullVisualizerSpec> {
    let mut visualizers = vec![FullVisualizerSpec {
        name: "LEDStageDepthLevelVisualizer",
        priority: 3,
        inputs: &["audio"],
    }];
    if config.stage.test_pattern == 1 {
        visualizers.push(FullVisualizerSpec {
            name: "LEDStageFlashColorsDiagnosticVisualizer",
            priority: 1000,
            inputs: &[],
        });
    }
    visualizers
}

fn active_dome_visualizer_name(index: u8) -> &'static str {
    match index {
        1 => "LEDDomeRadialVisualizer",
        2 => "LEDDomeRaceVisualizer",
        3 => "LEDDomeSnakesVisualizer",
        4 => "LEDDomeQuaternionTestVisualizer",
        5 => "LEDDomeQuaternionMultiTestVisualizer",
        6 => "LEDDomeQuaternionPaintbrushVisualizer",
        7 => "LEDDomeSplatVisualizer",
        8 => "LEDDomeTVStaticVisualizer",
        _ => "LEDDomeVolumeVisualizer",
    }
}

fn active_dome_visualizer_inputs(index: u8) -> &'static [&'static str] {
    match index {
        4 | 5 => &["orientation"],
        6 => &["audio", "orientation"],
        _ => &["audio"],
    }
}

fn dome_diagnostic_name(test_pattern: u8) -> Option<&'static str> {
    match test_pattern {
        1 => Some("LEDDomeFlashColorsDiagnosticVisualizer"),
        2 => Some("LEDDomeStrutIterationDiagnosticVisualizer"),
        3 => Some("LEDDomeStrandTestDiagnosticVisualizer"),
        4 => Some("LEDDomeFullColorFlashDiagnosticVisualizer"),
        _ => None,
    }
}

fn stage_side_lengths(config: &DomersConfig) -> Vec<usize> {
    config
        .stage
        .side_lengths
        .iter()
        .map(|length| *length as usize)
        .collect()
}

fn animated_beat_progress(now_ms: u64) -> f64 {
    f64::from((now_ms % 1_000) as u32) / 1_000.0
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "No-input simulator fallback is clamped before converting to visualizer f32 input"
)]
fn animated_volume(now_ms: u64) -> f32 {
    let phase = animated_beat_progress(now_ms) * std::f64::consts::TAU;
    ((phase.sin() + 1.0) * 0.35 + 0.25).clamp(0.0, 1.0) as f32
}

const fn visualizer_from_index(index: u8) -> LiveVisualizer {
    match index {
        1 => LiveVisualizer::Radial,
        2 => LiveVisualizer::Race,
        3 => LiveVisualizer::Snakes,
        4 => LiveVisualizer::QuaternionTest,
        5 => LiveVisualizer::QuaternionMultiTest,
        6 => LiveVisualizer::QuaternionPaintbrush,
        7 => LiveVisualizer::Splat,
        8 => LiveVisualizer::TvStatic,
        _ => LiveVisualizer::Volume,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        io::{ErrorKind, Read, Write},
        net::{SocketAddr, TcpStream, UdpSocket as StdUdpSocket},
        time::Duration,
    };

    use domers_core::{
        DomersConfig, MidiBindingAction, MidiBindingCommandKind, MidiBindingConfig, PaletteEntry,
        TempoSource, UdpInputConfig,
    };
    use domers_inputs::{MidiCommand, MidiCommandKind};
    use domers_outputs::DomeCommand;
    use tokio::time;
    use tokio::{io::AsyncReadExt, net::TcpListener};

    use super::{health, serve_listener, AppRuntime, ServerState, SimulatorCommand};

    fn free_udp_addr() -> SocketAddr {
        let socket = StdUdpSocket::bind("127.0.0.1:0").expect("ephemeral UDP bind");
        socket.local_addr().expect("local UDP addr")
    }

    fn send_udp(addr: SocketAddr, payload: &[u8]) {
        let socket = StdUdpSocket::bind("127.0.0.1:0").expect("UDP sender bind");
        socket.send_to(payload, addr).expect("UDP send succeeds");
    }

    async fn wait_for_input_events(runtime: &AppRuntime) -> super::ServerSnapshot {
        for _ in 0..50 {
            let snapshot = runtime.snapshot().await;
            if snapshot.inputs.audio_adapter.events == 1
                && snapshot.inputs.midi_adapter.events == 1
                && snapshot.inputs.orientation_adapter.events == 1
            {
                return snapshot;
            }
            time::sleep(Duration::from_millis(10)).await;
        }
        runtime.snapshot().await
    }

    #[test]
    fn health_is_ok() {
        assert_eq!(health(), "ok");
    }

    #[test]
    fn patches_config_and_streams_simulator_frame() {
        let mut state = ServerState::default();
        state.start();
        state.patch_dome_config(super::DomeConfigPatch {
            active_visualizer: Some(1),
            flash_speed: Some(8.0),
            color_palette_index: Some(4),
        });

        let frame = state.simulator_frame();

        assert!(state.running());
        assert_eq!(state.config().dome_active_vis, 1);
        assert!((state.config().flash_speed - 8.0).abs() < f64::EPSILON);
        assert_eq!(state.config().color_palette_index, 4);
        assert!(frame
            .dome
            .iter()
            .any(|command| matches!(command, DomeCommand::Frame(_))));
        assert_eq!(state.metrics().frames, 1);
        assert_eq!(state.metrics().simulator_frames, 1);
    }

    #[test]
    fn operator_frame_schedules_flash_overlay_and_all_output_streams() {
        let mut config = DomersConfig::default();
        config.dome.simulation_enabled = true;
        config.bar.simulation_enabled = true;
        config.bar.test_pattern = 1;
        config.stage.simulation_enabled = true;
        config.stage.side_lengths = vec![3, 4, 5];
        let mut state = ServerState::new(config);
        state.patch_simulator_controls(super::SimulatorControlsPatch {
            volume: None,
            beat_progress: None,
            flash_active: Some(true),
        });

        let frame = state.operator_frame();

        assert_eq!(frame.active_outputs, vec!["dome", "bar", "stage"]);
        assert!(frame
            .active_visualizers
            .contains(&"LEDDomeVolumeVisualizer"));
        assert!(frame.active_visualizers.contains(&"LEDDomeFlashVisualizer"));
        assert!(frame
            .active_visualizers
            .contains(&"LEDBarFlashColorsDiagnosticVisualizer"));
        assert!(frame
            .active_visualizers
            .contains(&"LEDStageDepthLevelVisualizer"));
        assert!(!frame.dome.is_empty());
        assert!(!frame.bar.is_empty());
        assert!(!frame.stage.is_empty());
    }

    #[test]
    fn started_runtime_animates_visualizer_inputs_without_live_sources() {
        let mut state = ServerState::default();
        state.patch_dome_config(super::DomeConfigPatch {
            active_visualizer: Some(1),
            flash_speed: None,
            color_palette_index: None,
        });
        state.start();
        state.engine_frame();
        let first = state.operator_frame().dome;
        for _ in 0..100 {
            state.engine_frame();
        }
        let second = state.operator_frame().dome;

        assert_ne!(first, second);
    }

    #[test]
    fn active_visualizer_index_eight_selects_tv_static() {
        let mut config = DomersConfig::default();
        config.dome.active_visualizer = 8;
        let state = ServerState::new(config);

        let frame = state.operator_frame();

        assert_eq!(frame.active_visualizers, vec!["LEDDomeTVStaticVisualizer"]);
    }

    #[test]
    fn operator_frame_diagnostics_override_active_dome_visualizer() {
        let mut config = DomersConfig::default();
        config.dome.test_pattern = 2;
        let state = ServerState::new(config);

        let frame = state.operator_frame();

        assert_eq!(
            frame.active_visualizers,
            vec!["LEDDomeStrutIterationDiagnosticVisualizer"]
        );
        assert!(frame
            .dome
            .iter()
            .any(|command| matches!(command, DomeCommand::Frame(_))));
    }

    #[test]
    fn patches_support_diagnostics_for_all_outputs() {
        let mut config = DomersConfig::default();
        config.bar.simulation_enabled = true;
        config.stage.simulation_enabled = true;
        config.stage.side_lengths = vec![3, 4, 5];
        let mut state = ServerState::new(config);

        state.patch_diagnostics(super::DiagnosticConfigPatch {
            dome_test_pattern: Some(4),
            bar_test_pattern: Some(1),
            stage_test_pattern: Some(1),
        });
        let snapshot = state.snapshot();
        let frame = state.operator_frame();

        assert_eq!(snapshot.diagnostics.dome_test_pattern, 4);
        assert_eq!(snapshot.diagnostics.bar_test_pattern, 1);
        assert_eq!(snapshot.diagnostics.stage_test_pattern, 1);
        assert!(frame
            .active_visualizers
            .contains(&"LEDDomeFullColorFlashDiagnosticVisualizer"));
        assert!(frame
            .active_visualizers
            .contains(&"LEDBarFlashColorsDiagnosticVisualizer"));
        assert!(frame
            .active_visualizers
            .contains(&"LEDStageFlashColorsDiagnosticVisualizer"));
    }

    #[test]
    fn support_diagnostics_animate_with_runtime_frames() {
        let mut config = DomersConfig::default();
        config.dome.test_pattern = 2;
        let mut state = ServerState::new(config);

        let first = state.operator_frame().dome;
        for _ in 0..12 {
            state.engine_frame();
        }
        let second = state.operator_frame().dome;

        assert_ne!(first, second);
    }

    #[test]
    fn patches_simulator_controls() {
        let mut state = ServerState::default();
        state.patch_simulator_controls(super::SimulatorControlsPatch {
            volume: Some(0.25),
            beat_progress: Some(0.75),
            flash_active: Some(false),
        });

        let snapshot = state.snapshot();

        assert!((snapshot.simulator.volume - 0.25).abs() < f32::EPSILON);
        assert!((snapshot.simulator.beat_progress - 0.75).abs() < f64::EPSILON);
        assert!(!snapshot.simulator.flash_active);
    }

    #[test]
    fn runtime_inputs_update_status_and_visualizer_controls() {
        let mut state = ServerState::default();
        state.apply_audio_volume(0.33);
        state.apply_midi_commands(&[
            MidiCommand {
                kind: MidiCommandKind::Note,
                index: 64,
                value: 0.0,
            },
            MidiCommand {
                kind: MidiCommandKind::ControlChange,
                index: 1,
                value: 0.5,
            },
        ]);
        assert!(state.apply_orientation_datagram(&[1; 15]));
        state.tap_tempo();
        for _ in 0..200 {
            state.engine_frame();
        }
        state.tap_tempo();
        for _ in 0..200 {
            state.engine_frame();
        }
        state.tap_tempo();
        assert!(state.report_madmom_line("BEAT:10.000"));
        assert!(state.report_madmom_line("BEAT:10.400"));

        let snapshot = state.snapshot();

        assert_eq!(snapshot.inputs.volume, Some(0.33));
        assert_eq!(snapshot.inputs.beat_ms, Some(400));
        assert_eq!(snapshot.inputs.taps, 3);
        assert_eq!(snapshot.inputs.madmom_beats, 2);
        assert_eq!(snapshot.inputs.midi_commands, 2);
        assert_eq!(snapshot.inputs.last_orientation.as_deref(), Some("WandV1"));
        assert_eq!(snapshot.inputs.orientation_devices.len(), 1);
        assert_eq!(snapshot.inputs.orientation_devices[0].device_id, 1);
        assert!(!snapshot.simulator.flash_active);
        assert!((snapshot.simulator.volume - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn midi_bindings_drive_runtime_actions_and_log() {
        let mut config = DomersConfig::default();
        config.inputs.midi.bindings = vec![
            MidiBindingConfig {
                command_kind: MidiBindingCommandKind::Note,
                index: 60,
                action: MidiBindingAction::TapTempo,
                target_index: None,
            },
            MidiBindingConfig {
                command_kind: MidiBindingCommandKind::ControlChange,
                index: 10,
                action: MidiBindingAction::Palette,
                target_index: Some(6),
            },
            MidiBindingConfig {
                command_kind: MidiBindingCommandKind::Program,
                index: 2,
                action: MidiBindingAction::Visualizer,
                target_index: Some(8),
            },
        ];
        let mut state = ServerState::new(config);

        state.apply_midi_commands(&[
            MidiCommand {
                kind: MidiCommandKind::Note,
                index: 60,
                value: 1.0,
            },
            MidiCommand {
                kind: MidiCommandKind::ControlChange,
                index: 10,
                value: 0.5,
            },
            MidiCommand {
                kind: MidiCommandKind::Program,
                index: 2,
                value: 1.0,
            },
        ]);

        let snapshot = state.snapshot();
        assert_eq!(snapshot.inputs.taps, 1);
        assert_eq!(snapshot.config.color_palette_index, 6);
        assert_eq!(snapshot.config.dome_active_vis, 8);
        assert_eq!(snapshot.inputs.midi_commands, 3);
        assert_eq!(snapshot.inputs.midi_log.len(), 3);
        assert_eq!(snapshot.inputs.midi_log[0].actions, ["tap_tempo"]);
        assert_eq!(snapshot.inputs.midi_log[1].actions, ["palette:6"]);
        assert_eq!(snapshot.inputs.midi_log[2].actions, ["visualizer:8"]);
    }

    #[tokio::test]
    async fn runtime_udp_input_adapters_feed_live_state() {
        let audio_addr = free_udp_addr();
        let midi_addr = free_udp_addr();
        let orientation_addr = free_udp_addr();
        let mut config = DomersConfig::default();
        config.inputs.audio = UdpInputConfig {
            bind: Some(audio_addr.to_string()),
        };
        config.inputs.midi = domers_core::MidiInputConfig {
            bind: Some(midi_addr.to_string()),
            ..domers_core::MidiInputConfig::default()
        };
        config.inputs.orientation = UdpInputConfig {
            bind: Some(orientation_addr.to_string()),
        };
        let runtime = AppRuntime::new(config);

        runtime.start().await;
        time::sleep(Duration::from_millis(25)).await;
        send_udp(audio_addr, b"0.42");
        send_udp(midi_addr, b"cc,1,0.25");
        send_udp(orientation_addr, &[1; 15]);

        let snapshot = wait_for_input_events(&runtime).await;
        runtime.stop().await;

        assert_eq!(snapshot.inputs.volume, Some(0.42));
        assert_eq!(snapshot.inputs.midi_commands, 1);
        assert_eq!(snapshot.inputs.last_orientation.as_deref(), Some("WandV1"));
        assert_eq!(snapshot.inputs.audio_adapter.events, 1);
        assert_eq!(snapshot.inputs.midi_adapter.events, 1);
        assert_eq!(snapshot.inputs.orientation_adapter.events, 1);
    }

    #[tokio::test]
    async fn runtime_madmom_fake_sidecar_feeds_beats() {
        let script = env::temp_dir().join(format!("domers-fake-madmom-{}.py", std::process::id()));
        fs::write(
            &script,
            "import time\nprint('BEAT:1.000', flush=True)\ntime.sleep(0.02)\nprint('BEAT:1.400', flush=True)\ntime.sleep(0.02)\n",
        )
        .expect("fake sidecar script writes");

        let mut config = DomersConfig::default();
        config.tempo.source = TempoSource::Madmom;
        config.madmom.command = "python3".to_string();
        config.madmom.tracker = Some(script.to_string_lossy().to_string());
        let runtime = AppRuntime::new(config);

        runtime.start().await;
        let mut snapshot = runtime.snapshot().await;
        for _ in 0..50 {
            if snapshot.inputs.madmom_adapter.events >= 2 {
                break;
            }
            time::sleep(Duration::from_millis(10)).await;
            snapshot = runtime.snapshot().await;
        }
        runtime.stop().await;
        let _ = fs::remove_file(script);

        assert_eq!(snapshot.inputs.madmom_beats, 2);
        assert_eq!(snapshot.inputs.beat_ms, Some(400));
        assert_eq!(snapshot.inputs.madmom_adapter.last_error, None);
    }

    #[test]
    fn patches_runtime_palette_entry() {
        let mut state = ServerState::default();
        state.patch_dome_config(super::DomeConfigPatch {
            active_visualizer: None,
            flash_speed: None,
            color_palette_index: Some(2),
        });
        state.patch_palette_entry(super::PaletteEntryPatch {
            color_palette_index: None,
            relative_index: 1,
            color1: 0xaa_bb_cc,
            color2: Some(0x11_22_33),
            color2_enabled: Some(true),
        });

        let absolute_index = domers_core::ColorPalette::absolute_index(1, 2);
        assert_eq!(
            state.config().color_palette.colors[absolute_index],
            PaletteEntry::gradient(0xaa_bb_cc, 0x11_22_33)
        );
    }

    #[test]
    fn patches_explicit_runtime_palette_bank() {
        let mut state = ServerState::default();
        state.patch_dome_config(super::DomeConfigPatch {
            active_visualizer: None,
            flash_speed: None,
            color_palette_index: Some(2),
        });
        state.patch_palette_entry(super::PaletteEntryPatch {
            color_palette_index: Some(5),
            relative_index: 2,
            color1: 0x44_55_66,
            color2: None,
            color2_enabled: Some(false),
        });

        let edited_index = domers_core::ColorPalette::absolute_index(2, 5);
        let active_bank_index = domers_core::ColorPalette::absolute_index(2, 2);
        assert_eq!(
            state.config().color_palette.colors[edited_index],
            PaletteEntry::solid(0x44_55_66)
        );
        assert_ne!(
            state.config().color_palette.colors[active_bank_index],
            PaletteEntry::solid(0x44_55_66)
        );
    }

    #[tokio::test]
    async fn runtime_start_streams_frames_and_stop_halts() {
        let runtime = AppRuntime::default();
        let mut frames = runtime.subscribe();

        runtime.start().await;
        let frame = time::timeout(Duration::from_millis(100), frames.recv())
            .await
            .expect("frame should arrive")
            .expect("frame channel should stay open");

        assert!(frame.metrics.frames > 0);
        assert_eq!(frame.metrics.simulator_frames, 1);
        assert!(!frame.commands.is_empty());

        runtime.stop().await;
        assert!(!runtime.snapshot().await.running);
    }

    #[tokio::test]
    async fn runtime_preview_frame_is_visible_without_starting() {
        let runtime = AppRuntime::default();
        let frame = runtime.simulator_frame().await;

        assert!(!runtime.snapshot().await.running);
        assert!(frame.commands.iter().any(|command| {
            matches!(
                command,
                SimulatorCommand::Frame { .. } | SimulatorCommand::Pixel { .. }
            )
        }));
    }

    #[tokio::test]
    async fn sandbox_frame_does_not_patch_runtime_state() {
        let runtime = AppRuntime::default();
        runtime
            .patch_dome_config(super::DomeConfigPatch {
                active_visualizer: Some(0),
                flash_speed: Some(1.0),
                color_palette_index: Some(0),
            })
            .await;
        runtime
            .patch_simulator_controls(super::SimulatorControlsPatch {
                volume: Some(0.25),
                beat_progress: Some(0.25),
                flash_active: Some(false),
            })
            .await;

        let before = runtime.snapshot().await;
        let frame = runtime
            .simulator_sandbox_frame(super::SimulatorSandboxRequest {
                active_visualizer: Some(7),
                volume: Some(1.0),
                beat_progress: Some(0.9),
                flash_active: Some(true),
                primary: Some(0xff_00_00),
                secondary: Some(0x00_ff_00),
                accent: Some(0x00_00_ff),
            })
            .await;
        let after = runtime.snapshot().await;

        assert!(frame.commands.iter().any(|command| {
            matches!(
                command,
                SimulatorCommand::Frame { .. } | SimulatorCommand::Pixel { .. }
            )
        }));
        assert_eq!(before.config, after.config);
        assert_eq!(before.simulator, after.simulator);
        assert_eq!(before.metrics, after.metrics);
    }

    #[tokio::test]
    async fn hardware_outputs_send_mapped_dome_frame_to_loopback_opc() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener binds");
        let addr = listener.local_addr().expect("listener has local addr");
        let read = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accepts");
            let mut header = [0_u8; 4];
            stream.read_exact(&mut header).await.expect("reads header");
            let length = u16::from_be_bytes([header[2], header[3]]) as usize;
            let mut body = vec![0_u8; length];
            stream.read_exact(&mut body).await.expect("reads body");
            (header, body)
        });
        let mut config = DomersConfig::default();
        config.dome.enabled = true;
        config.dome.opc_address = format!("127.0.0.1:{}", addr.port());
        let frame = super::OperatorCommandFrame {
            dome: vec![
                DomeCommand::Pixel {
                    strut_index: 0,
                    led_index: 0,
                    color: domers_core::Rgb::from_u24(0xff_00_00),
                },
                DomeCommand::Flush,
            ],
            ..super::OperatorCommandFrame::default()
        };
        let mut hardware = super::HardwareOutputs::default();

        hardware.send_operator_frame(&config, &frame).await;

        let (header, body) = read.await.expect("read joins");
        let device_index = 880;
        let byte_index = device_index * 3;
        assert_eq!(header[0], 0);
        assert_eq!(&body[byte_index..byte_index + 3], &[0xff, 0, 0]);
        assert!(hardware.status().dome.connected);
        assert_eq!(hardware.status().dome.frames_sent, 1);
    }

    #[tokio::test]
    async fn hardware_outputs_blackout_sends_zero_frame_on_stop() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener binds");
        let addr = listener.local_addr().expect("listener has local addr");
        let read = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accepts");
            let mut header = [0_u8; 4];
            stream.read_exact(&mut header).await.expect("reads header");
            let length = u16::from_be_bytes([header[2], header[3]]) as usize;
            let mut body = vec![0_u8; length];
            stream.read_exact(&mut body).await.expect("reads body");
            (header, body)
        });
        let mut config = DomersConfig::default();
        config.dome.enabled = true;
        config.dome.opc_address = format!("127.0.0.1:{}", addr.port());
        let mut hardware = super::HardwareOutputs::default();

        hardware.blackout(&config).await;

        let (header, body) = read.await.expect("read joins");
        assert_eq!(header[0], 0);
        assert_eq!(body.len(), super::dome_pixel_count(&config) * 3);
        assert!(body.iter().all(|channel| *channel == 0));
        assert!(hardware.status().dome.connected);
    }

    #[tokio::test]
    async fn hardware_outputs_reconnect_after_loopback_opc_returns() {
        let reserved = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener binds");
        let addr = reserved.local_addr().expect("listener has local addr");
        drop(reserved);
        let mut config = DomersConfig::default();
        config.dome.enabled = true;
        config.dome.opc_address = format!("127.0.0.1:{}", addr.port());
        let frame = super::OperatorCommandFrame {
            dome: vec![
                DomeCommand::Pixel {
                    strut_index: 0,
                    led_index: 0,
                    color: domers_core::Rgb::from_u24(0x00_ff_00),
                },
                DomeCommand::Flush,
            ],
            ..super::OperatorCommandFrame::default()
        };
        let mut hardware = super::HardwareOutputs::default();

        hardware.send_operator_frame(&config, &frame).await;
        assert!(!hardware.status().dome.connected);
        assert!(hardware.status().dome.last_error.is_some());

        let listener = TcpListener::bind(addr).await.expect("listener rebinds");
        let read = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accepts");
            let mut header = [0_u8; 4];
            stream.read_exact(&mut header).await.expect("reads header");
            let length = u16::from_be_bytes([header[2], header[3]]) as usize;
            let mut body = vec![0_u8; length];
            stream.read_exact(&mut body).await.expect("reads body");
            (header, body)
        });

        hardware.send_operator_frame(&config, &frame).await;

        let (header, body) = read.await.expect("read joins");
        assert_eq!(header[0], 0);
        assert!(!body.is_empty());
        assert!(hardware.status().dome.connected);
        assert_eq!(hardware.status().dome.frames_sent, 1);
        assert!(hardware.status().dome.last_error.is_none());
    }

    #[tokio::test]
    async fn http_adapter_serves_ui_state_and_start() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener binds");
        let addr = listener.local_addr().expect("listener has local address");
        let server = tokio::spawn(async move {
            serve_listener(listener, DomersConfig::default())
                .await
                .expect("server runs");
        });

        let html = http_request(
            addr,
            "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert!(html.contains("MindShark Dome Control Panel"));

        let simulator = http_request(
            addr,
            "GET /simulator HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert!(simulator.contains("MindShark Dome Simulator"));

        let state = http_request(
            addr,
            "GET /api/state HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert!(state.contains("\"running\":false"));

        let started = http_request(
            addr,
            "POST /api/start HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert!(started.contains("\"running\":true"));

        let tapped = http_request(
            addr,
            "POST /api/input/tap HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert!(tapped.contains("\"taps\":1"));

        let geometry = http_request(
            addr,
            "GET /api/dome/geometry HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert!(geometry.contains("\"line_count\":190"));

        let mapping = http_request(
            addr,
            "GET /api/dome/mapping HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert!(mapping.contains("\"strut_count\":190"));

        server.abort();
    }

    #[test]
    fn stop_updates_running_state_without_dropping_config() {
        let mut state = ServerState::default();
        state.patch_dome_config(super::DomeConfigPatch {
            active_visualizer: Some(7),
            flash_speed: None,
            color_palette_index: None,
        });
        state.start();
        state.stop();

        assert!(!state.running());
        assert_eq!(state.config().dome_active_vis, 7);
    }

    async fn http_request(addr: SocketAddr, request: &'static str) -> String {
        tokio::task::spawn_blocking(move || blocking_http_request(addr, request))
            .await
            .expect("blocking request joins")
    }

    fn blocking_http_request(addr: SocketAddr, request: &str) -> String {
        let mut stream = TcpStream::connect(addr).expect("connect to test server");
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("read timeout is set");
        stream
            .write_all(request.as_bytes())
            .expect("request writes");

        let mut response = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => response.extend_from_slice(&buffer[..read]),
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == ErrorKind::TimedOut => break,
                Err(error) => panic!("response reads: {error}"),
            }
        }

        String::from_utf8(response).expect("response is utf8")
    }
}
