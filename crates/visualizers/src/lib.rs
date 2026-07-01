//! Visualizer inventory and porting order.

#![allow(
    clippy::large_types_passed_by_value,
    reason = "VisualizerInput is Copy and passed by value throughout the Spectrum port"
)]

mod buffer;
mod color_util;
mod diagnostics;
mod dome;
mod geometry;
#[cfg(test)]
mod hash;
mod input;
mod inventory;
mod math;
mod quaternion;
mod render;
mod rng;
mod runtime;

#[cfg(test)]
mod tests;

pub use dome::{VOLUME_GRADIENT_SPEED, VOLUME_ROTATION_SPEED};
pub use input::{
    BarDiagnosticVisualizer, DiagnosticInput, DomeDiagnosticVisualizer, LiveVisualizer,
    MidiNoteInput, OrientationDeviceInput, OrientationOverride, StageVisualizer,
    StageVisualizerInput, VisualizerInput, MAX_FRAME_MIDI_NOTES, MAX_ORIENTATION_DEVICES,
};
pub use inventory::{Classification, VisualizerInventory, INVENTORY};
pub use quaternion::Quaternion;
pub use render::{
    render_bar_diagnostic, render_dome_diagnostic, render_dome_visualizer, render_stage_visualizer,
    render_stage_visualizer_with_input,
};
pub use runtime::VisualizerRuntime;
