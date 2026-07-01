use domers_outputs::DomeCommand;

use crate::{
    buffer::DomeBuffer,
    input::VisualizerInput,
    math::{map_value, SPLAT_FADE},
    rng::DotNetRandom,
};

/// Persistent Splat runtime porting `LEDDomeSplatVisualizer`.
#[derive(Clone, Debug)]
pub(crate) struct SplatRuntime {
    buffer: DomeBuffer,
    rng: DotNetRandom,
    last_progress: f64,
    seen_first: bool,
}

impl SplatRuntime {
    pub(crate) fn new() -> Self {
        Self {
            buffer: DomeBuffer::new(),
            rng: DotNetRandom::new(0),
            last_progress: 0.0,
            seen_first: false,
        }
    }

    #[allow(
        clippy::cast_sign_loss,
        reason = "Spectrum indexes the palette with a non-negative Random.Next() modulo 8"
    )]
    pub(crate) fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        let level = f64::from(input.volume.clamp(0.0, 1.0));
        let adjusted_level = level.sqrt().clamp(0.1, 1.0);
        let progress = input.beat_progress;

        self.buffer.fade(SPLAT_FADE, 0.0);

        if self.seen_first && progress < self.last_progress {
            let cx = map_value(self.rng.next_double(), 0.0, 1.0, 0.1, 0.9);
            let cy = map_value(self.rng.next_double(), 0.0, 1.0, 0.1, 0.9);
            let radius = adjusted_level * 0.25;
            let color_index = (self.rng.next().rem_euclid(8)) as usize;
            for pixel in &mut self.buffer.pixels {
                let dx = pixel.x - cx;
                let dy = pixel.y - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < radius {
                    let color =
                        input.palette_entries[color_index].gradient_color(dist / radius, 0.0, true);
                    pixel.set_color(color);
                }
            }
        }

        self.last_progress = progress;
        self.seen_first = true;
        out.extend(self.buffer.frame_commands());
    }
}
