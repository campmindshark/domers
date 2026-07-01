use domers_outputs::{BarCommand, DomeCommand, DomeOutputSink, StageCommand};

use crate::{
    diagnostics::{
        bar_flash_colors, dome_flash_colors_commands, dome_full_color_flash_commands,
        dome_strand_test_commands, dome_strut_iteration_commands, stage_depth_level,
        stage_flash_colors,
    },
    dome::{
        quaternion_multi_test_frame, quaternion_paintbrush_frame, quaternion_test_frame,
        race_commands, radial_frame, snakes_commands, splat_frame, tv_static_commands,
        volume_commands,
    },
    input::{
        BarDiagnosticVisualizer, DiagnosticInput, DomeDiagnosticVisualizer, LiveVisualizer,
        StageVisualizer, StageVisualizerInput, VisualizerInput,
    },
};

/// Render one deterministic simulator frame for a live visualizer.
#[must_use]
pub fn render_dome_visualizer(
    visualizer: LiveVisualizer,
    input: VisualizerInput,
) -> Vec<DomeCommand> {
    if visualizer == LiveVisualizer::Flash {
        return Vec::new();
    }
    if visualizer == LiveVisualizer::TvStatic {
        return tv_static_commands(input);
    }
    if visualizer == LiveVisualizer::Snakes {
        return snakes_commands(input);
    }
    if visualizer == LiveVisualizer::Race {
        return race_commands(input);
    }
    if visualizer == LiveVisualizer::Volume {
        return volume_commands(input);
    }
    let mut sink = DomeOutputSink::new(false, true);
    sink.write_buffer(match visualizer {
        LiveVisualizer::TvStatic => unreachable!("TV Static writes Spectrum-style pixel commands"),
        LiveVisualizer::Volume => unreachable!("Volume writes Spectrum-style pixel commands"),
        LiveVisualizer::Flash => unreachable!("Flash visualizer is event-driven"),
        LiveVisualizer::Radial => radial_frame(input),
        LiveVisualizer::Splat => splat_frame(input),
        LiveVisualizer::Race => unreachable!("Race writes Spectrum-style pixel commands"),
        LiveVisualizer::Snakes => unreachable!("Snakes writes Spectrum-style pixel commands"),
        LiveVisualizer::QuaternionTest => quaternion_test_frame(input),
        LiveVisualizer::QuaternionMultiTest => quaternion_multi_test_frame(input),
        LiveVisualizer::QuaternionPaintbrush => quaternion_paintbrush_frame(input),
    });
    sink.flush();
    sink.drain_commands()
}
/// Render one used dome diagnostic visualizer frame.
#[must_use]
pub fn render_dome_diagnostic(
    visualizer: DomeDiagnosticVisualizer,
    input: DiagnosticInput,
) -> Vec<DomeCommand> {
    match visualizer {
        DomeDiagnosticVisualizer::FlashColors => dome_flash_colors_commands(input),
        DomeDiagnosticVisualizer::StrutIteration => dome_strut_iteration_commands(input),
        DomeDiagnosticVisualizer::StrandTest => dome_strand_test_commands(input),
        DomeDiagnosticVisualizer::FullColorFlash => dome_full_color_flash_commands(input),
    }
}

/// Render one used bar diagnostic visualizer frame.
#[must_use]
pub fn render_bar_diagnostic(
    visualizer: BarDiagnosticVisualizer,
    input: DiagnosticInput,
    infinity_width: usize,
    infinity_length: usize,
    runner_length: usize,
) -> Vec<BarCommand> {
    match visualizer {
        BarDiagnosticVisualizer::FlashColors => {
            bar_flash_colors(input, infinity_width, infinity_length, runner_length)
        }
    }
}

/// Render one used stage visualizer frame.
#[must_use]
pub fn render_stage_visualizer(
    visualizer: StageVisualizer,
    input: DiagnosticInput,
    side_lengths: &[usize],
) -> Vec<StageCommand> {
    render_stage_visualizer_with_input(
        visualizer,
        StageVisualizerInput {
            diagnostic: input,
            ..StageVisualizerInput::default()
        },
        side_lengths,
    )
}

/// Render one used stage visualizer frame with full Spectrum palette context.
#[must_use]
pub fn render_stage_visualizer_with_input(
    visualizer: StageVisualizer,
    input: StageVisualizerInput,
    side_lengths: &[usize],
) -> Vec<StageCommand> {
    match visualizer {
        StageVisualizer::FlashColorsDiagnostic => {
            stage_flash_colors(input.diagnostic, side_lengths)
        }
        StageVisualizer::DepthLevel => stage_depth_level(input, side_lengths),
    }
}
