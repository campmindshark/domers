//! Runnable Domers server contract and HTTP/WebSocket adapter.

use std::{
    collections::VecDeque,
    net::SocketAddr,
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};

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
#[cfg(feature = "native-capture")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use domers_core::{
    BeatBroadcaster, ColorPalette, DomersConfig, EngineConfig, LevelDriverPresetConfig,
    MidiBindingAction, MidiBindingCommandKind, MidiBindingConfig, PaletteEntry, Rgb, TempoSource,
};
use domers_engine::{schedule_operator_frame, FullVisualizerSpec, InputSpec, OutputSpec};
use domers_inputs::{
    capture_devices_from_all_endpoints, current_audio_device_index, parse_beat_line,
    parse_link_tempo_line, parse_midi_payload, parse_volume_payload, AudioDeviceFlow,
    EnumeratedAudioEndpoint, MadmomLaunchConfig, MidiCommand, MidiCommandKind, OrientationDevice,
    OrientationInputState, OrientationQuaternion,
};
use domers_outputs::{
    apply_bar_commands, apply_dome_commands, apply_stage_commands, BarCommand, DomeCommand,
    OpcAddress, OpcClient, PersistentChannel, StageCommand,
};
use domers_visualizers::{
    render_bar_diagnostic, render_dome_diagnostic, render_dome_visualizer, render_stage_visualizer,
    render_stage_visualizer_with_input, BarDiagnosticVisualizer, DiagnosticInput,
    DomeDiagnosticVisualizer, LiveVisualizer, OrientationOverride, StageVisualizer,
    StageVisualizerInput, VisualizerInput,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "native-capture")]
use tokio::sync::mpsc;
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

/// Emit a browser simulator frame roughly every 17.5 ms while the engine runs.
pub const SIMULATOR_FRAME_STRIDE: u64 = 7;

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
    /// Spectrum-style BPM display.
    pub bpm: String,
    /// Current beat progress in `[0.0, 1.0)`.
    pub beat_progress: f64,
    /// Number of tap-tempo taps accepted.
    pub taps: u64,
    /// Spectrum-style tap counter text.
    pub tap_counter_text: String,
    /// Whether the tap counter should be highlighted as active.
    pub tap_counter_active: bool,
    /// Number of parsed Madmom beat lines accepted.
    pub madmom_beats: u64,
    /// Number of MIDI commands applied.
    pub midi_commands: u64,
    /// Recent MIDI command/action log.
    pub midi_log: Vec<MidiLogEntry>,
    /// Current Spectrum MIDI ADSR level-driver channel values.
    pub midi_level_driver_channels: Vec<Option<f64>>,
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
    /// Ableton Link / Carabiner-compatible sidecar status.
    pub link_adapter: InputAdapterStatus,
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
    /// Whether simulator orientation angles override visualizer orientation.
    pub orientation_override_enabled: bool,
    /// Simulator yaw angle in degrees.
    pub orientation_yaw: f64,
    /// Simulator pitch angle in degrees.
    pub orientation_pitch: f64,
    /// Simulator roll angle in degrees.
    pub orientation_roll: f64,
}

impl Default for SimulatorControls {
    fn default() -> Self {
        Self {
            volume: 0.7,
            beat_progress: 0.25,
            flash_active: false,
            orientation_override_enabled: false,
            orientation_yaw: 0.0,
            orientation_pitch: -90.0,
            orientation_roll: 0.0,
        }
    }
}

impl SimulatorControls {
    fn visualizer_input(self, config: &EngineConfig, animation_frame: u64) -> VisualizerInput {
        let palette = std::array::from_fn(|index| {
            config
                .color_palette
                .single_color(index, config.color_palette_index)
        });
        let palette_entries = std::array::from_fn(|index| {
            config
                .color_palette
                .entry(domers_core::ColorPalette::absolute_index(
                    index,
                    config.color_palette_index,
                ))
        });
        VisualizerInput {
            volume: self.volume,
            beat_progress: self.beat_progress,
            animation_frame,
            orientation_override: self.orientation_override(),
            flash_active: self.flash_active,
            primary: palette[0],
            secondary: palette[1],
            accent: palette[2],
            palette,
            palette_entries,
        }
    }

    fn orientation_override(self) -> Option<OrientationOverride> {
        self.orientation_override_enabled.then(|| {
            orientation_override_from_degrees(
                self.orientation_yaw,
                self.orientation_pitch,
                self.orientation_roll,
            )
        })
    }
}

/// In-process server state shared by HTTP handlers and the engine task.
#[derive(Clone, Debug)]
pub struct ServerState {
    config: DomersConfig,
    simulator: SimulatorControls,
    inputs: InputRuntime,
    metrics: Metrics,
    running: bool,
    input_epoch: Instant,
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new(DomersConfig::default())
    }
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
                orientation_override_enabled: false,
                orientation_yaw: 0.0,
                orientation_pitch: -90.0,
                orientation_roll: 0.0,
            },
            inputs: InputRuntime::default(),
            metrics: Metrics {
                frames: 0,
                simulator_frames: 0,
            },
            running: false,
            input_epoch: Instant::now(),
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
        if let Some(enabled) = patch.orientation_override_enabled {
            self.simulator.orientation_override_enabled = enabled;
        }
        if let Some(yaw) = patch.orientation_yaw {
            self.simulator.orientation_yaw = clamp_orientation_degrees(yaw);
        }
        if let Some(pitch) = patch.orientation_pitch {
            self.simulator.orientation_pitch = clamp_orientation_degrees(pitch);
        }
        if let Some(roll) = patch.orientation_roll {
            self.simulator.orientation_roll = clamp_orientation_degrees(roll);
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
        self.tap_tempo_at(self.now_ms());
    }

    /// Record a human tap-tempo event at a known timestamp.
    ///
    /// This keeps runtime taps on wall-clock time while preserving deterministic
    /// tests and fixture inputs that need explicit timestamps.
    pub fn tap_tempo_at(&mut self, timestamp_ms: u64) {
        self.inputs.taps = self.inputs.taps.saturating_add(1);
        self.inputs.beat.add_tap(timestamp_ms);
    }

    /// Reset tempo state.
    pub fn reset_tempo(&mut self) {
        self.inputs.beat.reset();
        self.inputs.taps = 0;
        self.inputs.madmom_beats = 0;
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

    /// Parse and record an Ableton Link / Carabiner sidecar tempo line.
    pub fn report_link_line(&mut self, line: &str) -> bool {
        if let Some(event) = parse_link_tempo_line(line) {
            self.inputs.link_adapter.events = self.inputs.link_adapter.events.saturating_add(1);
            self.inputs.link_adapter.last_error = None;
            self.inputs
                .beat
                .report_link_tempo(event.bpm, event.phase, self.now_ms());
            true
        } else {
            self.inputs.link_adapter.last_error = Some("malformed Link tempo line".to_string());
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
            MidiBindingAction::AdsrLevelDriver => self.apply_adsr_level_driver_binding(
                binding.index,
                command.index,
                f64::from(command.value),
            ),
        }
    }

    fn apply_adsr_level_driver_binding(
        &mut self,
        range_start: u8,
        note_index: u8,
        velocity: f64,
    ) -> String {
        let channel_index = note_index.saturating_sub(range_start);
        if channel_index >= MIDI_LEVEL_DRIVER_CHANNELS_U8 {
            return "adsr:out_of_range".to_string();
        }
        let channel_index = usize::from(channel_index);
        if self.midi_level_driver_preset(channel_index).is_none() {
            return format!("adsr:{channel_index}:unassigned");
        }
        let now_ms = self.now_ms();
        if velocity == 0.0 {
            if let Some(driver) = &mut self.inputs.midi_level_drivers[channel_index] {
                driver.release_timestamp_ms = Some(now_ms);
                self.inputs.last_level_driver_interaction_ms = now_ms;
            }
            format!("adsr:{channel_index}:release")
        } else {
            self.inputs.midi_level_drivers[channel_index] = Some(MidiLevelDriverInstance {
                press_timestamp_ms: now_ms,
                press_velocity: velocity.clamp(0.0, 1.0),
                release_timestamp_ms: None,
            });
            self.inputs.last_level_driver_interaction_ms = now_ms;
            format!("adsr:{channel_index}:press")
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
        self.inputs.audio_adapter.enabled =
            config.inputs.audio.bind.is_some() || config.inputs.audio.native_enabled;
        self.inputs.audio_adapter.target = if config.inputs.audio.native_enabled {
            Some(config.inputs.audio.device_id.clone().map_or_else(
                || "native audio".to_string(),
                |device| format!("native:{device}"),
            ))
        } else {
            config.inputs.audio.bind.clone()
        };
        self.inputs.midi_adapter.enabled =
            config.inputs.midi.bind.is_some() || config.inputs.midi.native_enabled;
        self.inputs.midi_adapter.target = if config.inputs.midi.native_enabled {
            Some(config.inputs.midi.device_id.clone().map_or_else(
                || "native MIDI".to_string(),
                |device| format!("native:{device}"),
            ))
        } else {
            config.inputs.midi.bind.clone()
        };
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
        self.inputs.link_adapter.enabled = matches!(config.tempo.source, TempoSource::Link);
        self.inputs.link_adapter.target = if self.inputs.link_adapter.enabled {
            Some(config.carabiner.command.clone())
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
            InputAdapter::Link => &mut self.inputs.link_adapter,
        };
        status.enabled = true;
        status.last_error = Some(error.into());
    }

    /// Produce one deterministic simulator frame for the selected visualizer.
    pub fn simulator_frame(&mut self) -> OperatorCommandFrame {
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

    #[allow(
        clippy::cast_possible_truncation,
        reason = "Runtime uptime only needs millisecond precision for a single server process"
    )]
    fn now_ms(&self) -> u64 {
        self.input_epoch
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    }

    #[cfg(test)]
    fn set_now_ms_for_test(&mut self, now_ms: u64) {
        self.input_epoch = Instant::now()
            .checked_sub(Duration::from_millis(now_ms))
            .expect("test time offset fits in Instant range");
    }

    fn prune_input_state(&mut self) {
        self.inputs.orientation.remove_stale_devices(self.now_ms());
    }

    fn visualizer_controls(&self) -> SimulatorControls {
        let mut controls = self.simulator;
        if let Some(volume) = self.active_midi_level_driver_volume() {
            controls.volume = volume;
        } else if let Some(volume) = self.inputs.volume {
            controls.volume = volume;
        } else if self.running {
            controls.volume = animated_volume(self.now_ms());
        }
        if self.inputs.beat.beat_ms().is_some() {
            controls.beat_progress = self
                .inputs
                .beat
                .progress(self.now_ms(), MEASURE_PROGRESS_FACTOR);
        } else if self.running {
            controls.beat_progress = animated_beat_progress(self.now_ms());
        }
        controls
    }

    fn input_status(&self) -> InputStatus {
        InputStatus {
            volume: self.inputs.volume,
            beat_ms: self.inputs.beat.beat_ms(),
            bpm: self.inputs.beat.bpm_string(),
            beat_progress: self.inputs.beat.progress(self.now_ms(), 1.0),
            taps: self.inputs.taps,
            tap_counter_text: self.inputs.beat.tap_counter_text(self.now_ms()),
            tap_counter_active: self.inputs.beat.tap_counter_active(self.now_ms()),
            madmom_beats: self.inputs.madmom_beats,
            midi_commands: self.inputs.midi_commands,
            midi_log: self.inputs.midi_log.iter().cloned().collect(),
            midi_level_driver_channels: self.current_midi_level_driver_values(),
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
            link_adapter: self.inputs.link_adapter.clone(),
        }
    }

    fn current_midi_level_driver_values(&self) -> Vec<Option<f64>> {
        (0..MIDI_LEVEL_DRIVER_CHANNELS)
            .map(|channel_index| self.current_midi_level_driver_value(channel_index))
            .collect()
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "ADSR levels are clamped to the visualizer f32 input range"
    )]
    fn active_midi_level_driver_volume(&self) -> Option<f32> {
        self.current_midi_level_driver_values()
            .into_iter()
            .flatten()
            .reduce(f64::max)
            .map(|value| value.clamp(0.0, 1.0) as f32)
    }

    fn midi_level_driver_preset(&self, channel_index: usize) -> Option<&LevelDriverPresetConfig> {
        let channel = u8::try_from(channel_index).ok()?;
        let preset_name = self.config.level_drivers.midi_channels.get(&channel)?;
        let preset = self.config.level_drivers.presets.get(preset_name)?;
        preset.is_midi().then_some(preset)
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "ADSR envelope durations are millisecond musical values converted to interpolation ratios"
    )]
    fn current_midi_level_driver_value(&self, channel_index: usize) -> Option<f64> {
        let preset = self.midi_level_driver_preset(channel_index)?;
        let LevelDriverPresetConfig::Midi {
            attack_time,
            peak_level,
            decay_time,
            sustain_level,
            release_time,
        } = preset
        else {
            return None;
        };
        let driver = self.inputs.midi_level_drivers[channel_index]?;
        let now = self.now_ms();
        let current_value = midi_level_without_release(
            driver,
            *attack_time,
            *peak_level,
            *decay_time,
            *sustain_level,
            now,
        );
        let Some(release_timestamp_ms) = driver.release_timestamp_ms else {
            return Some(current_value);
        };
        if now < release_timestamp_ms {
            return Some(current_value);
        }
        let time_since_release = now.saturating_sub(release_timestamp_ms);
        if time_since_release > *release_time {
            if now
                > self
                    .inputs
                    .last_level_driver_interaction_ms
                    .saturating_add(5_000)
            {
                return None;
            }
            return Some(0.0);
        }
        if *release_time == 0 {
            return Some(0.0);
        }
        let level_at_release = midi_level_without_release(
            driver,
            *attack_time,
            *peak_level,
            *decay_time,
            *sustain_level,
            release_timestamp_ms,
        );
        Some(
            level_at_release * ((*release_time - time_since_release) as f64) / *release_time as f64,
        )
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
    midi_level_drivers: [Option<MidiLevelDriverInstance>; MIDI_LEVEL_DRIVER_CHANNELS],
    last_level_driver_interaction_ms: u64,
    last_orientation: Option<String>,
    orientation: OrientationInputState,
    audio_adapter: InputAdapterStatus,
    midi_adapter: InputAdapterStatus,
    orientation_adapter: InputAdapterStatus,
    madmom_adapter: InputAdapterStatus,
    link_adapter: InputAdapterStatus,
}

const MIDI_LEVEL_DRIVER_CHANNELS: usize = 8;
const MIDI_LEVEL_DRIVER_CHANNELS_U8: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
struct MidiLevelDriverInstance {
    press_timestamp_ms: u64,
    press_velocity: f64,
    release_timestamp_ms: Option<u64>,
}

#[derive(Clone, Copy)]
enum InputAdapter {
    Audio,
    Midi,
    Orientation,
    Madmom,
    Link,
}

fn midi_binding_matches(binding: &MidiBindingConfig, command: MidiCommand) -> bool {
    let index_matches = if binding.action == MidiBindingAction::AdsrLevelDriver {
        matches!(binding.command_kind, MidiBindingCommandKind::Note)
            && command.index >= binding.index
            && command.index < binding.index.saturating_add(MIDI_LEVEL_DRIVER_CHANNELS_U8)
    } else {
        binding.index == command.index
    };
    binding
        .device_index
        .map_or(true, |device_index| device_index == command.device_index)
        && index_matches
        && midi_kind_matches(binding.command_kind, command.kind)
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
    clippy::cast_precision_loss,
    reason = "ADSR envelope durations are millisecond musical values converted to interpolation ratios"
)]
fn midi_level_without_release(
    driver: MidiLevelDriverInstance,
    attack_time: u64,
    peak_level: f64,
    decay_time: u64,
    sustain_level: f64,
    current_time_ms: u64,
) -> f64 {
    let time_since_press = current_time_ms.saturating_sub(driver.press_timestamp_ms);
    let real_peak = driver.press_velocity * peak_level;
    if attack_time > 0 && time_since_press < attack_time {
        return (time_since_press as f64 / attack_time as f64) * real_peak;
    }
    let time_since_decay_began = time_since_press.saturating_sub(attack_time);
    let real_sustain = driver.press_velocity * sustain_level;
    if decay_time == 0 || time_since_decay_began > decay_time {
        return real_sustain;
    }
    real_peak - time_since_decay_began as f64 / decay_time as f64 * (real_peak - real_sustain)
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
            .route("/assets/main.js", get(main_js))
            .route("/assets/styles.css", get(styles_css))
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
            .route("/api/input/tempo/reset", post(reset_tempo))
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
        if config.inputs.audio.native_enabled {
            tasks.push(spawn_native_audio_task(self.state.clone(), config.clone()));
        }
        if let Some(bind) = &config.inputs.midi.bind {
            tasks.push(spawn_midi_udp_task(self.state.clone(), bind.clone()));
        }
        if config.inputs.midi.native_enabled {
            tasks.push(spawn_native_midi_task(self.state.clone(), config.clone()));
        }
        if let Some(bind) = &config.inputs.orientation.bind {
            tasks.push(spawn_orientation_udp_task(self.state.clone(), bind.clone()));
        }
        if matches!(config.tempo.source, TempoSource::Madmom) {
            tasks.push(spawn_madmom_task(self.state.clone(), config.clone()));
        }
        if matches!(config.tempo.source, TempoSource::Link) {
            tasks.push(spawn_link_task(self.state.clone(), config.clone()));
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

    /// Reset tempo input state.
    pub async fn reset_tempo(&self) -> ServerSnapshot {
        let mut state = self.state.lock().await;
        state.reset_tempo();
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

#[cfg(feature = "native-capture")]
fn spawn_native_audio_task(state: Arc<Mutex<ServerState>>, config: DomersConfig) -> JoinHandle<()> {
    tokio::spawn(async move {
        let host = cpal::default_host();
        let device = match native_audio_device(&host, config.inputs.audio.device_id.as_deref()) {
            Ok(device) => device,
            Err(error) => {
                state
                    .lock()
                    .await
                    .record_input_adapter_error(InputAdapter::Audio, error);
                return;
            }
        };
        let stream_config = match device.default_input_config() {
            Ok(config) => config,
            Err(error) => {
                state
                    .lock()
                    .await
                    .record_input_adapter_error(InputAdapter::Audio, error.to_string());
                return;
            }
        };
        let (tx, mut rx) = mpsc::unbounded_channel();
        let err_state = state.clone();
        let stream = match build_native_audio_stream(&device, &stream_config, tx) {
            Ok(stream) => stream,
            Err(error) => {
                state
                    .lock()
                    .await
                    .record_input_adapter_error(InputAdapter::Audio, error);
                return;
            }
        };
        if let Err(error) = stream.play() {
            state
                .lock()
                .await
                .record_input_adapter_error(InputAdapter::Audio, error.to_string());
            return;
        }
        while let Some(volume) = rx.recv().await {
            state.lock().await.apply_audio_volume(volume);
        }
        drop(stream);
        err_state
            .lock()
            .await
            .record_input_adapter_error(InputAdapter::Audio, "native audio stream closed");
    })
}

#[cfg(not(feature = "native-capture"))]
fn spawn_native_audio_task(
    state: Arc<Mutex<ServerState>>,
    _config: DomersConfig,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        state.lock().await.record_input_adapter_error(
            InputAdapter::Audio,
            "native audio capture requires the native-capture build feature",
        );
    })
}

#[cfg(feature = "native-capture")]
fn spawn_native_midi_task(state: Arc<Mutex<ServerState>>, config: DomersConfig) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut input = match midir::MidiInput::new("domers-native-midi") {
            Ok(input) => input,
            Err(error) => {
                state
                    .lock()
                    .await
                    .record_input_adapter_error(InputAdapter::Midi, error.to_string());
                return;
            }
        };
        input.ignore(midir::Ignore::None);
        let ports = input.ports();
        let Some(port) = native_midi_port(&input, &ports, config.inputs.midi.device_id.as_deref())
        else {
            state
                .lock()
                .await
                .record_input_adapter_error(InputAdapter::Midi, "native MIDI port not found");
            return;
        };
        let (tx, mut rx) = mpsc::unbounded_channel();
        let connection = match input.connect(
            &port,
            "domers-native-midi-input",
            move |_timestamp, message, _| {
                if let Some(command) = midi_message_to_command(0, message) {
                    let _ = tx.send(command);
                }
            },
            (),
        ) {
            Ok(connection) => connection,
            Err(error) => {
                state
                    .lock()
                    .await
                    .record_input_adapter_error(InputAdapter::Midi, error.to_string());
                return;
            }
        };
        while let Some(command) = rx.recv().await {
            state.lock().await.apply_midi_commands(&[command]);
        }
        drop(connection);
    })
}

#[cfg(not(feature = "native-capture"))]
fn spawn_native_midi_task(state: Arc<Mutex<ServerState>>, _config: DomersConfig) -> JoinHandle<()> {
    tokio::spawn(async move {
        state.lock().await.record_input_adapter_error(
            InputAdapter::Midi,
            "native MIDI capture requires the native-capture build feature",
        );
    })
}

#[cfg(feature = "native-capture")]
fn native_audio_device(
    host: &cpal::Host,
    configured_device: Option<&str>,
) -> Result<cpal::Device, String> {
    if let Some(configured_device) = configured_device {
        let devices = host.input_devices().map_err(|error| error.to_string())?;
        for device in devices {
            if device
                .name()
                .map(|name| name == configured_device)
                .unwrap_or(false)
            {
                return Ok(device);
            }
        }
        Err(format!(
            "native audio device not found: {configured_device}"
        ))
    } else {
        host.default_input_device()
            .ok_or_else(|| "native default audio input not found".to_string())
    }
}

#[cfg(feature = "native-capture")]
fn build_native_audio_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    tx: mpsc::UnboundedSender<f32>,
) -> Result<cpal::Stream, String> {
    let stream_config = config.config();
    let channels = usize::from(stream_config.channels.max(1));
    let err_fn = |error| eprintln!("native audio stream error: {error}");
    match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _| {
                    let _ = tx.send(rms_volume_f32(data, channels));
                },
                err_fn,
                None,
            )
            .map_err(|error| error.to_string()),
        cpal::SampleFormat::I16 => device
            .build_input_stream(
                &stream_config,
                move |data: &[i16], _| {
                    let _ = tx.send(rms_volume_i16(data, channels));
                },
                err_fn,
                None,
            )
            .map_err(|error| error.to_string()),
        cpal::SampleFormat::U16 => device
            .build_input_stream(
                &stream_config,
                move |data: &[u16], _| {
                    let _ = tx.send(rms_volume_u16(data, channels));
                },
                err_fn,
                None,
            )
            .map_err(|error| error.to_string()),
        sample_format => Err(format!(
            "unsupported native audio sample format: {sample_format:?}"
        )),
    }
}

#[cfg(feature = "native-capture")]
fn native_midi_port(
    input: &midir::MidiInput,
    ports: &[midir::MidiInputPort],
    configured_device: Option<&str>,
) -> Option<midir::MidiInputPort> {
    if let Some(configured_device) = configured_device {
        ports.iter().find_map(|port| {
            input
                .port_name(port)
                .ok()
                .filter(|name| name == configured_device)
                .map(|_| port.clone())
        })
    } else {
        ports.first().cloned()
    }
}

#[cfg(feature = "native-capture")]
fn midi_message_to_command(device_index: u8, message: &[u8]) -> Option<MidiCommand> {
    let status = *message.first()?;
    let kind = status & 0xf0;
    match kind {
        0x80 => Some(MidiCommand {
            device_index,
            kind: MidiCommandKind::Note,
            index: *message.get(1)?,
            value: 0.0,
        }),
        0x90 => {
            let velocity = f32::from(*message.get(2)?) / 127.0;
            Some(MidiCommand {
                device_index,
                kind: MidiCommandKind::Note,
                index: *message.get(1)?,
                value: velocity.clamp(0.0, 1.0),
            })
        }
        0xb0 => {
            let value = f32::from(*message.get(2)?) / 127.0;
            Some(MidiCommand {
                device_index,
                kind: MidiCommandKind::ControlChange,
                index: *message.get(1)?,
                value: value.clamp(0.0, 1.0),
            })
        }
        0xc0 => Some(MidiCommand {
            device_index,
            kind: MidiCommandKind::Program,
            index: *message.get(1)?,
            value: 1.0,
        }),
        _ => None,
    }
}

#[cfg(feature = "native-capture")]
fn rms_volume_f32(samples: &[f32], channels: usize) -> f32 {
    rms_volume(
        samples
            .iter()
            .step_by(channels)
            .map(|sample| f64::from(*sample)),
    )
}

#[cfg(feature = "native-capture")]
fn rms_volume_i16(samples: &[i16], channels: usize) -> f32 {
    rms_volume(
        samples
            .iter()
            .step_by(channels)
            .map(|sample| f64::from(*sample) / f64::from(i16::MAX)),
    )
}

#[cfg(feature = "native-capture")]
fn rms_volume_u16(samples: &[u16], channels: usize) -> f32 {
    rms_volume(
        samples
            .iter()
            .step_by(channels)
            .map(|sample| (f64::from(*sample) - 32768.0) / 32768.0),
    )
}

#[cfg(feature = "native-capture")]
#[allow(
    clippy::cast_possible_truncation,
    reason = "RMS value is clamped to the normalized f32 visualizer input range"
)]
fn rms_volume(samples: impl Iterator<Item = f64>) -> f32 {
    let mut count = 0_u64;
    let mut sum = 0.0;
    for sample in samples {
        count = count.saturating_add(1);
        sum += sample * sample;
    }
    if count == 0 {
        return 0.0;
    }
    (sum / count as f64).sqrt().clamp(0.0, 1.0) as f32
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

fn spawn_madmom_task(state: Arc<Mutex<ServerState>>, config: DomersConfig) -> JoinHandle<()> {
    tokio::spawn(async move {
        let audio_input_index = madmom_audio_input_index(&config);
        let launch = MadmomLaunchConfig {
            command: config.madmom.command,
            tracker: config.madmom.tracker,
            audio_input_index,
        }
        .resolve();
        let mut command = Command::new(&launch.command);
        if let Some(working_directory) = launch.working_directory() {
            command.current_dir(working_directory);
        }
        if let Some(python_path) = launch.python_path() {
            command.env("PYTHONPATH", python_path);
        }
        let mut child = match command
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
        match child.wait().await {
            Ok(status) if status.success() => {}
            Ok(status) => {
                state.lock().await.record_input_adapter_error(
                    InputAdapter::Madmom,
                    format!("Madmom exited with status {status}"),
                );
            }
            Err(error) => {
                state
                    .lock()
                    .await
                    .record_input_adapter_error(InputAdapter::Madmom, error.to_string());
            }
        }
    })
}

fn spawn_link_task(state: Arc<Mutex<ServerState>>, config: DomersConfig) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut child = match Command::new(&config.carabiner.command)
            .args(config.carabiner.args)
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
                    .record_input_adapter_error(InputAdapter::Link, error.to_string());
                return;
            }
        };
        let Some(stdout) = child.stdout.take() else {
            state
                .lock()
                .await
                .record_input_adapter_error(InputAdapter::Link, "Link sidecar stdout unavailable");
            let _ = child.kill().await;
            return;
        };
        let mut lines = BufReader::new(stdout).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    state.lock().await.report_link_line(&line);
                }
                Ok(None) => break,
                Err(error) => {
                    state
                        .lock()
                        .await
                        .record_input_adapter_error(InputAdapter::Link, error.to_string());
                    break;
                }
            }
        }
        if let Err(error) = child.wait().await {
            state
                .lock()
                .await
                .record_input_adapter_error(InputAdapter::Link, error.to_string());
        }
    })
}

fn madmom_audio_input_index(config: &DomersConfig) -> Option<u32> {
    if config.madmom.audio_input_index.is_some() {
        return config.madmom.audio_input_index;
    }
    let endpoints: Vec<_> = config
        .inputs
        .audio
        .devices
        .iter()
        .map(|device| EnumeratedAudioEndpoint {
            id: device.id.clone(),
            name: device.name.clone(),
            flow: match device.flow {
                domers_core::AudioDeviceFlowConfig::Capture => AudioDeviceFlow::Capture,
                domers_core::AudioDeviceFlowConfig::Render => AudioDeviceFlow::Render,
            },
        })
        .collect();
    let devices = capture_devices_from_all_endpoints(&endpoints);
    let index = current_audio_device_index(config.inputs.audio.device_id.as_deref(), &devices);
    u32::try_from(index).ok()
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

#[derive(Clone, Copy, Debug, Default, Deserialize)]
/// Simulator control patch payload.
pub struct SimulatorControlsPatch {
    /// Normalized audio volume preview.
    pub volume: Option<f32>,
    /// Beat phase preview in `[0.0, 1.0)`.
    pub beat_progress: Option<f64>,
    /// Whether the flash overlay is active.
    pub flash_active: Option<bool>,
    /// Whether simulator orientation angles override visualizer orientation.
    pub orientation_override_enabled: Option<bool>,
    /// Simulator yaw angle in degrees.
    pub orientation_yaw: Option<f64>,
    /// Simulator pitch angle in degrees.
    pub orientation_pitch: Option<f64>,
    /// Simulator roll angle in degrees.
    pub orientation_roll: Option<f64>,
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
    /// Whether simulator orientation angles override visualizer orientation.
    pub orientation_override_enabled: Option<bool>,
    /// Simulator yaw angle in degrees.
    pub orientation_yaw: Option<f64>,
    /// Simulator pitch angle in degrees.
    pub orientation_pitch: Option<f64>,
    /// Simulator roll angle in degrees.
    pub orientation_roll: Option<f64>,
    /// Primary preview color encoded as `0xRRGGBB`.
    pub primary: Option<u32>,
    /// Secondary preview color encoded as `0xRRGGBB`.
    pub secondary: Option<u32>,
    /// Accent/flash preview color encoded as `0xRRGGBB`.
    pub accent: Option<u32>,
}

impl SimulatorSandboxRequest {
    fn visualizer_input(self) -> VisualizerInput {
        let palette_entries = [
            domers_core::PaletteEntry::solid(self.primary.unwrap_or(0x00_ff_00)),
            domers_core::PaletteEntry::solid(self.secondary.unwrap_or(0x00_80_ff)),
            domers_core::PaletteEntry::solid(self.accent.unwrap_or(0xff_40_80)),
            domers_core::PaletteEntry::solid(0xff_ff_00),
            domers_core::PaletteEntry::solid(0xff_00_ff),
            domers_core::PaletteEntry::solid(0x00_ff_ff),
            domers_core::PaletteEntry::solid(0xff_ff_ff),
            domers_core::PaletteEntry::solid(0),
        ];
        VisualizerInput {
            volume: self.volume.unwrap_or(0.7).clamp(0.0, 1.0),
            beat_progress: self.beat_progress.unwrap_or(0.25).clamp(0.0, 1.0),
            animation_frame: 0,
            orientation_override: self.orientation_override_enabled.unwrap_or(false).then(|| {
                orientation_override_from_degrees(
                    self.orientation_yaw.unwrap_or(0.0),
                    self.orientation_pitch.unwrap_or(-90.0),
                    self.orientation_roll.unwrap_or(0.0),
                )
            }),
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
            palette_entries,
        }
    }
}

async fn index_html() -> Html<&'static str> {
    Html(include_str!("../../../ui/dist/index.html"))
}

async fn simulator_html() -> Html<&'static str> {
    Html(include_str!("../../../ui/dist/simulator.html"))
}

async fn main_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../ui/dist/assets/main.js"),
    )
}

async fn styles_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../../../ui/dist/assets/styles.css"),
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

async fn reset_tempo(State(runtime): State<AppRuntime>) -> Json<ServerSnapshot> {
    runtime.reset_tempo().await;
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
    let visualizer_input = simulator.visualizer_input(&engine, frame_index);
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
        "LEDStageDepthLevelVisualizer" => frame.stage.extend(render_stage_visualizer_with_input(
            StageVisualizer::DepthLevel,
            StageVisualizerInput {
                diagnostic: DiagnosticInput {
                    brightness: brightness_f32(config.stage.brightness),
                    ..diagnostic_input
                },
                color_palette: config.color_palette.clone(),
                color_palette_index: config.color_palette_index,
                stage_brightness: config.stage.brightness,
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

fn orientation_override_from_degrees(yaw: f64, pitch: f64, roll: f64) -> OrientationOverride {
    OrientationOverride {
        yaw: clamp_orientation_degrees(yaw).to_radians(),
        pitch: clamp_orientation_degrees(pitch).to_radians(),
        roll: clamp_orientation_degrees(roll).to_radians(),
    }
}

fn clamp_orientation_degrees(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(-180.0, 180.0)
    } else {
        0.0
    }
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

const FALLBACK_BEAT_MS: u64 = 1_000;
const BEATS_PER_MEASURE: u64 = 4;
const FALLBACK_MEASURE_MS: u64 = FALLBACK_BEAT_MS * BEATS_PER_MEASURE;
const FALLBACK_MEASURE_MS_F64: f64 = 4_000.0;
const MEASURE_PROGRESS_FACTOR: f64 = 0.25;

fn animated_beat_progress(now_ms: u64) -> f64 {
    f64::from((now_ms % FALLBACK_MEASURE_MS) as u32) / FALLBACK_MEASURE_MS_F64
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
        AudioDeviceConfig, AudioDeviceFlowConfig, DomersConfig, LevelDriverPresetConfig,
        MidiBindingAction, MidiBindingCommandKind, MidiBindingConfig, PaletteEntry, TempoSource,
        UdpInputConfig,
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
            if snapshot.inputs.volume == Some(0.42)
                && snapshot.inputs.audio_adapter.events == 1
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
        assert_eq!(state.metrics().frames, 0);
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
            ..super::SimulatorControlsPatch::default()
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
        state.set_now_ms_for_test(0);
        state.engine_frame();
        let first = state.operator_frame().dome;
        state.set_now_ms_for_test(250);
        let second = state.operator_frame().dome;

        assert_ne!(first, second);
    }

    #[test]
    fn fallback_preview_uses_spectrum_measure_timing() {
        let mut state = ServerState::default();
        state.start();

        state.set_now_ms_for_test(1_000);
        assert!((state.visualizer_controls().beat_progress - 0.25).abs() < f64::EPSILON);

        state.set_now_ms_for_test(4_000);
        assert!(state.visualizer_controls().beat_progress.abs() < f64::EPSILON);
    }

    #[test]
    fn tapped_tempo_visualizers_use_measure_progress() {
        let mut state = ServerState::default();
        state.start();
        state.tap_tempo_at(0);
        state.tap_tempo_at(500);
        state.tap_tempo_at(1_000);

        state.set_now_ms_for_test(1_250);

        assert!((state.snapshot().inputs.beat_progress - 0.5).abs() < 0.01);
        assert!((state.visualizer_controls().beat_progress - 0.125).abs() < 0.01);
    }

    #[test]
    fn preview_frames_do_not_advance_engine_time_or_beat_phase() {
        let mut state = ServerState::default();
        state.tap_tempo_at(0);
        state.tap_tempo_at(500);
        state.tap_tempo_at(1_000);
        state.set_now_ms_for_test(1_250);
        let before = state.snapshot().inputs.beat_progress;

        for _ in 0..10 {
            let _ = state.simulator_frame();
        }

        state.set_now_ms_for_test(1_250);
        let after = state.snapshot().inputs.beat_progress;
        assert_eq!(state.metrics().frames, 0);
        assert_eq!(state.metrics().simulator_frames, 10);
        assert!((after - before).abs() < 0.01);
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
            .any(|command| matches!(command, DomeCommand::Pixel { .. })));
        assert!(frame
            .dome
            .iter()
            .any(|command| matches!(command, DomeCommand::Flush)));
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
            orientation_override_enabled: Some(true),
            orientation_yaw: Some(90.0),
            orientation_pitch: Some(-45.0),
            orientation_roll: Some(270.0),
        });

        let snapshot = state.snapshot();

        assert!((snapshot.simulator.volume - 0.25).abs() < f32::EPSILON);
        assert!((snapshot.simulator.beat_progress - 0.75).abs() < f64::EPSILON);
        assert!(!snapshot.simulator.flash_active);
        assert!(snapshot.simulator.orientation_override_enabled);
        assert!((snapshot.simulator.orientation_yaw - 90.0).abs() < f64::EPSILON);
        assert!((snapshot.simulator.orientation_pitch + 45.0).abs() < f64::EPSILON);
        assert!((snapshot.simulator.orientation_roll - 180.0).abs() < f64::EPSILON);
    }

    #[test]
    fn tap_tempo_uses_wall_clock_instead_of_engine_frame_time() {
        let mut state = ServerState::default();

        state.tap_tempo();
        for _ in 0..400 {
            state.engine_frame();
        }
        std::thread::sleep(Duration::from_millis(120));
        state.tap_tempo();
        for _ in 0..400 {
            state.engine_frame();
        }
        std::thread::sleep(Duration::from_millis(120));
        state.tap_tempo();

        let beat_ms = state
            .snapshot()
            .inputs
            .beat_ms
            .expect("three taps should set tempo");
        assert!(
            (90..=180).contains(&beat_ms),
            "tap tempo used {beat_ms}ms instead of the real click interval"
        );
    }

    #[test]
    fn runtime_inputs_update_status_and_visualizer_controls() {
        let mut state = ServerState::default();
        state.apply_audio_volume(0.33);
        state.apply_midi_commands(&[
            MidiCommand {
                device_index: 0,
                kind: MidiCommandKind::Note,
                index: 64,
                value: 0.0,
            },
            MidiCommand {
                device_index: 0,
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
                device_index: None,
                command_kind: MidiBindingCommandKind::Note,
                index: 60,
                action: MidiBindingAction::TapTempo,
                target_index: None,
            },
            MidiBindingConfig {
                device_index: None,
                command_kind: MidiBindingCommandKind::ControlChange,
                index: 10,
                action: MidiBindingAction::Palette,
                target_index: Some(6),
            },
            MidiBindingConfig {
                device_index: None,
                command_kind: MidiBindingCommandKind::Program,
                index: 2,
                action: MidiBindingAction::Visualizer,
                target_index: Some(8),
            },
        ];
        let mut state = ServerState::new(config);

        state.apply_midi_commands(&[
            MidiCommand {
                device_index: 0,
                kind: MidiCommandKind::Note,
                index: 60,
                value: 1.0,
            },
            MidiCommand {
                device_index: 0,
                kind: MidiCommandKind::ControlChange,
                index: 10,
                value: 0.5,
            },
            MidiCommand {
                device_index: 0,
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

    #[test]
    fn adsr_midi_binding_drives_level_driver_channels() {
        let mut config = DomersConfig::default();
        config.level_drivers.presets.insert(
            "midi test".to_string(),
            LevelDriverPresetConfig::Midi {
                attack_time: 0,
                peak_level: 1.0,
                decay_time: 0,
                sustain_level: 0.75,
                release_time: 100,
            },
        );
        config
            .level_drivers
            .midi_channels
            .insert(0, "midi test".to_string());
        config.inputs.midi.bindings = vec![MidiBindingConfig {
            device_index: None,
            command_kind: MidiBindingCommandKind::Note,
            index: 48,
            action: MidiBindingAction::AdsrLevelDriver,
            target_index: None,
        }];
        let mut state = ServerState::new(config);

        state.apply_midi_commands(&[MidiCommand {
            device_index: 0,
            kind: MidiCommandKind::Note,
            index: 48,
            value: 0.8,
        }]);
        let snapshot = state.snapshot();

        assert_eq!(snapshot.inputs.midi_commands, 1);
        assert_eq!(snapshot.inputs.midi_log[0].actions, ["adsr:0:press"]);
        assert!((snapshot.inputs.midi_level_driver_channels[0].unwrap() - 0.6).abs() < 0.000_001);
        assert!((state.visualizer_controls().volume - 0.6).abs() < 0.000_001);

        state.apply_midi_commands(&[MidiCommand {
            device_index: 0,
            kind: MidiCommandKind::Note,
            index: 48,
            value: 0.0,
        }]);
        let snapshot = state.snapshot();

        assert_eq!(snapshot.inputs.midi_log[1].actions, ["adsr:0:release"]);
        assert!(snapshot.inputs.midi_level_driver_channels[0].is_some());
    }

    #[test]
    fn madmom_audio_index_derives_from_selected_capture_device() {
        let mut config = DomersConfig::default();
        config.inputs.audio.device_id = Some("mic-b".to_string());
        config.inputs.audio.devices = vec![
            AudioDeviceConfig {
                id: "speaker".to_string(),
                name: "Speaker".to_string(),
                flow: AudioDeviceFlowConfig::Render,
            },
            AudioDeviceConfig {
                id: "mic-a".to_string(),
                name: "Mic A".to_string(),
                flow: AudioDeviceFlowConfig::Capture,
            },
            AudioDeviceConfig {
                id: "loopback".to_string(),
                name: "Loopback".to_string(),
                flow: AudioDeviceFlowConfig::Render,
            },
            AudioDeviceConfig {
                id: "mic-b".to_string(),
                name: "Mic B".to_string(),
                flow: AudioDeviceFlowConfig::Capture,
            },
        ];

        assert_eq!(super::madmom_audio_input_index(&config), Some(3));
        config.madmom.audio_input_index = Some(9);
        assert_eq!(super::madmom_audio_input_index(&config), Some(9));
    }

    #[tokio::test]
    async fn runtime_udp_input_adapters_feed_live_state() {
        let audio_addr = free_udp_addr();
        let midi_addr = free_udp_addr();
        let orientation_addr = free_udp_addr();
        let mut config = DomersConfig::default();
        config.inputs.audio = domers_core::AudioInputConfig {
            bind: Some(audio_addr.to_string()),
            ..domers_core::AudioInputConfig::default()
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

    #[tokio::test]
    async fn runtime_link_fake_sidecar_feeds_tempo() {
        let script = env::temp_dir().join(format!("domers-fake-link-{}.sh", std::process::id()));
        fs::write(
            &script,
            "#!/bin/sh\nprintf 'LINK 120 0.25\\n'\nsleep 0.02\nprintf 'tempo=90 phase=0.5\\n'\n",
        )
        .expect("fake link sidecar script writes");

        let mut config = DomersConfig::default();
        config.tempo.source = TempoSource::Link;
        config.carabiner.command = "sh".to_string();
        config.carabiner.args = vec![script.to_string_lossy().to_string()];
        let runtime = AppRuntime::new(config);

        runtime.start().await;
        let mut snapshot = runtime.snapshot().await;
        for _ in 0..50 {
            if snapshot.inputs.link_adapter.events >= 2 {
                break;
            }
            time::sleep(Duration::from_millis(10)).await;
            snapshot = runtime.snapshot().await;
        }
        runtime.stop().await;
        let _ = fs::remove_file(script);

        assert_eq!(snapshot.inputs.link_adapter.events, 2);
        assert_eq!(snapshot.inputs.beat_ms, Some(667));
        assert_eq!(snapshot.inputs.bpm, "89");
        assert_eq!(snapshot.inputs.link_adapter.last_error, None);
    }

    #[cfg(not(feature = "native-capture"))]
    #[tokio::test]
    async fn native_capture_without_feature_reports_status_errors() {
        let mut config = DomersConfig::default();
        config.inputs.audio.native_enabled = true;
        config.inputs.midi.native_enabled = true;
        let runtime = AppRuntime::new(config);

        runtime.start().await;
        let mut snapshot = runtime.snapshot().await;
        for _ in 0..20 {
            if snapshot.inputs.audio_adapter.last_error.is_some()
                && snapshot.inputs.midi_adapter.last_error.is_some()
            {
                break;
            }
            time::sleep(Duration::from_millis(10)).await;
            snapshot = runtime.snapshot().await;
        }
        runtime.stop().await;

        assert_eq!(
            snapshot.inputs.audio_adapter.last_error.as_deref(),
            Some("native audio capture requires the native-capture build feature")
        );
        assert_eq!(
            snapshot.inputs.midi_adapter.last_error.as_deref(),
            Some("native MIDI capture requires the native-capture build feature")
        );
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
                ..super::SimulatorControlsPatch::default()
            })
            .await;

        let before = runtime.snapshot().await;
        let frame = runtime
            .simulator_sandbox_frame(super::SimulatorSandboxRequest {
                active_visualizer: Some(7),
                volume: Some(1.0),
                beat_progress: Some(0.9),
                flash_active: Some(true),
                orientation_override_enabled: Some(true),
                orientation_yaw: Some(45.0),
                orientation_pitch: Some(-30.0),
                orientation_roll: Some(15.0),
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
