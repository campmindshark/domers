use domers_core::Rgb;
use domers_outputs::{
    dome_strut_length,
    topology::{DOME_PIXELS, DOME_STRUTS},
    DomeCommand,
};

use crate::input::VisualizerInput;

pub(crate) const VOLUME_ANIMATION_SIZE: usize = 4;
/// Spectrum `domeGradientSpeed` default from `spectrum_default_config.xml`.
pub const VOLUME_GRADIENT_SPEED: f64 = 0.25;
/// Spectrum `domeVolumeRotationSpeed` default from `spectrum_default_config.xml`.
pub const VOLUME_ROTATION_SPEED: f64 = 0.25;
pub(crate) const VOLUME_STARTING_POINTS: [usize; 6] = [22, 26, 30, 34, 38, 70];

#[allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    reason = "Volume port mirrors Spectrum's small integer layout ratios and exact filled-section checks"
)]
pub(crate) fn volume_commands(input: VisualizerInput) -> Vec<DomeCommand> {
    let beat_progress = if input.animation_frame == 0 {
        0.0
    } else {
        input.beat_progress
    };
    volume_commands_with_wipe(input, beat_progress, input.animation_frame == 0)
}

pub(crate) fn volume_rotation_progress(input: &VisualizerInput, beat_progress: f64) -> f64 {
    input
        .beat_progress_rotation
        .unwrap_or_else(|| progress_through_beat(beat_progress, VOLUME_ROTATION_SPEED))
}

pub(crate) fn volume_gradient_progress(input: &VisualizerInput, beat_progress: f64) -> f64 {
    input
        .beat_progress_gradient
        .unwrap_or_else(|| progress_through_beat(beat_progress, VOLUME_GRADIENT_SPEED))
}

pub(crate) fn volume_center_offset_for_input(input: &VisualizerInput, beat_progress: f64) -> usize {
    volume_center_offset_from_progress(volume_rotation_progress(input, beat_progress))
}

pub(crate) fn volume_center_offset_from_progress(rotation_progress: f64) -> usize {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Spectrum truncates ProgressThroughBeat times four to choose the volume center"
    )]
    let center = (rotation_progress * 4.0) as usize;
    center
}

#[allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    reason = "Volume port mirrors Spectrum's small integer layout ratios and exact filled-section checks"
)]
pub(crate) fn volume_commands_with_wipe(
    input: VisualizerInput,
    beat_progress: f64,
    include_wipe: bool,
) -> Vec<DomeCommand> {
    let layouts = volume_layouts(volume_center_offset_for_input(&input, beat_progress));
    let total_parts = VOLUME_ANIMATION_SIZE;
    let volume_split_into = 2 * ((total_parts - 1) / 2 + 1);
    let level = f64::from(input.volume.clamp(0.0, 1.0));
    let gradient_focus = volume_gradient_progress(&input, beat_progress);
    let mut commands = if include_wipe {
        volume_wipe_commands()
    } else {
        Vec::new()
    };

    for part in (0..total_parts).step_by(2) {
        let start_range = part as f64 / volume_split_into as f64;
        let end_range = (part + 2) as f64 / volume_split_into as f64;
        let scaled = if end_range == start_range {
            0.0
        } else {
            ((level - start_range) / (end_range - start_range)).clamp(0.0, 1.0)
        };
        let start_lit_range = if level == 0.0 {
            1.0
        } else {
            (start_range / level).min(1.0)
        };
        let end_lit_range = if level == 0.0 {
            1.0
        } else {
            (end_range / level).min(1.0)
        };

        for strut in &layouts.part.segments[part].struts {
            update_volume_strut(
                &mut commands,
                &layouts.part,
                input,
                *strut,
                scaled,
                start_lit_range,
                end_lit_range,
                gradient_focus,
            );
        }

        if part + 1 == total_parts {
            break;
        }

        for section_index in 0..6 {
            let segment = &layouts.section.segments[section_index + part * 3];
            let gradient_step = 1.0 / segment.struts.len() as f64;
            let mut gradient_start_pos = 0.0;
            for strut in &segment.struts {
                let gradient_end_pos = gradient_start_pos + gradient_step;
                update_volume_strut(
                    &mut commands,
                    &layouts.part,
                    input,
                    *strut,
                    if scaled == 1.0 { 1.0 } else { 0.0 },
                    gradient_start_pos,
                    gradient_end_pos,
                    gradient_focus,
                );
                gradient_start_pos = gradient_end_pos;
            }
        }
    }

    commands.push(DomeCommand::Flush);
    commands
}

pub(crate) fn volume_wipe_commands() -> Vec<DomeCommand> {
    let mut commands = Vec::with_capacity(DOME_PIXELS);
    for strut_index in 0..DOME_STRUTS {
        let Some(length) = dome_strut_length(strut_index) else {
            continue;
        };
        for led_index in 0..length {
            commands.push(DomeCommand::Pixel {
                strut_index,
                led_index,
                color: Rgb::BLACK,
            });
        }
    }
    commands
}

#[allow(
    clippy::too_many_arguments,
    reason = "Mirrors Spectrum LEDDomeVolumeVisualizer.UpdateStrut without hiding the layout inputs"
)]
pub(crate) fn update_volume_strut(
    commands: &mut Vec<DomeCommand>,
    part_layout: &VolumeStrutLayout,
    input: VisualizerInput,
    strut: VolumeStrut,
    percentage_lit: f64,
    start_lit_range: f64,
    end_lit_range: f64,
    gradient_focus: f64,
) {
    let Some(length) = dome_strut_length(strut.index) else {
        return;
    };
    for led_index in 0..length {
        let color = volume_gradient_pos(
            strut,
            length,
            percentage_lit,
            start_lit_range,
            end_lit_range,
            led_index,
        )
        .map_or(Rgb::BLACK, |gradient_pos| {
            volume_color_from_part(
                part_layout,
                input,
                strut.index,
                gradient_pos,
                gradient_focus,
            )
        });
        commands.push(DomeCommand::Pixel {
            strut_index: strut.index,
            led_index,
            color,
        });
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Volume strut lengths and LED indexes are small Spectrum topology constants"
)]
pub(crate) fn volume_gradient_pos(
    strut: VolumeStrut,
    length: usize,
    percentage_lit: f64,
    start_lit_range: f64,
    end_lit_range: f64,
    led_index: usize,
) -> Option<f64> {
    if percentage_lit == 0.0 {
        return None;
    }
    let led = if strut.reversed {
        length.saturating_sub(led_index)
    } else {
        led_index
    };
    let step = (end_lit_range - start_lit_range) / (length as f64 * percentage_lit);
    let gradient_pos = start_lit_range + led as f64 * step;
    (gradient_pos <= 1.0).then_some(gradient_pos)
}

pub(crate) fn volume_color_from_part(
    part_layout: &VolumeStrutLayout,
    input: VisualizerInput,
    strut_index: usize,
    pixel_pos: f64,
    gradient_focus: f64,
) -> Rgb {
    let color_index = match part_layout.segment_index_of_strut(strut_index) {
        Some(0) => 1,
        Some(1) => 2,
        Some(2) => 3,
        _ => 0,
    };
    input.palette_entries[color_index].gradient_color(pixel_pos, gradient_focus, true)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Spectrum truncates ProgressThroughBeat times four to choose the volume center"
)]
#[allow(
    dead_code,
    reason = "Golden tests use progress_through_beat on injected beat_progress"
)]
pub(crate) fn volume_center_offset(beat_progress: f64) -> usize {
    volume_center_offset_from_progress(progress_through_beat(beat_progress, VOLUME_ROTATION_SPEED))
}

/// Mirror `BeatBroadcaster.ProgressThroughBeat` for injected measure progress.
pub(crate) fn progress_through_beat(beat_progress: f64, factor: f64) -> f64 {
    if factor == 0.0 {
        return 0.0;
    }
    let beat_length = 1.0 / factor;
    let progress_in_beat = beat_progress % beat_length;
    progress_in_beat / beat_length
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VolumeStrut {
    pub(crate) index: usize,
    pub(crate) reversed: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct VolumeStrutLayoutSegment {
    pub(crate) struts: Vec<VolumeStrut>,
}

#[derive(Clone, Debug)]
pub(crate) struct VolumeStrutLayout {
    pub(crate) segments: Vec<VolumeStrutLayoutSegment>,
    strut_to_segment: [Option<usize>; DOME_STRUTS],
}

impl VolumeStrutLayout {
    pub(crate) fn new(segments: Vec<VolumeStrutLayoutSegment>) -> Self {
        let mut strut_to_segment = [None; DOME_STRUTS];
        for (segment_index, segment) in segments.iter().enumerate() {
            for strut in &segment.struts {
                strut_to_segment[strut.index] = Some(segment_index);
            }
        }
        Self {
            segments,
            strut_to_segment,
        }
    }

    pub(crate) fn segment_index_of_strut(&self, strut_index: usize) -> Option<usize> {
        self.strut_to_segment.get(strut_index).copied().flatten()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct VolumeLayouts {
    part: VolumeStrutLayout,
    section: VolumeStrutLayout,
}

pub(crate) fn volume_layouts(center_offset: usize) -> VolumeLayouts {
    let mut points = VOLUME_STARTING_POINTS;
    for point in points.iter_mut().take(5) {
        *point += center_offset;
    }
    if points[4] >= 40 {
        points[4] -= 20;
    }

    let edge_dictionary = volume_edge_dictionary();
    let mut cur_points_by_group: Vec<Vec<usize>> =
        points.iter().copied().map(|point| vec![point]).collect();
    let mut spoke_segments = Vec::new();
    let mut struts_by_group: [Vec<VolumeStrut>; 6] = std::array::from_fn(|_| Vec::new());
    let mut circle_segments = Vec::new();
    let mut used_struts = [false; DOME_STRUTS];
    let mut layers_left = VOLUME_ANIMATION_SIZE;

    while layers_left > 0 {
        let mut layer1 = Vec::new();
        let mut next_points_by_group = Vec::new();
        for (group_index, group) in cur_points_by_group.iter().enumerate() {
            let mut new_points = Vec::new();
            for &point in group {
                for edge in &edge_dictionary[point] {
                    if used_struts[edge.strut.index] {
                        continue;
                    }
                    used_struts[edge.strut.index] = true;
                    push_unique_strut(&mut layer1, edge.strut);
                    push_unique_strut(&mut struts_by_group[group_index], edge.strut);
                    push_unique_usize(&mut new_points, edge.connected_point);
                }
            }
            next_points_by_group.push(new_points);
        }
        spoke_segments.push(VolumeStrutLayoutSegment { struts: layer1 });
        layers_left -= 1;
        if layers_left == 0 {
            break;
        }

        cur_points_by_group = next_points_by_group;
        let mut layer2 = Vec::new();
        for (group_index, group) in cur_points_by_group.iter().enumerate() {
            let Some(mut current_point) = group.first().copied() else {
                circle_segments.push(VolumeStrutLayoutSegment { struts: Vec::new() });
                continue;
            };
            for &point in group {
                let connected_count = edge_dictionary[point]
                    .iter()
                    .filter(|edge| group.contains(&edge.connected_point))
                    .count();
                if connected_count == 1 {
                    current_point = point;
                    break;
                }
            }

            let mut points_left = group.clone();
            let mut circle_struts = Vec::new();
            loop {
                let mut next_point_in_loop = None;
                for edge in &edge_dictionary[current_point] {
                    if !group.contains(&edge.connected_point) || used_struts[edge.strut.index] {
                        continue;
                    }
                    used_struts[edge.strut.index] = true;
                    push_unique_strut(&mut layer2, edge.strut);
                    push_unique_strut(&mut circle_struts, edge.strut);
                    push_unique_strut(&mut struts_by_group[group_index], edge.strut);
                    if points_left.contains(&edge.connected_point) {
                        next_point_in_loop = Some(edge.connected_point);
                    }
                    break;
                }
                points_left.retain(|point| *point != current_point);
                if let Some(next_point) = next_point_in_loop {
                    current_point = next_point;
                } else {
                    break;
                }
            }
            circle_segments.push(VolumeStrutLayoutSegment {
                struts: circle_struts,
            });
        }
        spoke_segments.push(VolumeStrutLayoutSegment { struts: layer2 });
        layers_left -= 1;
    }

    VolumeLayouts {
        part: VolumeStrutLayout::new(spoke_segments),
        section: VolumeStrutLayout::new(circle_segments),
    }
}

pub(crate) fn concentric_layout_from_point(
    starting_point: usize,
    num_layers: usize,
) -> VolumeStrutLayout {
    concentric_layout_from_starting_points(&[starting_point], num_layers)
}

pub(crate) fn concentric_layout_from_starting_points(
    starting_points: &[usize],
    num_layers: usize,
) -> VolumeStrutLayout {
    let edge_dictionary = volume_edge_dictionary();
    let mut cur_points_by_group: Vec<Vec<usize>> = starting_points
        .iter()
        .copied()
        .map(|point| vec![point])
        .collect();
    let mut segments = Vec::new();
    let mut used_struts = [false; DOME_STRUTS];
    let mut layers_left = num_layers;

    while layers_left > 0 {
        let mut layer1 = Vec::new();
        let mut next_points_by_group = Vec::new();
        for group in &cur_points_by_group {
            let mut new_points = Vec::new();
            for &point in group {
                for edge in &edge_dictionary[point] {
                    if used_struts[edge.strut.index] {
                        continue;
                    }
                    used_struts[edge.strut.index] = true;
                    push_unique_strut(&mut layer1, edge.strut);
                    push_unique_usize(&mut new_points, edge.connected_point);
                }
            }
            next_points_by_group.push(new_points);
        }
        segments.push(VolumeStrutLayoutSegment { struts: layer1 });
        layers_left -= 1;
        if layers_left == 0 {
            break;
        }

        cur_points_by_group = next_points_by_group;
        let mut layer2 = Vec::new();
        for group in &cur_points_by_group {
            let Some(mut current_point) = group.first().copied() else {
                continue;
            };
            for &point in group {
                let connected_count = edge_dictionary[point]
                    .iter()
                    .filter(|edge| group.contains(&edge.connected_point))
                    .count();
                if connected_count == 1 {
                    current_point = point;
                    break;
                }
            }

            let mut points_left = group.clone();
            loop {
                let mut next_point_in_loop = None;
                for edge in &edge_dictionary[current_point] {
                    if !group.contains(&edge.connected_point) || used_struts[edge.strut.index] {
                        continue;
                    }
                    used_struts[edge.strut.index] = true;
                    push_unique_strut(&mut layer2, edge.strut);
                    if points_left.contains(&edge.connected_point) {
                        next_point_in_loop = Some(edge.connected_point);
                    }
                    break;
                }
                points_left.retain(|point| *point != current_point);
                if let Some(next_point) = next_point_in_loop {
                    current_point = next_point;
                } else {
                    break;
                }
            }
        }
        segments.push(VolumeStrutLayoutSegment { struts: layer2 });
        layers_left -= 1;
    }

    VolumeStrutLayout::new(segments)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VolumeEdge {
    pub(crate) connected_point: usize,
    pub(crate) strut: VolumeStrut,
}

pub(crate) fn volume_edge_dictionary() -> Vec<Vec<VolumeEdge>> {
    let mut edges = vec![Vec::new(); 71];
    for (strut_index, [point0, point1]) in VOLUME_LINES.iter().copied().enumerate() {
        edges[point0].push(VolumeEdge {
            connected_point: point1,
            strut: VolumeStrut {
                index: strut_index,
                reversed: false,
            },
        });
        edges[point1].push(VolumeEdge {
            connected_point: point0,
            strut: VolumeStrut {
                index: strut_index,
                reversed: true,
            },
        });
    }
    edges
}

pub(crate) fn push_unique_usize(values: &mut Vec<usize>, value: usize) {
    if !values.contains(&value) {
        values.push(value);
    }
}

pub(crate) fn push_unique_strut(values: &mut Vec<VolumeStrut>, value: VolumeStrut) {
    if !values.iter().any(|strut| strut.index == value.index) {
        values.push(value);
    }
}

pub(crate) const VOLUME_LINES: [[usize; 2]; DOME_STRUTS] = [
    [0, 1],
    [1, 2],
    [3, 2],
    [3, 4],
    [4, 5],
    [5, 6],
    [7, 6],
    [7, 8],
    [8, 9],
    [9, 10],
    [11, 10],
    [11, 12],
    [12, 13],
    [13, 14],
    [15, 14],
    [15, 16],
    [16, 17],
    [17, 18],
    [19, 18],
    [19, 0],
    [20, 21],
    [22, 21],
    [23, 22],
    [24, 23],
    [24, 25],
    [26, 25],
    [27, 26],
    [28, 27],
    [28, 29],
    [30, 29],
    [31, 30],
    [32, 31],
    [32, 33],
    [34, 33],
    [35, 34],
    [36, 35],
    [36, 37],
    [38, 37],
    [39, 38],
    [20, 39],
    [41, 40],
    [42, 41],
    [43, 42],
    [44, 43],
    [45, 44],
    [46, 45],
    [47, 46],
    [48, 47],
    [49, 48],
    [50, 49],
    [51, 50],
    [52, 51],
    [53, 52],
    [54, 53],
    [40, 54],
    [56, 55],
    [57, 56],
    [58, 57],
    [59, 58],
    [60, 59],
    [61, 60],
    [62, 61],
    [63, 62],
    [64, 63],
    [55, 64],
    [65, 66],
    [66, 67],
    [67, 68],
    [68, 69],
    [69, 65],
    [20, 0],
    [0, 21],
    [21, 1],
    [1, 22],
    [2, 22],
    [23, 2],
    [23, 3],
    [24, 3],
    [24, 4],
    [4, 25],
    [25, 5],
    [5, 26],
    [6, 26],
    [27, 6],
    [27, 7],
    [28, 7],
    [28, 8],
    [8, 29],
    [29, 9],
    [9, 30],
    [10, 30],
    [31, 10],
    [31, 11],
    [32, 11],
    [32, 12],
    [12, 33],
    [33, 13],
    [13, 34],
    [14, 34],
    [35, 14],
    [35, 15],
    [36, 15],
    [36, 16],
    [16, 37],
    [37, 17],
    [17, 38],
    [18, 38],
    [39, 18],
    [39, 19],
    [20, 19],
    [20, 40],
    [21, 40],
    [21, 41],
    [22, 41],
    [41, 23],
    [42, 23],
    [24, 42],
    [24, 43],
    [25, 43],
    [25, 44],
    [26, 44],
    [44, 27],
    [45, 27],
    [28, 45],
    [28, 46],
    [29, 46],
    [29, 47],
    [30, 47],
    [47, 31],
    [48, 31],
    [32, 48],
    [32, 49],
    [33, 49],
    [33, 50],
    [34, 50],
    [50, 35],
    [51, 35],
    [36, 51],
    [36, 52],
    [37, 52],
    [37, 53],
    [38, 53],
    [53, 39],
    [54, 39],
    [20, 54],
    [40, 55],
    [40, 56],
    [41, 56],
    [56, 42],
    [42, 57],
    [43, 57],
    [43, 58],
    [44, 58],
    [58, 45],
    [45, 59],
    [46, 59],
    [46, 60],
    [47, 60],
    [60, 48],
    [48, 61],
    [49, 61],
    [49, 62],
    [50, 62],
    [62, 51],
    [51, 63],
    [52, 63],
    [52, 64],
    [53, 64],
    [64, 54],
    [54, 55],
    [55, 65],
    [56, 65],
    [57, 65],
    [57, 66],
    [58, 66],
    [59, 66],
    [59, 67],
    [60, 67],
    [61, 67],
    [61, 68],
    [62, 68],
    [63, 68],
    [63, 69],
    [64, 69],
    [55, 69],
    [65, 70],
    [66, 70],
    [67, 70],
    [68, 70],
    [69, 70],
];
