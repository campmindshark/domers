use domers_core::{ColorPalette, Rgb};
use domers_outputs::{
    dome_strut_index_for_control_box, dome_strut_length,
    topology::{DOME_STRUTS, STAGE_LAYERS},
    BarCommand, DomeCommand, StageCommand,
};

use crate::{
    color_util::{diagnostic_colors, scale_rgb_f64, white},
    input::{DiagnosticInput, StageVisualizerInput},
};

pub(crate) fn dome_set_all_commands(color: Rgb) -> Vec<DomeCommand> {
    let mut commands = Vec::new();
    for strut_index in 0..DOME_STRUTS {
        let Some(length) = dome_strut_length(strut_index) else {
            continue;
        };
        for led_index in 0..length {
            commands.push(DomeCommand::Pixel {
                strut_index,
                led_index,
                color,
            });
        }
    }
    commands.push(DomeCommand::Flush);
    commands
}

pub(crate) fn dome_set_all_control_box_commands(color: Rgb) -> Vec<DomeCommand> {
    let mut commands = Vec::new();
    for control_box in 0..5 {
        for local_index in 0..38 {
            let Some(strut_index) = dome_strut_index_for_control_box(control_box, local_index)
            else {
                continue;
            };
            let Some(strut_length) = dome_strut_length(strut_index) else {
                continue;
            };
            for led_index in 0..strut_length {
                commands.push(DomeCommand::Pixel {
                    strut_index,
                    led_index,
                    color,
                });
            }
        }
    }
    commands.push(DomeCommand::Flush);
    commands
}

pub(crate) fn dome_flash_colors_commands(input: DiagnosticInput) -> Vec<DomeCommand> {
    if input.state == 0 {
        return dome_set_all_commands(Rgb::BLACK);
    }

    let colors = diagnostic_colors(input.brightness);
    let mut commands = Vec::new();
    for control_box in 0..5 {
        let mut color_index = 0;
        for local_index in 0..38 {
            let Some(strut_index) = dome_strut_index_for_control_box(control_box, local_index)
            else {
                continue;
            };
            let Some(strut_length) = dome_strut_length(strut_index) else {
                continue;
            };
            let color = colors[color_index % 6];
            if input.state == 2 {
                for led_index in 1..strut_length.saturating_sub(1) {
                    commands.push(DomeCommand::Pixel {
                        strut_index,
                        led_index,
                        color: Rgb::BLACK,
                    });
                }
                for led_index in [0, strut_length.saturating_sub(1)] {
                    commands.push(DomeCommand::Pixel {
                        strut_index,
                        led_index,
                        color,
                    });
                }
            } else {
                for led_index in 0..strut_length {
                    commands.push(DomeCommand::Pixel {
                        strut_index,
                        led_index,
                        color,
                    });
                }
            }
            color_index = (color_index + 1) % colors.len();
        }
    }
    commands.push(DomeCommand::Flush);
    commands
}

pub(crate) fn dome_strut_iteration_commands(input: DiagnosticInput) -> Vec<DomeCommand> {
    let mut commands = Vec::new();
    let local_index = input.step % 38;
    let control_box = (input.step / 38) % 5;
    if local_index == 0 {
        for strut_index in 0..DOME_STRUTS {
            let Some(strut_length) = dome_strut_length(strut_index) else {
                continue;
            };
            for led_index in 0..strut_length {
                commands.push(DomeCommand::Pixel {
                    strut_index,
                    led_index,
                    color: Rgb::from_u24(0x00_00_ff),
                });
            }
        }
    }
    let color_cycle = [
        Rgb::from_u24(0xff_00_00),
        Rgb::from_u24(0x00_ff_00),
        Rgb::from_u24(0x00_00_ff),
        Rgb::from_u24(0xff_ff_ff),
    ];
    let color =
        color_cycle[((input.step / (38 * 5)) + 1) % color_cycle.len()].scale(input.brightness);
    if let Some(strut_index) = dome_strut_index_for_control_box(control_box, local_index) {
        if let Some(strut_length) = dome_strut_length(strut_index) {
            for led_index in 0..strut_length {
                commands.push(DomeCommand::Pixel {
                    strut_index,
                    led_index,
                    color,
                });
            }
        }
    }
    commands.push(DomeCommand::Flush);
    commands
}

pub(crate) fn dome_strand_test_commands(input: DiagnosticInput) -> Vec<DomeCommand> {
    if input.state == 0 {
        dome_set_all_commands(Rgb::BLACK)
    } else {
        dome_set_all_control_box_commands(white(input.brightness))
    }
}

pub(crate) fn dome_full_color_flash_commands(input: DiagnosticInput) -> Vec<DomeCommand> {
    if input.state == 0 {
        dome_set_all_commands(Rgb::BLACK)
    } else {
        dome_set_all_control_box_commands(white(input.brightness))
    }
}

pub(crate) fn bar_flash_colors(
    input: DiagnosticInput,
    infinity_width: usize,
    infinity_length: usize,
    runner_length: usize,
) -> Vec<BarCommand> {
    let mut commands = Vec::new();
    let colors = diagnostic_colors(input.brightness);
    let infinity_pixels = 2 * infinity_length + 2 * infinity_width;

    for index in 0..infinity_pixels {
        let color = if input.state == 0
            || (input.state == 2 && !is_bar_border(index, infinity_width, infinity_length))
        {
            Rgb::BLACK
        } else {
            colors[bar_segment(index, infinity_width, infinity_length)]
        };
        commands.push(BarCommand::Pixel {
            is_runner: false,
            led_index: index,
            color,
        });
    }

    for index in 0..runner_length {
        let color =
            if input.state == 0 || (input.state == 2 && index != 0 && index + 1 != runner_length) {
                Rgb::BLACK
            } else {
                colors[4]
            };
        commands.push(BarCommand::Pixel {
            is_runner: true,
            led_index: index,
            color,
        });
    }

    commands.push(BarCommand::Flush);
    commands
}

pub(crate) fn stage_flash_colors(
    input: DiagnosticInput,
    side_lengths: &[usize],
) -> Vec<StageCommand> {
    let mut commands = Vec::new();
    let colors = diagnostic_colors(input.brightness);
    if input.state == 0 {
        for (side_index, side_length) in side_lengths.iter().copied().enumerate() {
            for led_index in 0..side_length {
                for layer_index in 0..STAGE_LAYERS {
                    commands.push(StageCommand::Pixel {
                        side_index,
                        led_index,
                        layer_index,
                        color: Rgb::BLACK,
                    });
                }
            }
        }
        commands.push(StageCommand::Flush);
        return commands;
    }

    let mut color_index = 0;
    for (side_index, side_length) in side_lengths.iter().copied().enumerate() {
        for layer_index in 0..STAGE_LAYERS {
            for led_index in 0..side_length {
                let color = if input.state == 2 && led_index != 0 && led_index + 1 != side_length {
                    Rgb::BLACK
                } else {
                    colors[color_index % colors.len()]
                };
                commands.push(StageCommand::Pixel {
                    side_index,
                    led_index,
                    layer_index,
                    color,
                });
            }
            color_index += 1;
        }
    }
    commands.push(StageCommand::Flush);
    commands
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "Stage preview converts small fixture indices into normalized animation positions"
)]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Stage visualizer input is assembled per frame and kept by value with other renderer inputs"
)]
pub(crate) fn stage_depth_level(
    input: StageVisualizerInput,
    side_lengths: &[usize],
) -> Vec<StageCommand> {
    let mut commands = Vec::new();
    let diagnostic = input.diagnostic;
    let triangles = side_lengths.len() / 3;
    for triangle_index in 0..triangles {
        let tracer_index =
            stage_tracer_led_index(side_lengths, triangle_index, diagnostic.beat_progress);
        let max_triangle_counter = triangle_length(side_lengths, triangle_index);
        let mut triangle_counter = 0;
        for side_offset in 0..3 {
            let side_index = triangle_index * 3 + side_offset;
            let side_length = side_lengths[side_index];
            for led_index in 0..side_length {
                let second_part = stage_second_part(side_index) ^ (diagnostic.beat_progress > 0.5);
                let color = stage_gradient_color(
                    &input.color_palette,
                    input.color_palette_index,
                    usize::from(second_part),
                    triangle_counter,
                    max_triangle_counter,
                    tracer_index,
                    input.stage_brightness,
                    diagnostic.volume,
                );
                for layer_index in 0..STAGE_LAYERS {
                    commands.push(StageCommand::Pixel {
                        side_index,
                        led_index,
                        layer_index,
                        color,
                    });
                }
                triangle_counter += 1;
            }
        }
    }
    commands.push(StageCommand::Flush);
    commands
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "Stage tracer math mirrors Spectrum's double-to-int LED index calculation"
)]
pub(crate) fn stage_tracer_led_index(
    side_lengths: &[usize],
    triangle_index: usize,
    beat_progress: f64,
) -> usize {
    let progress = (beat_progress.fract() * 3.0).clamp(0.0, 2.999_999);
    let base = triangle_index * 3;
    if progress < 1.0 {
        (progress * side_lengths[base] as f64) as usize
    } else if progress < 2.0 {
        side_lengths[base] + ((progress - 1.0) * side_lengths[base + 1] as f64) as usize
    } else {
        side_lengths[base]
            + side_lengths[base + 1]
            + ((progress - 2.0) * side_lengths[base + 2] as f64) as usize
    }
}

pub(crate) fn triangle_length(side_lengths: &[usize], triangle_index: usize) -> usize {
    side_lengths[triangle_index * 3..triangle_index * 3 + 3]
        .iter()
        .sum()
}

pub(crate) fn stage_second_part(side_index: usize) -> bool {
    if (side_index / 12) == 1 {
        ((side_index / 3) % 4) == 2
    } else {
        ((side_index / 3) % 4) == 1
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "Visualizer color focus uses normalized LED counters before RGB scaling"
)]
#[allow(
    clippy::too_many_arguments,
    reason = "Mirrors Spectrum's gradient calculation inputs without hiding state in a temporary struct"
)]
pub(crate) fn stage_gradient_color(
    palette: &ColorPalette,
    palette_index: u8,
    relative_color_index: usize,
    triangle_counter: usize,
    max_triangle_counter: usize,
    tracer_index: usize,
    stage_brightness: f64,
    volume: f32,
) -> Rgb {
    let pixel_pos = triangle_counter as f64 / max_triangle_counter.max(1) as f64;
    let focus_pos = tracer_index as f64 / max_triangle_counter.max(1) as f64;
    let color = palette.gradient_color(
        relative_color_index,
        palette_index,
        pixel_pos,
        focus_pos,
        true,
    );
    scale_rgb_f64(scale_rgb_f64(color, stage_brightness), f64::from(volume))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Spectrum LEDColor.ScaleColor truncates scaled double channels to integers"
)]
pub(crate) fn is_bar_border(index: usize, infinity_width: usize, infinity_length: usize) -> bool {
    let second_length_start = infinity_width + infinity_length;
    let second_width_start = infinity_width + 2 * infinity_length;
    index == 0
        || index + 1 == infinity_length
        || index == infinity_length
        || index + 1 == infinity_length + infinity_width
        || index == second_length_start
        || index + 1 == second_length_start + infinity_length
        || index == second_width_start
        || index + 1 == second_width_start + infinity_width
}

pub(crate) fn bar_segment(index: usize, infinity_width: usize, infinity_length: usize) -> usize {
    if index < infinity_length {
        0
    } else if index < infinity_length + infinity_width {
        1
    } else if index < infinity_width + 2 * infinity_length {
        2
    } else {
        3
    }
}
