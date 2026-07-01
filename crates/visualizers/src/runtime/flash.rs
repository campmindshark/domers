use domers_outputs::DomeCommand;

use crate::{
    dome::{
        animate_flash_polygon, clear_flash_strut, concentric_layout_from_point,
        flash_layout_struts, FlashPolygonAnimation, FlashShape,
    },
    input::VisualizerInput,
    rng::DotNetRandom,
};

/// Persistent Flash runtime mirroring `LEDDomeFlashVisualizer`.
#[derive(Clone, Debug)]
pub(crate) struct FlashRuntime {
    shapes: Vec<FlashShape>,
    pads_to_last_animation: [Option<usize>; 16],
    rng: DotNetRandom,
    last_user_animation_created: u64,
}

impl FlashRuntime {
    pub(crate) fn new() -> Self {
        let mut shapes = Vec::with_capacity(51);
        for starting_point in 20..=70 {
            let layout = concentric_layout_from_point(starting_point, 2);
            let struts = flash_layout_struts(&layout);
            shapes.push(FlashShape {
                layout,
                struts,
                animation: None,
            });
        }
        Self {
            shapes,
            pads_to_last_animation: [None; 16],
            rng: DotNetRandom::new(0),
            last_user_animation_created: 0,
        }
    }

    pub(crate) fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        let now_ms = input.now_ms;

        for (shape_index, shape) in self.shapes.iter_mut().enumerate() {
            let Some(animation) = shape.animation.as_ref() else {
                continue;
            };
            if animation.active(now_ms, FlashShape::enabled()) {
                continue;
            }
            if self.pads_to_last_animation[animation.pad as usize] == Some(shape_index) {
                self.pads_to_last_animation[animation.pad as usize] = None;
            }
            for &strut_index in &shape.struts {
                clear_flash_strut(strut_index, out);
            }
            shape.animation = None;
        }

        let measure_length_ms = input.measure_length_ms.unwrap_or(400);
        'midi: for note in input.midi_notes.into_iter().flatten() {
            if note.index > 15 {
                continue;
            }
            let pad = note.index as usize;
            if let Some(shape_index) = self.pads_to_last_animation[pad] {
                if let Some(animation) = self.shapes[shape_index].animation.as_mut() {
                    animation.release(now_ms);
                }
                if note.value == 0.0 {
                    continue;
                }
            }
            let Some(shape_index) = self.random_available_shape_index() else {
                break 'midi;
            };
            let starting_time = now_ms;
            self.shapes[shape_index].animation = Some(FlashPolygonAnimation::new(
                note.index,
                note.value,
                measure_length_ms,
                starting_time,
            ));
            self.pads_to_last_animation[pad] = Some(shape_index);
            self.last_user_animation_created = starting_time;
        }

        for shape in &self.shapes {
            let Some(animation) = shape.animation.as_ref() else {
                continue;
            };
            if !animation.active(now_ms, FlashShape::enabled()) {
                continue;
            }
            animate_flash_polygon(shape, animation, input, now_ms, out);
        }
    }

    pub(crate) fn random_available_shape_index(&mut self) -> Option<usize> {
        let available: Vec<usize> = self
            .shapes
            .iter()
            .enumerate()
            .filter_map(|(index, shape)| shape.available().then_some(index))
            .collect();
        if available.is_empty() {
            return None;
        }
        let len = i32::try_from(available.len()).unwrap_or(i32::MAX);
        let pick = self.rng.next_int(0, len);
        Some(available[usize::try_from(pick).unwrap_or(0)])
    }
}
