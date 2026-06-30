//! Fixture topology constants captured from the Spectrum inventory.

/// Number of logical dome struts.
pub const DOME_STRUTS: usize = 190;
/// Number of logical dome LEDs across all struts.
pub const DOME_PIXELS: usize = 7_580;
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
    use super::{
        BAR_DOME_CONTROL_BOX, DOME_PIXELS, DOME_STRUTS, DOME_VERTICES, STAGE_LAYERS, STAGE_SIDES,
    };

    #[test]
    fn captures_known_fixture_counts() {
        assert_eq!(DOME_STRUTS, 190);
        assert_eq!(DOME_PIXELS, 7_580);
        assert_eq!(DOME_VERTICES, 71);
        assert_eq!(BAR_DOME_CONTROL_BOX, 5);
        assert_eq!(STAGE_SIDES, 48);
        assert_eq!(STAGE_LAYERS, 3);
    }

    #[test]
    fn matches_extracted_mapping_and_geometry_fixtures() {
        let mapping = include_str!("../../../fixtures/spectrum-csharp/dome_mapping.json");
        assert!(mapping.contains(r#""strut_count": 190"#));
        assert!(mapping.contains(r#""bar_control_box": 5"#));

        let geometry = include_str!("../../../fixtures/spectrum-csharp/dome_geometry.json");
        assert!(geometry.contains(r#""line_count": 190"#));
        assert!(geometry.contains(r#""point_count": 71"#));

        let topology = include_str!("../../../fixtures/spectrum-csharp/bar_stage_topology.json");
        assert!(topology.contains(r#""side_count": 48"#));
        assert!(topology.contains(r#""layer_count": 3"#));
    }
}
