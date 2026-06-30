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
use domers_core::EngineConfig;
use domers_outputs::DomeCommand;
use domers_visualizers::{render_dome_visualizer, LiveVisualizer, VisualizerInput};
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ServerSnapshot {
    /// Whether the engine loop is currently running.
    pub running: bool,
    /// Active engine config.
    pub config: EngineConfig,
    /// Runtime counters.
    pub metrics: Metrics,
}

/// Browser-facing simulator frame.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SimulatorFrame {
    /// Metrics after this frame was produced.
    pub metrics: Metrics,
    /// Dome simulator commands for the frame.
    pub commands: Vec<SimulatorCommand>,
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

/// In-process server state shared by HTTP handlers and the engine task.
#[derive(Clone, Debug, Default)]
pub struct ServerState {
    config: EngineConfig,
    metrics: Metrics,
    running: bool,
}

impl ServerState {
    /// Create server state from an engine config.
    #[must_use]
    pub const fn new(config: EngineConfig) -> Self {
        Self {
            config,
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
        self.config.clone()
    }

    /// Patch the active dome visualizer.
    pub fn patch_dome_active_vis(&mut self, dome_active_vis: u8) {
        self.config.dome_active_vis = dome_active_vis;
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

    /// Produce one deterministic simulator frame for the selected visualizer.
    pub fn simulator_frame(&mut self) -> Vec<DomeCommand> {
        self.metrics.frames = self.metrics.frames.saturating_add(1);
        self.metrics.simulator_frames = self.metrics.simulator_frames.saturating_add(1);
        let visualizer = match self.config.dome_active_vis {
            1 => LiveVisualizer::Radial,
            2 => LiveVisualizer::Race,
            3 => LiveVisualizer::Snakes,
            4 => LiveVisualizer::QuaternionTest,
            5 => LiveVisualizer::QuaternionMultiTest,
            6 => LiveVisualizer::QuaternionPaintbrush,
            7 => LiveVisualizer::Splat,
            _ => LiveVisualizer::Volume,
        };
        render_dome_visualizer(visualizer, VisualizerInput::default())
    }

    /// Return a serializable snapshot.
    #[must_use]
    pub fn snapshot(&self) -> ServerSnapshot {
        ServerSnapshot {
            running: self.running,
            config: self.config.clone(),
            metrics: self.metrics,
        }
    }
}

/// Shared runnable app runtime.
#[derive(Clone)]
pub struct AppRuntime {
    state: Arc<Mutex<ServerState>>,
    frames: broadcast::Sender<SimulatorFrame>,
    engine_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Default for AppRuntime {
    fn default() -> Self {
        Self::new(EngineConfig::default())
    }
}

impl AppRuntime {
    /// Create a runtime from an engine config.
    #[must_use]
    pub fn new(config: EngineConfig) -> Self {
        let (frames, _) = broadcast::channel(32);
        Self {
            state: Arc::new(Mutex::new(ServerState::new(config))),
            frames,
            engine_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Build the HTTP/WebSocket router.
    pub fn router(self) -> Router {
        Router::new()
            .route("/", get(index_html))
            .route("/main.mjs", get(main_js))
            .route("/api/health", get(health_json))
            .route("/api/state", get(get_state))
            .route("/api/start", post(start_engine))
            .route("/api/stop", post(stop_engine))
            .route("/api/config/dome", patch(patch_dome_config))
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
        self.state.lock().await.snapshot()
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
        self.state.lock().await.stop();
        if let Some(task) = self.engine_task.lock().await.take() {
            task.abort();
        }
    }

    /// Patch the active dome visualizer.
    pub async fn patch_dome_active_vis(&self, dome_active_vis: u8) {
        self.state
            .lock()
            .await
            .patch_dome_active_vis(dome_active_vis);
    }

    async fn run_engine_loop(self) {
        let mut interval = time::interval(ENGINE_FRAME_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut frame_count = 0_u64;

        loop {
            interval.tick().await;
            frame_count = frame_count.saturating_add(1);

            let maybe_frame = {
                let mut state = self.state.lock().await;
                if !state.running() {
                    return;
                }

                #[allow(
                    clippy::manual_is_multiple_of,
                    reason = "The is_multiple_of method is newer than the workspace MSRV"
                )]
                if frame_count % SIMULATOR_FRAME_STRIDE == 0 {
                    let commands = state.simulator_frame();
                    Some(SimulatorFrame {
                        metrics: state.metrics(),
                        commands: serialize_commands(commands),
                    })
                } else {
                    state.engine_frame();
                    None
                }
            };

            if let Some(frame) = maybe_frame {
                let _ = self.frames.send(frame);
            }
        }
    }
}

/// Serve Domers on the provided socket address.
///
/// # Errors
///
/// Returns an error if the TCP listener cannot bind or the HTTP server fails.
pub async fn serve(addr: SocketAddr, config: EngineConfig) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, AppRuntime::new(config).router()).await
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct DomeConfigPatch {
    active_visualizer: Option<u8>,
}

async fn index_html() -> Html<&'static str> {
    Html(include_str!("../../../ui/index.html"))
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
    if let Some(active_visualizer) = patch.active_visualizer {
        runtime.patch_dome_active_vis(active_visualizer).await;
    }
    Json(runtime.snapshot().await)
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use domers_outputs::DomeCommand;
    use tokio::time;

    use super::{health, AppRuntime, ServerState};

    #[test]
    fn health_is_ok() {
        assert_eq!(health(), "ok");
    }

    #[test]
    fn patches_config_and_streams_simulator_frame() {
        let mut state = ServerState::default();
        state.start();
        state.patch_dome_active_vis(1);

        let frame = state.simulator_frame();

        assert!(state.running());
        assert_eq!(state.config().dome_active_vis, 1);
        assert!(frame
            .iter()
            .any(|command| matches!(command, DomeCommand::Frame(_))));
        assert_eq!(state.metrics().frames, 1);
        assert_eq!(state.metrics().simulator_frames, 1);
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

    #[test]
    fn stop_updates_running_state_without_dropping_config() {
        let mut state = ServerState::default();
        state.patch_dome_active_vis(7);
        state.start();
        state.stop();

        assert!(!state.running());
        assert_eq!(state.config().dome_active_vis, 7);
    }
}
