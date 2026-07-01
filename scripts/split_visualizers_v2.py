#!/usr/bin/env python3
"""Split visualizers/src/lib.rs into modules with import headers."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "crates" / "visualizers" / "src"
LIB = SRC / "lib.rs"

HEADERS: dict[str, str] = {
    "inventory.rs": "",
    "input.rs": """use domers_core::{ColorPalette, PaletteEntry, Rgb};

""",
    "render.rs": """use domers_core::Rgb;
use domers_outputs::{topology::DOME_PIXELS, BarCommand, DomeCommand, DomeOutputSink, StageCommand};

use crate::{
    diagnostics::{bar_flash_colors, dome_flash_colors_commands, dome_full_color_flash_commands, dome_strand_test_commands, dome_strut_iteration_commands, stage_depth_level, stage_flash_colors},
    dome::{race_commands, radial_frame, snakes_commands, splat_frame, tv_static_commands, volume_commands, quaternion_multi_test_frame, quaternion_paintbrush_frame, quaternion_test_frame},
    input::{BarDiagnosticVisualizer, DiagnosticInput, DomeDiagnosticVisualizer, LiveVisualizer, StageVisualizer, StageVisualizerInput, VisualizerInput},
};

""",
    "runtime/mod.rs": """mod flash;
mod paintbrush;
mod race;
mod radial;
mod snakes;
mod splat;
mod tv_static;
mod volume;

use domers_core::Rgb;
use domers_outputs::{topology::DOME_PIXELS, DomeCommand};

use crate::{
    dome,
    input::{LiveVisualizer, VisualizerInput},
    render::render_dome_visualizer,
};

use flash::FlashRuntime;
use paintbrush::PaintbrushRuntime;
use race::RaceRuntime;
use radial::RadialRuntime;
use snakes::SnakesRuntime;
use splat::SplatRuntime;
use tv_static::TvStaticRuntime;
use volume::VolumeRuntime;

""",
    "runtime/volume.rs": """use domers_outputs::DomeCommand;

use crate::{
    dome::volume::{volume_center_offset, volume_commands_with_wipe, volume_wipe_commands},
    input::VisualizerInput,
};

""",
    "runtime/tv_static.rs": """use domers_outputs::{dome_strut_length, topology::DOME_STRUTS, DomeCommand};

use crate::rng::DotNetRandom;

""",
    "runtime/flash.rs": """use domers_core::Rgb;
use domers_outputs::DomeCommand;

use crate::{
    dome::{
        animate_flash_polygon, clear_flash_strut, flash_layout_struts, flash_pad_gradient_color,
        flash_pad_single_color, volume_layouts, VolumeStrutLayout,
    },
    input::VisualizerInput,
    rng::DotNetRandom,
};

""",
    "runtime/snakes.rs": """use domers_outputs::DomeCommand;

use crate::{
    dome::{SnakesState, SNAKES_MAX_CATCHUP_STEPS, SNAKES_STEP_MS},
    input::VisualizerInput,
};

""",
    "runtime/race.rs": """use domers_core::Rgb;
use domers_outputs::{dome_strut_length, topology::DOME_STRUTS, DomeCommand};

use crate::{
    dome::race::{race_pixel_color, RaceRacer, RACE_RACER_CONFIGS},
    geometry::{build_dome_led_points, DomeLedPoint, DOME_LED_POINTS},
    input::VisualizerInput,
};

""",
    "runtime/radial.rs": """use domers_core::Rgb;
use domers_outputs::DomeCommand;

use crate::{
    buffer::DomeBuffer,
    geometry::{build_dome_led_points, DOME_LED_POINTS},
    input::VisualizerInput,
    math::{map_value, map_wrap, polar_to_cartesian, radial_effect, wrap, DOME_RADIAL_CENTER_ANGLE, DOME_RADIAL_CENTER_DISTANCE, DOME_RADIAL_CENTER_SPEED, DOME_RADIAL_EFFECT, DOME_RADIAL_FREQUENCY, DOME_RADIAL_SIZE, DOME_GLOBAL_FADE_SPEED, DOME_GLOBAL_HUE_SPEED},
};

""",
    "runtime/splat.rs": """use domers_outputs::DomeCommand;

use crate::{
    buffer::DomeBuffer,
    input::VisualizerInput,
    math::runtime_visualizer_progress_unwrapped,
    rng::DotNetRandom,
};

""",
    "runtime/paintbrush.rs": """use domers_core::Rgb;
use domers_outputs::DomeCommand;

use crate::{
    buffer::DomeBuffer,
    color_util::{hsv_to_rgb, light_paint},
    geometry::{distance3, hemisphere_point, DOME_LED_POINTS, build_dome_led_points},
    input::VisualizerInput,
    math::spectrum_nudge,
    quaternion::Quaternion,
    rng::DotNetRandom,
};

""",
    "rng.rs": """use domers_core::Rgb;

""",
    "buffer.rs": """use domers_core::Rgb;

use crate::{
    color_util::scale_rgb_f64,
    geometry::{build_dome_led_points, DOME_LED_POINTS},
};

""",
    "diagnostics.rs": """use domers_core::Rgb;
use domers_outputs::{
    dome_strut_index_for_control_box, dome_strut_length,
    topology::{DOME_STRUTS, STAGE_LAYERS},
    BarCommand, DomeCommand, StageCommand,
};

use crate::{
    color_util::{diagnostic_colors, scale_rgb_f64, white},
    input::{DiagnosticInput, StageVisualizerInput},
};

""",
    "dome/tv_static.rs": """use domers_outputs::{dome_strut_length, topology::{DOME_PIXELS, DOME_STRUTS}, DomeCommand};

use crate::{input::VisualizerInput, rng::DotNetRandom};

""",
    "dome/volume.rs": """use domers_core::Rgb;
use domers_outputs::{dome_strut_length, topology::DOME_STRUTS, DomeCommand};

use crate::{color_util::scale_rgb_f64, input::VisualizerInput};

""",
    "dome/flash.rs": """use domers_core::Rgb;
use domers_outputs::{dome_strut_length, topology::DOME_STRUTS, DomeCommand};

use crate::{
    dome::volume::{VolumeStrut, VolumeStrutLayout},
    input::VisualizerInput,
};

""",
    "dome/radial.rs": """use domers_core::Rgb;
use domers_outputs::topology::DOME_PIXELS;

use crate::{
    dome::race::VOLUME_ROTATION_SPEED,
    dome::volume::VOLUME_GRADIENT_SPEED,
    geometry::{build_dome_led_points, DOME_LED_POINTS},
    input::VisualizerInput,
    math::{map_wrap, runtime_visualizer_progress_unwrapped, wrap},
};

""",
    "dome/splat.rs": """use domers_core::Rgb;
use domers_outputs::topology::DOME_PIXELS;

use crate::{
    geometry::{build_dome_led_points, distance2, DOME_LED_POINTS},
    input::VisualizerInput,
    math::{runtime_visualizer_progress_unwrapped, SPLAT_FADE},
};

""",
    "dome/race.rs": """use domers_core::Rgb;
use domers_outputs::{dome_strut_length, topology::{DOME_PIXELS, DOME_STRUTS}, DomeCommand};

use crate::{
    geometry::{build_dome_led_points, DomeLedPoint, DOME_LED_POINTS},
    input::VisualizerInput,
};

""",
    "dome/snakes.rs": """use std::collections::VecDeque;
use std::sync::OnceLock;

use domers_core::Rgb;
use domers_outputs::{dome_strut_length, DomeCommand};

use crate::{
    color_util::scale_rgb_f64,
    input::VisualizerInput,
    rng::DotNetRandom,
};

""",
    "dome/quaternion.rs": """use domers_core::Rgb;
use domers_outputs::topology::DOME_PIXELS;

use crate::{
    geometry::{build_dome_led_points, DOME_LED_POINTS},
    input::VisualizerInput,
    math::{max_axis_by_abs, runtime_visualizer_progress, spectrum_quaternion_test_point},
    quaternion::Quaternion,
};

""",
    "dome/paintbrush.rs": """use domers_core::Rgb;
use domers_outputs::topology::DOME_PIXELS;

use crate::{
    color_util::{hsv_to_rgb, light_paint},
    geometry::{build_dome_led_points, distance3, hemisphere_point, DOME_LED_POINTS},
    input::VisualizerInput,
    math::{paintbrush_frame_in_cycle, paintbrush_twinkle, spectrum_nudge},
    quaternion::Quaternion,
    rng::DotNetRandom,
};

""",
    "quaternion.rs": "",
    "math.rs": """use domers_core::Rgb;

use crate::{input::VisualizerInput, rng::DotNetRandom};

""",
    "geometry.rs": """use std::sync::OnceLock;

use domers_outputs::topology::DOME_PIXELS;
use serde::Deserialize;

pub(crate) const DOME_GEOMETRY_JSON: &str =
    include_str!("../../../fixtures/spectrum-csharp/dome_geometry.json");
pub(crate) const DOME_MAPPING_JSON: &str =
    include_str!("../../../fixtures/spectrum-csharp/dome_mapping.json");
pub(crate) static DOME_LED_POINTS: OnceLock<Vec<DomeLedPoint>> = OnceLock::new();

""",
    "color_util.rs": """use domers_core::Rgb;

""",
    "hash.rs": """use domers_outputs::{BarCommand, DomeCommand, StageCommand};

""",
    "tests/mod.rs": """use domers_core::import_spectrum_xml;
use domers_outputs::{topology::DOME_PIXELS, DomeCommand};
use serde::Deserialize;

use crate::{
    hash::{bar_frame_hash, frame_hash, stage_frame_hash},
    render::{render_bar_diagnostic, render_dome_diagnostic, render_dome_visualizer, render_stage_visualizer, render_stage_visualizer_with_input},
    input::{BarDiagnosticVisualizer, DiagnosticInput, DomeDiagnosticVisualizer, LiveVisualizer, MidiNoteInput, OrientationOverride, StageVisualizer, StageVisualizerInput, VisualizerInput},
    inventory::{Classification, INVENTORY},
    runtime::VisualizerRuntime,
    input::{MAX_FRAME_MIDI_NOTES, MAX_ORIENTATION_DEVICES},
};

""",
    "dome/mod.rs": """mod flash;
mod paintbrush;
mod quaternion;
mod race;
mod radial;
mod snakes;
mod splat;
mod tv_static;
mod volume;

pub(crate) use flash::{animate_flash_polygon, clear_flash_strut, flash_layout_struts, flash_pad_gradient_color, flash_pad_single_color};
pub(crate) use paintbrush::quaternion_paintbrush_frame;
pub(crate) use quaternion::{quaternion_multi_test_frame, quaternion_test_frame};
pub(crate) use race::race_commands;
pub(crate) use radial::radial_frame;
pub(crate) use snakes::snakes_commands;
pub(crate) use splat::splat_frame;
pub(crate) use tv_static::tv_static_commands;
pub(crate) use volume::{volume_center_offset, volume_commands, volume_commands_with_wipe, volume_layouts, volume_wipe_commands, VolumeStrutLayout};

""",
}

SECTIONS: list[tuple[str, list[tuple[int, int]]]] = [
    ("inventory.rs", [(23, 111)]),
    ("input.rs", [(113, 330)]),
    ("render.rs", [(332, 368), (1518, 1578)]),
    ("runtime/volume.rs", [(467, 502)]),
    ("runtime/tv_static.rs", [(504, 535)]),
    ("runtime/flash.rs", [(537, 718)]),
    ("runtime/snakes.rs", [(720, 758)]),
    ("runtime/race.rs", [(873, 938)]),
    ("buffer.rs", [(940, 1104)]),
    ("runtime/radial.rs", [(1161, 1244)]),
    ("runtime/splat.rs", [(1245, 1296)]),
    ("runtime/paintbrush.rs", [(1298, 1516)]),
    ("rng.rs", [(1602, 1703)]),
    ("dome/tv_static.rs", [(1580, 1599)]),
    ("diagnostics.rs", [(1705, 2061), (2085, 2108)]),
    ("dome/volume.rs", [(2110, 2556), (2690, 2914)]),
    ("dome/flash.rs", [(2557, 2688)]),
    ("dome/radial.rs", [(2916, 2963)]),
    ("dome/splat.rs", [(2969, 3030)]),
    ("dome/race.rs", [(760, 871), (3031, 3174)]),
    ("dome/snakes.rs", [(3176, 3514)]),
    ("dome/quaternion.rs", [(3516, 3567)]),
    ("dome/paintbrush.rs", [(3569, 3632), (3728, 3814)]),
    ("quaternion.rs", [(3634, 3726)]),
    ("math.rs", [(1106, 1157), (3820, 3925)]),
    ("geometry.rs", [(3927, 4041)]),
    ("color_util.rs", [(2062, 2083), (4049, 4079)]),
    ("hash.rs", [(4081, 4171)]),
    ("tests/mod.rs", [(4174, 5252)]),
]

# runtime/mod.rs body appended separately
RUNTIME_BODY = (380, 465)


def extract(lines: list[str], ranges: list[tuple[int, int]]) -> str:
    chunks: list[str] = []
    for start, end in ranges:
        chunks.append("".join(lines[start - 1 : end]))
    return "\n".join(chunk.rstrip("\n") for chunk in chunks) + "\n"


def pub_crate_body(body: str) -> str:
    out: list[str] = []
    for line in body.splitlines(keepends=True):
        stripped = line.lstrip()
        if stripped.startswith(("struct ", "enum ", "fn ", "const ", "static ", "type ")):
            indent = line[: len(line) - len(stripped)]
            if not stripped.startswith("pub"):
                line = f"{indent}pub(crate) {stripped}"
        out.append(line)
    return "".join(out)


def main() -> None:
    text = LIB.read_text()
    lines = text.splitlines(keepends=True)
    backup = LIB.with_suffix(".rs.bak")
    if not backup.exists() or backup.read_text() != text:
        backup.write_text(text)

    for rel_path, ranges in SECTIONS:
        out = SRC / rel_path
        out.parent.mkdir(parents=True, exist_ok=True)
        header = HEADERS.get(rel_path, "")
        body = pub_crate_body(extract(lines, ranges))
        out.write_text(header + body)

    runtime_mod = SRC / "runtime" / "mod.rs"
    runtime_mod.write_text(
        HEADERS["runtime/mod.rs"] + pub_crate_body(extract(lines, [RUNTIME_BODY]))
    )
    (SRC / "dome" / "mod.rs").write_text(HEADERS["dome/mod.rs"])

    lib_rs = '''//! Visualizer inventory and porting order.

#![allow(
    clippy::large_types_passed_by_value,
    reason = "VisualizerInput is Copy and passed by value throughout the Spectrum port"
)]

mod buffer;
mod color_util;
mod diagnostics;
mod dome;
mod geometry;
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

pub use input::{
    BarDiagnosticVisualizer, DiagnosticInput, DomeDiagnosticVisualizer, LiveVisualizer,
    MidiNoteInput, OrientationDeviceInput, OrientationOverride, StageVisualizer,
    StageVisualizerInput, VisualizerInput, MAX_FRAME_MIDI_NOTES, MAX_ORIENTATION_DEVICES,
};
pub use inventory::{Classification, VisualizerInventory, INVENTORY};
pub use quaternion::Quaternion;
pub use render::{
    render_bar_diagnostic, render_dome_diagnostic, render_dome_visualizer,
    render_stage_visualizer, render_stage_visualizer_with_input,
};
pub use runtime::VisualizerRuntime;
'''
    LIB.write_text(lib_rs)
    print("Split complete.")


if __name__ == "__main__":
    main()
