#!/usr/bin/env python3
"""Apply manual fixes after split_visualizers_v2.py."""

from __future__ import annotations

from pathlib import Path

SRC = Path(__file__).resolve().parents[1] / "crates" / "visualizers" / "src"


def write(path: str, content: str) -> None:
    (SRC / path).write_text(content)


def patch(path: str, old: str, new: str) -> None:
    p = SRC / path
    text = p.read_text()
    if old not in text:
        return
    p.write_text(text.replace(old, new, 1))


def main() -> None:
    # input.rs — Default impl visibility + Quaternion import
    patch("input.rs", "    pub(crate) fn default() -> Self {", "    fn default() -> Self {")
    if "use crate::quaternion::Quaternion;" not in (SRC / "input.rs").read_text():
        patch(
            "input.rs",
            "use domers_core::{ColorPalette, PaletteEntry, Rgb};\n",
            "use domers_core::{ColorPalette, PaletteEntry, Rgb};\nuse serde::Deserialize;\n\n"
            "use crate::quaternion::Quaternion;\n\n",
        )

    # rng.rs
    patch(
        "rng.rs",
        "pub(crate) struct DotNetRandom {",
        "#[derive(Clone, Debug)]\npub(crate) struct DotNetRandom {",
    )
    patch(
        "rng.rs",
        "    pub(crate) const MBIG: i32 = 2_147_483_647;\n    pub(crate) const MSEED: i32 = 161_803_398;",
        "    const MBIG: i32 = 2_147_483_647;\n    const MSEED: i32 = 161_803_398;",
    )

    # geometry / buffer / quaternion field visibility
    patch(
        "geometry.rs",
        "pub(crate) struct DomeLedPoint {\n    index: usize,\n    x: f64,\n    y: f64,\n}",
        "pub(crate) struct DomeLedPoint {\n    pub(crate) index: usize,\n    pub(crate) x: f64,\n    pub(crate) y: f64,\n}",
    )
    patch(
        "buffer.rs",
        "pub(crate) struct DomeBufferPixel {\n    x: f64,\n    y: f64,",
        "#[derive(Clone, Copy, Debug)]\npub(crate) struct DomeBufferPixel {\n    pub(crate) x: f64,\n    pub(crate) y: f64,",
    )
    patch(
        "buffer.rs",
        "pub(crate) struct DomeBuffer {\n    pixels: Vec<DomeBufferPixel>,",
        "pub(crate) struct DomeBuffer {\n    pub(crate) pixels: Vec<DomeBufferPixel>,",
    )
    patch(
        "buffer.rs",
        "use crate::{\n    color_util::scale_rgb_f64,\n    geometry::{build_dome_led_points, DOME_LED_POINTS},\n};",
        "use crate::{\n    color_util::{light_paint, scale_rgb_f64},\n    geometry::{build_dome_led_points, DOME_LED_POINTS},\n};\n"
        "use domers_outputs::{topology::DOME_PIXELS, DomeCommand};",
    )
    patch(
        "quaternion.rs",
        "pub struct Quaternion {\n    x: f64,\n    y: f64,\n    z: f64,\n    w: f64,\n}",
        "pub struct Quaternion {\n    pub(crate) x: f64,\n    pub(crate) y: f64,\n    pub(crate) z: f64,\n    pub(crate) w: f64,\n}",
    )

    # volume.rs fixes
    patch(
        "dome/volume.rs",
        "use domers_outputs::{dome_strut_length, topology::DOME_STRUTS, DomeCommand};\n\nuse crate::{color_util::scale_rgb_f64, input::VisualizerInput};",
        "use domers_outputs::{dome_strut_length, topology::{DOME_PIXELS, DOME_STRUTS}, DomeCommand};\n\n"
        "use crate::{color_util::scale_rgb_f64, dome::VOLUME_ROTATION_SPEED, input::VisualizerInput};",
    )
    patch(
        "dome/volume.rs",
        "pub(crate) pub(crate) struct VolumeStrutLayout {",
        "pub(crate) struct VolumeStrutLayout {",
    )
    patch(
        "dome/volume.rs",
        "pub(crate) struct VolumeStrut {\n    index: usize,\n    reversed: bool,\n}",
        "pub(crate) struct VolumeStrut {\n    pub(crate) index: usize,\n    pub(crate) reversed: bool,\n}",
    )
    patch(
        "dome/volume.rs",
        "pub(crate) struct VolumeStrutLayoutSegment {\n    struts: Vec<VolumeStrut>,\n}",
        "pub(crate) struct VolumeStrutLayoutSegment {\n    pub(crate) struts: Vec<VolumeStrut>,\n}",
    )
    if "struct VolumeEdge" not in (SRC / "dome/volume.rs").read_text():
        patch(
            "dome/volume.rs",
            "pub(crate) fn volume_edge_dictionary() -> Vec<Vec<VolumeEdge>> {",
            "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n"
            "pub(crate) struct VolumeEdge {\n"
            "    pub(crate) connected_point: usize,\n"
            "    pub(crate) strut: VolumeStrut,\n"
            "}\n\n"
            "pub(crate) fn volume_edge_dictionary() -> Vec<Vec<VolumeEdge>> {",
        )

    # flash.rs — move types from runtime, remove stray VolumeEdge
    write(
        "runtime/flash.rs",
        (SRC / "runtime/flash.rs").read_text(),  # placeholder
    )

    # dome/mod.rs
    write(
        "dome/mod.rs",
        """mod flash;
mod paintbrush;
mod quaternion;
mod race;
mod radial;
mod snakes;
mod splat;
mod tv_static;
mod volume;

pub(crate) use flash::{
    animate_flash_polygon, clear_flash_strut, flash_layout_struts, flash_pad_gradient_color,
    flash_pad_single_color, FlashPolygonAnimation, FlashShape,
};
pub(crate) use paintbrush::quaternion_paintbrush_frame;
pub(crate) use quaternion::{quaternion_multi_test_frame, quaternion_test_frame};
pub(crate) use race::{
    race_commands, race_pixel_color, RaceRacer, RACE_RACER_CONFIGS, VOLUME_ROTATION_SPEED,
};
pub(crate) use radial::radial_frame;
pub(crate) use snakes::{
    snake_triangles, snakes_commands, SnakesState, SNAKE_TRIANGLE_DEFS, SNAKES_MAX_CATCHUP_STEPS,
    SNAKES_STEP_FRAMES, SNAKES_STEP_MS,
};
pub(crate) use splat::splat_frame;
pub(crate) use tv_static::tv_static_commands;
pub(crate) use volume::{
    concentric_layout_from_point, volume_center_offset, volume_commands, volume_commands_with_wipe,
    volume_layouts, volume_wipe_commands, VolumeStrutLayout,
};
pub(crate) use volume::VOLUME_GRADIENT_SPEED;
""",
    )

    # snakes TriangleSeg fields
    patch(
        "dome/snakes.rs",
        """pub(crate) struct TriangleSeg {
    struts: [usize; 3],
    points_up: bool,
    left: Option<usize>,
    above: Option<usize>,
    right: Option<usize>,
    below: Option<usize>,
}""",
        """pub(crate) struct TriangleSeg {
    pub(crate) struts: [usize; 3],
    pub(crate) points_up: bool,
    pub(crate) left: Option<usize>,
    pub(crate) above: Option<usize>,
    pub(crate) right: Option<usize>,
    pub(crate) below: Option<usize>,
}""",
    )

    # race.rs
    patch(
        "dome/race.rs",
        "use crate::{\n    geometry::{build_dome_led_points, DomeLedPoint, DOME_LED_POINTS},\n    input::VisualizerInput,\n};",
        "use crate::{\n    color_util::scale_rgb_f64,\n    geometry::{build_dome_led_points, DomeLedPoint, DOME_LED_POINTS},\n    input::VisualizerInput,\n};",
    )
    patch(
        "dome/race.rs",
        "pub(crate) struct RaceRacer {\n    angle: f64,",
        "#[derive(Clone, Debug)]\npub(crate) struct RaceRacer {\n    pub(crate) angle: f64,",
    )
    patch(
        "runtime/race.rs",
        "use crate::{\n    dome::race::{race_pixel_color, RaceRacer, RACE_RACER_CONFIGS},",
        "use crate::{\n    dome::{race_pixel_color, RaceRacer, RACE_RACER_CONFIGS},",
    )
    patch(
        "runtime/race.rs",
        "pub(crate) struct RaceRuntime {",
        "#[derive(Clone, Debug)]\npub(crate) struct RaceRuntime {",
    )

    # radial runtime + dome radial imports
    patch(
        "runtime/radial.rs",
        "pub(crate) struct RadialRuntime {",
        "#[derive(Clone, Debug)]\npub(crate) struct RadialRuntime {",
    )
    patch(
        "runtime/radial.rs",
        "use crate::{\n    buffer::DomeBuffer,\n    geometry::{build_dome_led_points, DOME_LED_POINTS},\n    input::VisualizerInput,\n    math::{map_value, map_wrap, polar_to_cartesian, radial_effect, wrap, DOME_RADIAL_CENTER_ANGLE, DOME_RADIAL_CENTER_DISTANCE, DOME_RADIAL_CENTER_SPEED, DOME_RADIAL_EFFECT, DOME_RADIAL_FREQUENCY, DOME_RADIAL_SIZE, DOME_GLOBAL_FADE_SPEED, DOME_GLOBAL_HUE_SPEED},\n};",
        "use crate::{\n    buffer::DomeBuffer,\n    dome::{VOLUME_GRADIENT_SPEED, VOLUME_ROTATION_SPEED},\n    geometry::{build_dome_led_points, DOME_LED_POINTS},\n    input::VisualizerInput,\n    math::{map_value, map_wrap, polar_to_cartesian, radial_effect, wrap, DOME_RADIAL_CENTER_ANGLE, DOME_RADIAL_CENTER_DISTANCE, DOME_RADIAL_CENTER_SPEED, DOME_RADIAL_EFFECT, DOME_RADIAL_FREQUENCY, DOME_RADIAL_SIZE, DOME_GLOBAL_FADE_SPEED, DOME_GLOBAL_HUE_SPEED},\n};",
    )
    patch(
        "dome/radial.rs",
        "use crate::{\n    dome::race::VOLUME_ROTATION_SPEED,\n    dome::volume::VOLUME_GRADIENT_SPEED,",
        "use crate::{\n    dome::{VOLUME_GRADIENT_SPEED, VOLUME_ROTATION_SPEED},",
    )

    # splat, paintbrush runtime, diagnostics
    patch(
        "runtime/splat.rs",
        "    math::runtime_visualizer_progress_unwrapped,\n    rng::DotNetRandom,",
        "    math::{map_value, runtime_visualizer_progress_unwrapped, SPLAT_FADE},\n    rng::DotNetRandom,",
    )
    patch(
        "runtime/paintbrush.rs",
        "    math::spectrum_nudge,",
        "    math::{spectrum_nudge, DOME_GLOBAL_FADE_SPEED, DOME_GLOBAL_HUE_SPEED, DOME_RADIAL_SIZE},",
    )
    patch("diagnostics.rs", "use domers_core::Rgb;", "use domers_core::{ColorPalette, Rgb};")

    # dome/quaternion, splat imports
    patch(
        "dome/quaternion.rs",
        "use crate::{\n    geometry::{build_dome_led_points, DOME_LED_POINTS},",
        "use crate::{\n    color_util::hsv_to_rgb,\n    geometry::{build_dome_led_points, distance3, DOME_LED_POINTS},",
    )
    patch(
        "dome/splat.rs",
        "use crate::{\n    geometry::{build_dome_led_points, distance2, DOME_LED_POINTS},\n    input::VisualizerInput,\n    math::{runtime_visualizer_progress_unwrapped, SPLAT_FADE},\n};",
        "use crate::{\n    color_util::light_paint,\n    geometry::{build_dome_led_points, distance2, DOME_LED_POINTS},\n    input::VisualizerInput,\n    math::{runtime_visualizer_progress, runtime_visualizer_progress_unwrapped, SPLAT_FADE},\n};",
    )

    # runtime volume imports
    patch(
        "runtime/volume.rs",
        "    dome::volume::{volume_center_offset, volume_commands_with_wipe, volume_wipe_commands},",
        "    dome::{volume_center_offset, volume_commands_with_wipe, volume_wipe_commands},",
    )
    patch(
        "runtime/snakes.rs",
        "    dome::snakes::{SnakesState, SNAKES_MAX_CATCHUP_STEPS, SNAKES_STEP_MS},",
        "    dome::{SnakesState, SNAKES_MAX_CATCHUP_STEPS, SNAKES_STEP_MS},",
    )

    # flash module — prepend types, fix imports
    flash = (SRC / "dome/flash.rs").read_text()
    if "struct FlashShape" not in flash:
        types = (SRC / "runtime/flash.rs").read_text()
        # extract FlashShape..FlashPolygonAnimation impl block from old runtime/flash if present
        pass
    write(
        "runtime/flash.rs",
        """use domers_outputs::DomeCommand;

use crate::{
    dome::{
        animate_flash_polygon, clear_flash_strut, concentric_layout_from_point, flash_layout_struts,
        FlashPolygonAnimation, FlashShape,
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
""",
    )

    # prepend Flash types to dome/flash if missing
    flash_path = SRC / "dome/flash.rs"
    flash_text = flash_path.read_text()
    if "struct FlashShape" not in flash_text:
        header = """use domers_core::Rgb;
use domers_outputs::{dome_strut_length, topology::DOME_STRUTS, DomeCommand};

use crate::{color_util::scale_rgb_f64, input::VisualizerInput};

use super::volume::{push_unique_usize, volume_gradient_pos, VolumeStrut, VolumeStrutLayout};

#[derive(Clone, Debug)]
pub(crate) struct FlashShape {
    pub(crate) layout: VolumeStrutLayout,
    pub(crate) struts: Vec<usize>,
    pub(crate) animation: Option<FlashPolygonAnimation>,
}

impl FlashShape {
    pub(crate) const ENABLED: bool = true;
    pub(crate) fn enabled() -> bool { Self::ENABLED }
    pub(crate) fn available(&self) -> bool { Self::enabled() && self.animation.is_none() }
}

#[derive(Clone, Debug)]
pub(crate) struct FlashPolygonAnimation {
    pub(crate) pad: u8,
    velocity: f64,
    animation_length: u64,
    starting_time: u64,
    peak_time: u64,
    end_time: u64,
    released: bool,
}

impl FlashPolygonAnimation {
    pub(crate) fn new(pad: u8, velocity: f64, measure_length_ms: u32, now_ms: u64) -> Self {
        let animation_length = u64::from(measure_length_ms) / 4;
        let starting_time = now_ms;
        let peak_time = starting_time + (animation_length * 8 / 10);
        let end_time = starting_time + animation_length;
        Self { pad, velocity, animation_length, starting_time, peak_time, end_time, released: false }
    }
    pub(crate) fn active(&self, now_ms: u64, shape_enabled: bool) -> bool {
        shape_enabled && (!self.released || self.end_time > now_ms)
    }
    pub(crate) fn release(&mut self, now_ms: u64) {
        if self.released { return; }
        self.released = true;
        if now_ms > self.peak_time {
            self.end_time = now_ms + self.animation_length * 2 / 10;
        }
    }
    #[allow(clippy::cast_precision_loss, reason = "Flash intensity mirrors Spectrum millisecond ratios")]
    pub(crate) fn intensity(&self, now_ms: u64) -> f64 {
        if now_ms < self.peak_time {
            (now_ms.saturating_sub(self.starting_time)) as f64
                / (self.peak_time.saturating_sub(self.starting_time)) as f64
        } else if !self.released {
            1.0
        } else if now_ms >= self.end_time {
            0.0
        } else {
            1.0 - (now_ms.saturating_sub(self.peak_time)) as f64
                / (self.end_time.saturating_sub(self.peak_time)) as f64
        }
    }
}

"""
        # strip duplicate header from body
        for prefix in [
            "use domers_core::Rgb;\nuse domers_outputs:",
            "use crate::{color_util::scale_rgb_f64",
        ]:
            if flash_text.startswith(prefix.split("\n")[0]):
                break
        else:
            flash_path.write_text(header + flash_text.split("pub(crate) fn flash_layout_struts", 1)[-1].join if False else header + flash_text[flash_text.find("pub(crate) fn flash_layout_struts"):])

    # tests/mod.rs — flatten and fix paths
    tests = (SRC / "tests/mod.rs").read_text()
    if "mod tests {" in tests:
        tests = tests.replace("mod tests {\n", "", 1)
        if tests.rstrip().endswith("}"):
            tests = tests.rstrip()[:-1] + "\n"
    tests = tests.replace("../../../fixtures/", "../../../../fixtures/")
    tests = tests.replace("super::frame_hash", "frame_hash")
    tests = tests.replace("super::SNAKES_STEP_FRAMES", "SNAKES_STEP_FRAMES")
    tests = tests.replace("super::snake_triangles", "snake_triangles")
    tests = tests.replace("super::SNAKE_TRIANGLE_DEFS", "SNAKE_TRIANGLE_DEFS")
    tests = tests.replace("super::stage_tracer_led_index", "stage_tracer_led_index")
    if "diagnostics::stage_tracer_led_index" not in tests:
        tests = tests.replace(
            "use crate::{\n    hash::{bar_frame_hash, frame_hash, stage_frame_hash},",
            "use crate::{\n    diagnostics::stage_tracer_led_index,\n    dome::{snake_triangles, SNAKE_TRIANGLE_DEFS, SNAKES_STEP_FRAMES},\n    hash::{bar_frame_hash, frame_hash, stage_frame_hash},",
        )
    (SRC / "tests/mod.rs").write_text(tests)

    # remove orphan doc lines at file ends
    for rel in ["runtime/race.rs", "runtime/tv_static.rs", "dome/radial.rs"]:
        p = SRC / rel
        lines = p.read_text().splitlines()
        while lines and lines[-1].startswith("///"):
            lines.pop()
        p.write_text("\n".join(lines) + "\n")

    print("Fixes applied.")


if __name__ == "__main__":
    main()
