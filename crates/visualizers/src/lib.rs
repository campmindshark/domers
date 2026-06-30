//! Visualizer inventory and porting order.

/// Porting classification for a Spectrum visualizer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Classification {
    /// Must be ported for parity.
    Live,
    /// Port as diagnostic, fixture, or helper.
    Support,
    /// Do not port unless intentionally redesigned.
    Dead,
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
    VisualizerInventory { name: "LEDDomeVolumeVisualizer", classification: Classification::Live },
    VisualizerInventory { name: "LEDDomeFlashVisualizer", classification: Classification::Live },
    VisualizerInventory { name: "LEDDomeRadialVisualizer", classification: Classification::Live },
    VisualizerInventory { name: "LEDDomeRaceVisualizer", classification: Classification::Live },
    VisualizerInventory { name: "LEDDomeSnakesVisualizer", classification: Classification::Live },
    VisualizerInventory { name: "LEDDomeSplatVisualizer", classification: Classification::Live },
    VisualizerInventory { name: "LEDStageDepthLevelVisualizer", classification: Classification::Live },
    VisualizerInventory { name: "LEDDomeMidiTestVisualizer", classification: Classification::Dead },
    VisualizerInventory { name: "LEDStageTracerVisualizer", classification: Classification::Dead },
];

#[cfg(test)]
mod tests {
    use super::{Classification, INVENTORY};

    #[test]
    fn records_confirmed_dead_visualizers() {
        assert!(INVENTORY.iter().any(|v| v.name == "LEDDomeMidiTestVisualizer" && v.classification == Classification::Dead));
        assert!(INVENTORY.iter().any(|v| v.name == "LEDStageTracerVisualizer" && v.classification == Classification::Dead));
    }
}
