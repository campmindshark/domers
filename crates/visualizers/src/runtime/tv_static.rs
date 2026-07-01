use domers_outputs::{dome_strut_length, topology::DOME_STRUTS, DomeCommand};

use crate::rng::DotNetRandom;

/// Persistent TV Static runtime mirroring `LEDDomeTVStaticVisualizer`, which
/// advances one long-lived `Random` across frames rather than reseeding.
#[derive(Clone, Debug)]
pub(crate) struct TvStaticRuntime {
    rng: DotNetRandom,
}

impl TvStaticRuntime {
    pub(crate) fn new() -> Self {
        Self {
            rng: DotNetRandom::new(0),
        }
    }

    pub(crate) fn render(&mut self, out: &mut Vec<DomeCommand>) {
        for strut_index in 0..DOME_STRUTS {
            let Some(strut_length) = dome_strut_length(strut_index) else {
                continue;
            };
            for led_index in 0..strut_length {
                out.push(DomeCommand::Pixel {
                    strut_index,
                    led_index,
                    color: self.rng.next_color(255),
                });
            }
        }
        out.push(DomeCommand::Flush);
    }
}
