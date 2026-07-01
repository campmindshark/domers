use domers_outputs::{dome_strut_length, topology::DOME_STRUTS, DomeCommand};

use crate::{
    dome::{race_pixel_color, RaceRacer, RACE_RACER_CONFIGS},
    geometry::{build_dome_led_points, DomeLedPoint, DOME_LED_POINTS},
    input::VisualizerInput,
};

#[derive(Clone, Debug)]
pub(crate) struct RaceRuntime {
    racers: Vec<RaceRacer>,
    last_ms: Option<u64>,
}

impl RaceRuntime {
    pub(crate) fn new() -> Self {
        Self {
            racers: RACE_RACER_CONFIGS
                .iter()
                .copied()
                .map(RaceRacer::new)
                .collect(),
            last_ms: None,
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "Millisecond deltas are small and converted to fractional seconds"
    )]
    pub(crate) fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        if let Some(last) = self.last_ms {
            let num_seconds = input.now_ms.saturating_sub(last) as f64 / 1000.0;
            let volume = f64::from(input.volume.clamp(0.0, 1.0));
            for racer in &mut self.racers {
                racer.move_racer(num_seconds, volume, input.measure_length_ms);
            }
        }
        self.last_ms = Some(input.now_ms);

        let points = DOME_LED_POINTS.get_or_init(build_dome_led_points);
        let mut point_index = 0;
        let start_angles = self.racer_start_angles();
        for strut_index in 0..DOME_STRUTS {
            let Some(strut_length) = dome_strut_length(strut_index) else {
                continue;
            };
            for led_index in 0..strut_length {
                let point = points.get(point_index).copied().unwrap_or(DomeLedPoint {
                    index: point_index,
                    x: 0.5,
                    y: 0.5,
                });
                point_index += 1;
                out.push(DomeCommand::Pixel {
                    strut_index,
                    led_index,
                    color: race_pixel_color(*input, point.x, point.y, Some(start_angles)),
                });
            }
        }
        out.push(DomeCommand::Flush);
    }

    pub(crate) fn racer_start_angles(&self) -> [f64; 4] {
        let mut angles = [0.0; 4];
        for (index, racer) in self.racers.iter().enumerate().take(angles.len()) {
            angles[index] = racer.angle;
        }
        angles
    }
}
