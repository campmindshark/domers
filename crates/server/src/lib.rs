//! Runnable Domers server contract and HTTP/WebSocket adapter.

use std::{net::SocketAddr, sync::Arc, time::Duration};

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
use domers_core::{BeatBroadcaster, ColorPalette, DomersConfig, EngineConfig, PaletteEntry, Rgb};
use domers_engine::{schedule_operator_frame, FullVisualizerSpec, InputSpec, OutputSpec};
use domers_inputs::{classify_datagram, parse_beat_line, MidiCommand, MidiCommandKind};
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
    net::TcpListener,
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
    /// Hardware output connection status.
    pub hardware: HardwareStatus,
    /// Live input status and latest runtime values.
    pub inputs: InputStatus,
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
    /// Last recognized orientation datagram kind.
    pub last_orientation: Option<String>,
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
        VisualizerInput {
            volume: self.volume,
            beat_progress: self.beat_progress,
            flash_active: self.flash_active,
            primary: config
                .color_palette
                .single_color(0, config.color_palette_index),
            secondary: config
                .color_palette
                .single_color(1, config.color_palette_index),
            accent: config
                .color_palette
                .single_color(2, config.color_palette_index),
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

    /// Patch one runtime color palette entry.
    pub fn patch_palette_entry(&mut self, patch: PaletteEntryPatch) {
        let absolute_index = ColorPalette::absolute_index(
            usize::from(patch.relative_index.min(7)),
            self.config.color_palette_index,
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
            self.inputs.beat.report_madmom_beat(beat_ms, self.now_ms());
            true
        } else {
            false
        }
    }

    /// Apply MIDI commands to runtime state.
    pub fn apply_midi_commands(&mut self, commands: &[MidiCommand]) {
        self.inputs.midi_commands = self
            .inputs
            .midi_commands
            .saturating_add(commands.len() as u64);
        for command in commands {
            match command.kind {
                MidiCommandKind::Note if command.index == 64 => {
                    self.simulator.flash_active = command.value > 0.0;
                }
                MidiCommandKind::ControlChange if command.index == 1 => {
                    self.simulator.volume = command.value.clamp(0.0, 1.0);
                }
                _ => {}
            }
        }
    }

    /// Classify and record one orientation datagram.
    pub fn apply_orientation_datagram(&mut self, bytes: &[u8]) -> bool {
        if let Some(kind) = classify_datagram(bytes) {
            self.inputs.last_orientation = Some(format!("{kind:?}"));
            true
        } else {
            false
        }
    }

    /// Produce one deterministic simulator frame for the selected visualizer.
    pub fn simulator_frame(&mut self) -> Vec<DomeCommand> {
        self.metrics.frames = self.metrics.frames.saturating_add(1);
        self.metrics.simulator_frames = self.metrics.simulator_frames.saturating_add(1);
        self.operator_frame().dome
    }

    fn record_simulator_frame(&mut self) {
        self.metrics.simulator_frames = self.metrics.simulator_frames.saturating_add(1);
    }

    /// Produce one scheduled operator frame for all outputs.
    #[must_use]
    pub fn operator_frame(&self) -> OperatorCommandFrame {
        render_operator_frame(&self.config, self.visualizer_controls())
    }

    /// Return a serializable snapshot.
    #[must_use]
    pub fn snapshot(&self) -> ServerSnapshot {
        ServerSnapshot {
            running: self.running,
            config: EngineConfig::from(&self.config),
            metrics: self.metrics,
            simulator: self.simulator,
            hardware: HardwareStatus::default(),
            inputs: self.input_status(),
        }
    }

    fn now_ms(&self) -> u64 {
        self.metrics.frames.saturating_mul(2_500) / 1_000
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
            last_orientation: self.inputs.last_orientation.clone(),
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
    last_orientation: Option<String>,
}

/// Shared runnable app runtime.
#[derive(Clone)]
pub struct AppRuntime {
    state: Arc<Mutex<ServerState>>,
    hardware: Arc<Mutex<HardwareOutputs>>,
    frames: broadcast::Sender<SimulatorFrame>,
    engine_task: Arc<Mutex<Option<JoinHandle<()>>>>,
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
        Self {
            state: Arc::new(Mutex::new(ServerState::new(config))),
            hardware: Arc::new(Mutex::new(HardwareOutputs::default())),
            frames,
            engine_task: Arc::new(Mutex::new(None)),
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
            .route("/api/config/dome", patch(patch_dome_config))
            .route("/api/config/palette", patch(patch_palette_entry))
            .route("/api/input/tap", post(tap_tempo))
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
            let state = self.state.lock().await;
            (state.snapshot(), state.full_config())
        };
        snapshot.hardware = self.hardware.lock().await.status_for_config(&config);
        snapshot
    }

    /// Start the engine task if it is not already running.
    pub async fn start(&self) {
        self.state.lock().await.start();

        let mut task = self.engine_task.lock().await;
        if task.as_ref().is_some_and(|handle| !handle.is_finished()) {
            return;
        }

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
    }

    /// Patch dome runtime configuration.
    pub async fn patch_dome_config(&self, patch: DomeConfigPatch) {
        self.state.lock().await.patch_dome_config(patch);
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

    /// Produce one simulator frame immediately.
    pub async fn simulator_frame(&self) -> SimulatorFrame {
        let mut state = self.state.lock().await;
        let commands = state.simulator_frame();
        SimulatorFrame {
            metrics: state.metrics(),
            commands: serialize_commands(commands),
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
/// Runtime color palette patch payload.
pub struct PaletteEntryPatch {
    /// Color index within the active palette bank.
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

async fn patch_dome_config(
    State(runtime): State<AppRuntime>,
    Json(patch): Json<DomeConfigPatch>,
) -> Json<ServerSnapshot> {
    runtime.patch_dome_config(patch).await;
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

fn render_operator_frame(
    config: &DomersConfig,
    simulator: SimulatorControls,
) -> OperatorCommandFrame {
    let engine = EngineConfig::from(config);
    let inputs = input_specs(simulator);
    let outputs = output_specs(config);
    let schedule = schedule_operator_frame(&inputs, &outputs);
    let visualizer_input = simulator.visualizer_input(&engine);
    let diagnostic_input = DiagnosticInput {
        state: 1,
        step: 0,
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
        io::{ErrorKind, Read, Write},
        net::{SocketAddr, TcpStream},
        time::Duration,
    };

    use domers_core::{DomersConfig, PaletteEntry};
    use domers_inputs::{MidiCommand, MidiCommandKind};
    use domers_outputs::DomeCommand;
    use tokio::time;
    use tokio::{io::AsyncReadExt, net::TcpListener};

    use super::{health, serve_listener, AppRuntime, ServerState, SimulatorCommand};

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
        assert!(!snapshot.simulator.flash_active);
        assert!((snapshot.simulator.volume - 0.5).abs() < f32::EPSILON);
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
        assert!(html.contains("MindShark Dome Controls"));

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
