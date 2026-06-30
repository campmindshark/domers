//! Engine configuration model skeleton.

/// Minimal engine configuration used by the initial scheduler/output tests.
#[derive(Clone, Debug)]
pub struct EngineConfig {
    /// Whether the dome hardware path is enabled.
    pub dome_enabled: bool,
    /// Whether the dome simulator path is enabled.
    pub dome_simulation_enabled: bool,
    /// Active dome visualizer index, matching Spectrum's `domeActiveVis`.
    pub dome_active_vis: u8,
    /// Dome diagnostic pattern.
    pub dome_test_pattern: u8,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            dome_enabled: false,
            dome_simulation_enabled: true,
            dome_active_vis: 0,
            dome_test_pattern: 0,
        }
    }
}
