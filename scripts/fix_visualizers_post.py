#!/usr/bin/env python3
"""Post-split fixes for visualizers modules."""

from pathlib import Path

SRC = Path(__file__).resolve().parents[1] / "crates" / "visualizers" / "src"

REPLACEMENTS = [
    ("rng.rs", "pub(crate) struct DotNetRandom", "#[derive(Clone, Debug)]\npub(crate) struct DotNetRandom"),
    ("geometry.rs", "pub(crate) struct DomeLedPoint {\n    index:", "pub(crate) struct DomeLedPoint {\n    pub(crate) index:"),
    ("geometry.rs", "    x: f64,\n    y: f64,", "    pub(crate) x: f64,\n    pub(crate) y: f64,"),
    ("buffer.rs", "pub(crate) struct DomeBuffer {\n    pixels:", "pub(crate) struct DomeBuffer {\n    pub(crate) pixels:"),
    ("quaternion.rs", "    x: f64,\n    y: f64,\n    z: f64,\n    w: f64,", "    pub(crate) x: f64,\n    pub(crate) y: f64,\n    pub(crate) z: f64,\n    pub(crate) w: f64,"),
    ("dome/volume.rs", "struct VolumeStrutLayout {\n    segments:", "pub(crate) struct VolumeStrutLayout {\n    pub(crate) segments:"),
    ("dome/volume.rs", "struct VolumeEdge {", "pub(crate) struct VolumeEdge {"),
    ("dome/volume.rs", "pub(crate) const VOLUME_GRADIENT_SPEED", "pub(crate) const VOLUME_GRADIENT_SPEED"),
]

# Fix pub(crate) on impl inner consts in rng.rs
RNG_IMPL_CONST = """impl DotNetRandom {
    const MBIG: i32 = 2_147_483_647;
    const MSEED: i32 = 161_803_398;
"""


def main() -> None:
    rng = SRC / "rng.rs"
    text = rng.read_text()
    text = text.replace(
        "    pub(crate) const MBIG: i32 = 2_147_483_647;\n    pub(crate) const MSEED: i32 = 161_803_398;",
        "    const MBIG: i32 = 2_147_483_647;\n    const MSEED: i32 = 161_803_398;",
    )
    if "#[derive(Clone, Debug)]" not in text:
        text = text.replace(
            "pub(crate) struct DotNetRandom",
            "#[derive(Clone, Debug)]\npub(crate) struct DotNetRandom",
            1,
        )
    rng.write_text(text)

    geo = SRC / "geometry.rs"
    g = geo.read_text()
    g = g.replace(
        "pub(crate) struct DomeLedPoint {\n    index: usize,\n    x: f64,\n    y: f64,\n}",
        "pub(crate) struct DomeLedPoint {\n    pub(crate) index: usize,\n    pub(crate) x: f64,\n    pub(crate) y: f64,\n}",
    )
    geo.write_text(g)

    buf = SRC / "buffer.rs"
    b = buf.read_text()
    b = b.replace(
        "pub(crate) struct DomeBuffer {\n    pixels: Vec<DomeBufferPixel>,",
        "pub(crate) struct DomeBuffer {\n    pub(crate) pixels: Vec<DomeBufferPixel>,",
    )
    buf.write_text(b)

    quat = SRC / "quaternion.rs"
    q = quat.read_text()
    q = q.replace(
        "pub struct Quaternion {\n    x: f64,\n    y: f64,\n    z: f64,\n    w: f64,\n}",
        "pub struct Quaternion {\n    pub(crate) x: f64,\n    pub(crate) y: f64,\n    pub(crate) z: f64,\n    pub(crate) w: f64,\n}",
    )
    quat.write_text(q)

    vol = SRC / "dome" / "volume.rs"
    v = vol.read_text()
    v = v.replace(
        "struct VolumeStrutLayout {\n    segments: Vec<VolumeStrutLayoutSegment>,",
        "pub(crate) struct VolumeStrutLayout {\n    pub(crate) segments: Vec<VolumeStrutLayoutSegment>,",
    )
    v = v.replace("struct VolumeEdge {", "pub(crate) struct VolumeEdge {")
    vol.write_text(v)

    # Export SnakesState from dome/snakes for runtime
    snakes_mod = SRC / "dome" / "mod.rs"
    sm = snakes_mod.read_text()
    if "SnakesState" not in sm:
        sm = sm.replace(
            "pub(crate) use snakes::snakes_commands;",
            "pub(crate) use snakes::{snakes_commands, SnakesState, SNAKES_MAX_CATCHUP_STEPS, SNAKES_STEP_MS};",
        )
        snakes_mod.write_text(sm)

    runtime_snakes = SRC / "runtime" / "snakes.rs"
    rs = runtime_snakes.read_text()
    rs = rs.replace(
        "use crate::{\n    dome::snakes::{SnakesState, SNAKES_MAX_CATCHUP_STEPS, SNAKES_STEP_MS},\n    input::VisualizerInput,\n};",
        "use crate::{dome::snakes::{SnakesState, SNAKES_MAX_CATCHUP_STEPS, SNAKES_STEP_MS}, input::VisualizerInput};",
    )
    runtime_snakes.write_text(rs)

    print("Post-fix applied.")


if __name__ == "__main__":
    main()
