use domers_outputs::DomeCommand;

use crate::{
    buffer::DomeBuffer, dome::quaternion_multi_test_color_at, input::VisualizerInput,
    math::DOME_GLOBAL_FADE_SPEED,
};

/// Persistent multi-test runtime matching `LEDDomeQuaternionMultiTestVisualizer`.
#[derive(Clone, Debug)]
pub(crate) struct QuaternionMultiRuntime {
    buffer: DomeBuffer,
}

impl QuaternionMultiRuntime {
    pub(crate) fn new() -> Self {
        Self {
            buffer: DomeBuffer::new(),
        }
    }

    pub(crate) fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        self.buffer
            .fade(1.0 - 5f64.powf(-DOME_GLOBAL_FADE_SPEED), 0.0);

        for (point_index, pixel) in self.buffer.pixels.iter_mut().enumerate() {
            let spot = quaternion_multi_test_color_at(input, point_index);
            if spot != domers_core::Rgb::BLACK {
                pixel.blend_light_paint(spot);
            }
        }

        out.extend(self.buffer.frame_commands());
    }
}
