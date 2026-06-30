//! Minimal server contract placeholder.

use domers_core::EngineConfig;
use domers_outputs::DomeCommand;
use domers_visualizers::{render_dome_visualizer, LiveVisualizer, VisualizerInput};

/// Health status returned by the early API.
#[must_use]
pub const fn health() -> &'static str {
    "ok"
}

/// Runtime metrics exposed by the server contract.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    /// Operator frames produced.
    pub frames: u64,
    /// Simulator frames produced.
    pub simulator_frames: u64,
}

/// Minimal in-process server state used before the HTTP/WebSocket adapter.
#[derive(Clone, Debug, Default)]
pub struct ServerState {
    config: EngineConfig,
    metrics: Metrics,
    running: bool,
}

impl ServerState {
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
}

#[cfg(test)]
mod tests {
    use domers_outputs::DomeCommand;

    use super::{health, ServerState};

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
