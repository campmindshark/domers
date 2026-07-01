use std::sync::OnceLock;

use domers_outputs::topology::DOME_PIXELS;
use serde::Deserialize;

pub(crate) const DOME_GEOMETRY_JSON: &str =
    include_str!("../../../fixtures/spectrum-csharp/dome_geometry.json");
pub(crate) const DOME_MAPPING_JSON: &str =
    include_str!("../../../fixtures/spectrum-csharp/dome_mapping.json");
pub(crate) static DOME_LED_POINTS: OnceLock<Vec<DomeLedPoint>> = OnceLock::new();

#[derive(Clone, Copy, Debug)]
pub(crate) struct DomeLedPoint {
    pub(crate) index: usize,
    pub(crate) x: f64,
    pub(crate) y: f64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DomeGeometryFixture {
    hand_drawn_points: Vec<GeometryPoint>,
    lines: Vec<GeometryLine>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GeometryPoint {
    normalized_x: f64,
    normalized_y: f64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GeometryLine {
    start: usize,
    end: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DomeMappingFixture {
    control_box_strut_order: Vec<Vec<String>>,
    strut_lengths: std::collections::HashMap<String, usize>,
    strut_positions: Vec<StrutPosition>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StrutPosition {
    control_box_strut_index: usize,
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Dome fixture LED counts are small and converted only for normalized interpolation"
)]
pub(crate) fn build_dome_led_points() -> Vec<DomeLedPoint> {
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
    pub(crate) fn strut_length(&self, control_box_strut_index: usize) -> usize {
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

pub(crate) fn hemisphere_point(normalized_x: f64, normalized_y: f64) -> (f64, f64, f64) {
    let x = 2.0 * normalized_x - 1.0;
    let y = 1.0 - 2.0 * normalized_y;
    let z = if x.mul_add(x, y * y) > 1.0 {
        0.0
    } else {
        (1.0 - x * x - y * y).sqrt()
    };
    (x, y, z)
}

pub(crate) fn distance3(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> f64 {
    ((ax - bx).powi(2) + (ay - by).powi(2) + (az - bz).powi(2)).sqrt()
}

pub(crate) fn distance2(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
}
