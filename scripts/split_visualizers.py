#!/usr/bin/env python3
"""Split visualizers/src/lib.rs into focused modules."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "crates" / "visualizers" / "src"
LIB = SRC / "lib.rs"

# (relative_path, [(start_line, end_line), ...])  — 1-indexed, inclusive
SECTIONS: list[tuple[str, list[tuple[int, int]]]] = [
    ("inventory.rs", [(23, 111)]),
    ("input.rs", [(113, 330)]),
    ("render.rs", [(332, 368), (1518, 1578)]),
    ("runtime/volume.rs", [(467, 502)]),
    ("runtime/tv_static.rs", [(504, 535)]),
    ("runtime/flash.rs", [(537, 720)]),
    ("runtime/snakes.rs", [(722, 758)]),
    ("runtime/race.rs", [(760, 938)]),
    ("buffer.rs", [(940, 1104)]),
    ("runtime/radial.rs", [(1106, 1244)]),
    ("runtime/splat.rs", [(1245, 1296)]),
    ("runtime/paintbrush.rs", [(1298, 1516)]),
    ("rng.rs", [(1602, 1703)]),
    ("diagnostics.rs", [(1580, 1599), (1705, 2061), (2085, 2108)]),
    ("dome/volume.rs", [(2110, 2556), (2690, 2914)]),
    ("dome/flash.rs", [(2557, 2688)]),
    ("dome/radial.rs", [(2916, 2968)]),
    ("dome/splat.rs", [(2969, 3030)]),
    ("dome/race.rs", [(3031, 3174)]),
    ("dome/snakes.rs", [(3176, 3514)]),
    ("dome/quaternion.rs", [(3516, 3632)]),
    ("quaternion.rs", [(3634, 3726)]),
    ("dome/paintbrush.rs", [(3728, 3814)]),
    ("math.rs", [(1117, 1157), (3820, 3925), (3870, 3894)]),
    ("geometry.rs", [(3927, 4041)]),
    ("color_util.rs", [(2062, 2083), (4049, 4079)]),
    ("hash.rs", [(4081, 4171)]),
    ("tests/mod.rs", [(4174, 5252)]),
]

# Overlap fix: dome/volume should not include flash helpers; dome/flash is separate.
# dome/radial starts at VOLUME_LINES (2723) not 2916.


def extract(lines: list[str], ranges: list[tuple[int, int]]) -> str:
    chunks: list[str] = []
    for start, end in ranges:
        chunks.append("".join(lines[start - 1 : end]))
    return "\n".join(chunk.rstrip("\n") for chunk in chunks) + "\n"


def main() -> None:
    text = LIB.read_text()
    lines = text.splitlines(keepends=True)
    backup = LIB.with_suffix(".rs.bak")
    backup.write_text(text)
    print(f"Backed up to {backup}")

    for rel_path, ranges in SECTIONS:
        out = SRC / rel_path
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(extract(lines, ranges))
        print(f"Wrote {out.relative_to(ROOT)} ({sum(e - s + 1 for s, e in ranges)} lines)")

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
    (SRC / "runtime" / "mod.rs").write_text(
        '''mod flash;
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

'''
        + extract(lines, [(380, 465)])
    )
    (SRC / "dome" / "mod.rs").write_text(
        '''mod flash;
mod paintbrush;
mod quaternion;
mod race;
mod radial;
mod snakes;
mod splat;
mod tv_static;
mod volume;
'''
    )
    print(f"Wrote new {LIB.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
