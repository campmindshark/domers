//! Fixture topology constants captured from the Spectrum inventory.

/// Number of logical dome struts.
pub const DOME_STRUTS: usize = 190;
/// Number of dome projection vertices.
pub const DOME_VERTICES: usize = 71;
/// Bar control box index when routed through dome OPC.
pub const BAR_DOME_CONTROL_BOX: usize = 5;
/// Stage side count.
pub const STAGE_SIDES: usize = 48;
/// Stage layer count.
pub const STAGE_LAYERS: usize = 3;

#[cfg(test)]
mod tests {
    use super::{BAR_DOME_CONTROL_BOX, DOME_STRUTS, DOME_VERTICES, STAGE_LAYERS, STAGE_SIDES};

    #[test]
    fn captures_known_fixture_counts() {
        assert_eq!(DOME_STRUTS, 190);
        assert_eq!(DOME_VERTICES, 71);
        assert_eq!(BAR_DOME_CONTROL_BOX, 5);
        assert_eq!(STAGE_SIDES, 48);
        assert_eq!(STAGE_LAYERS, 3);
    }
}
