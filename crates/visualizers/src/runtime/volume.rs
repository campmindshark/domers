use domers_outputs::DomeCommand;

use crate::{
    dome::{volume_center_offset_for_input, volume_commands_with_wipe, volume_wipe_commands},
    input::VisualizerInput,
};

/// Persistent Volume runtime tracking center-offset changes (Spectrum `UpdateCenter`).
#[derive(Clone, Debug)]
pub(crate) struct VolumeRuntime {
    last_center_offset: Option<usize>,
    seen_frame: bool,
}

impl VolumeRuntime {
    pub(crate) fn new() -> Self {
        Self {
            last_center_offset: None,
            seen_frame: false,
        }
    }

    pub(crate) fn render(&mut self, input: &VisualizerInput, out: &mut Vec<DomeCommand>) {
        let beat_progress = if input.animation_frame == 0 {
            0.0
        } else {
            input.beat_progress
        };
        let center = volume_center_offset_for_input(input, beat_progress);
        let center_changed = self.last_center_offset.is_some_and(|last| last != center);
        let include_initial_wipe = input.animation_frame == 0 && !self.seen_frame;
        if center_changed && self.seen_frame {
            out.extend(volume_wipe_commands());
        }
        out.extend(volume_commands_with_wipe(
            *input,
            beat_progress,
            include_initial_wipe,
        ));
        self.last_center_offset = Some(center);
        self.seen_frame = true;
    }
}
