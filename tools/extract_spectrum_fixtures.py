#!/usr/bin/env python3
"""Extract reference fixtures from the Spectrum C# source tree.

This is intentionally lightweight and dependency-free. The goal for M0 is to
make the source of truth repeatable before Rust ports begin relying on these
values.
"""

from __future__ import annotations

import json
import re
import shutil
import hashlib
import xml.etree.ElementTree as ET
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPECTRUM = ROOT.parent / "spectrum"
OUT = ROOT / "fixtures"

VISUALIZER_CLASSES = [
    {
        "name": "LEDDomeStrutIterationDiagnosticVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeStrutIterationDiagnosticVisualizer.cs",
        "classification": "support",
        "case": "dome_diagnostic_strut_iteration",
    },
    {
        "name": "LEDDomeFlashColorsDiagnosticVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeFlashColorsDiagnosticVisualizer.cs",
        "classification": "support",
        "case": "dome_diagnostic_flash_colors",
    },
    {
        "name": "LEDDomeStrandTestDiagnosticVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeStrandTestDiagnosticVisualizer.cs",
        "classification": "support",
        "case": "dome_diagnostic_strand_test",
    },
    {
        "name": "LEDDomeFullColorFlashDiagnosticVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeFullColorFlashDiagnosticVisualizer.cs",
        "classification": "support",
        "case": "dome_diagnostic_full_color_flash",
    },
    {
        "name": "LEDDomeVolumeVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeVolumeVisualizer.cs",
        "classification": "live",
        "case": "dome_volume_default",
    },
    {
        "name": "LEDDomeRadialVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeRadialVisualizer.cs",
        "classification": "live",
        "case": "dome_radial_default",
    },
    {
        "name": "LEDDomeRaceVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeRaceVisualizer.cs",
        "classification": "live",
        "case": "dome_race_default",
    },
    {
        "name": "LEDDomeSnakesVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeSnakesVisualizer.cs",
        "classification": "live",
        "case": "dome_snakes_default",
    },
    {
        "name": "LEDDomeSplatVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeSplatVisualizer.cs",
        "classification": "live",
        "case": "dome_splat_default",
    },
    {
        "name": "LEDDomeQuaternionTestVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeQuaternionTestVisualizer.cs",
        "classification": "live",
        "case": "dome_quaternion_test_default",
    },
    {
        "name": "LEDDomeQuaternionMultiTestVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeQuaternionMultiTestVisualizer.cs",
        "classification": "live",
        "case": "dome_quaternion_multi_test_default",
    },
    {
        "name": "LEDDomeQuaternionPaintbrushVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeQuaternionPaintbrushVisualizer.cs",
        "classification": "live",
        "case": "dome_quaternion_paintbrush_default",
    },
    {
        "name": "LEDDomeTVStaticVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeTVStaticVisualizer.cs",
        "classification": "live",
        "case": "dome_tv_static_default",
    },
    {
        "name": "LEDDomeFlashVisualizer",
        "source": "Spectrum/Visualizers/LEDDomeFlashVisualizer.cs",
        "classification": "live",
        "case": "dome_flash_overlay_default",
    },
    {
        "name": "LEDBarFlashColorsDiagnosticVisualizer",
        "source": "Spectrum/Visualizers/LEDBarFlashColorsDiagnosticVisualizer.cs",
        "classification": "support",
        "case": "bar_diagnostic_flash_colors",
    },
    {
        "name": "LEDStageFlashColorsDiagnosticVisualizer",
        "source": "Spectrum/Visualizers/LEDStageFlashColorsDiagnosticVisualizer.cs",
        "classification": "support",
        "case": "stage_diagnostic_flash_colors",
    },
    {
        "name": "LEDStageDepthLevelVisualizer",
        "source": "Spectrum/Visualizers/LEDStageDepthLevelVisualizer.cs",
        "classification": "live",
        "case": "stage_depth_level_default",
    },
]


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def extract_block(text: str, start: str) -> str:
    start_idx = text.index(start)
    brace_idx = text.index("{", start_idx)
    depth = 0
    for idx in range(brace_idx, len(text)):
        char = text[idx]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return text[brace_idx : idx + 1]
    raise ValueError(f"unterminated block for {start}")


def ints_from_block(block: str) -> list[int]:
    block = re.sub(r"//.*", "", block)
    return [int(match) for match in re.findall(r"-?\d+", block)]


def extract_dome_mapping() -> dict[str, object]:
    text = read(SPECTRUM / "LEDs" / "LEDDomeOutput.cs")

    lengths_block = extract_block(text, "strutLengths")
    strut_lengths = {
        name: int(length)
        for name, length in re.findall(r"LEDDomeStrutTypes\.([A-Za-z]+)\]\s*=\s*(\d+)", lengths_block)
    }

    order_block = extract_block(text, "controlBoxStrutOrder")
    strand_order = [
        re.findall(r"LEDDomeStrutTypes\.([A-Za-z]+)", row)
        for row in re.findall(r"new LEDDomeStrutTypes\[\]\s*\{([^}]*)\}", order_block, re.S)
    ]

    positions_block = extract_block(text, "strutPositions")
    positions = [
        {"control_box": int(box), "control_box_strut_index": int(local)}
        for box, local in re.findall(r"Tuple<int, int>\((\d+),\s*(\d+)\)", positions_block)
    ]

    max_strip_length = max(
        sum(strut_lengths[strut_type] for strut_type in strand)
        for strand in strand_order
    )

    return {
        "source": "spectrum/LEDs/LEDDomeOutput.cs",
        "strut_count": len(positions),
        "strut_lengths": strut_lengths,
        "control_box_strut_order": strand_order,
        "strut_positions": positions,
        "max_strip_length": max_strip_length,
        "bar_control_box": 5,
        "known_unverified": [
            "Physical strut direction and control-box cabling still require hardware sign-off.",
        ],
    }


def extract_dome_geometry() -> dict[str, object]:
    text = read(SPECTRUM / "LEDs" / "StrutLayoutFactory.cs")

    lines_values = ints_from_block(extract_block(text, "public static int[,] lines"))
    lines = [
        {"start": lines_values[index], "end": lines_values[index + 1]}
        for index in range(0, len(lines_values), 2)
    ]

    point_values = ints_from_block(extract_block(text, "handDrawnPoints"))
    points = [
        {
            "x": point_values[index],
            "y": point_values[index + 1],
            "normalized_x": (point_values[index] - 70.0) / 557.0,
            "normalized_y": (point_values[index + 1] - 86.0) / 551.0,
        }
        for index in range(0, len(point_values), 2)
    ]

    return {
        "source": "spectrum/LEDs/StrutLayoutFactory.cs",
        "line_count": len(lines),
        "point_count": len(points),
        "lines": lines,
        "hand_drawn_points": points,
        "known_unverified": [
            "Projection is the C# simulator reference, not a surveyed physical dome model.",
        ],
    }


def extract_topology() -> dict[str, object]:
    config_path = SPECTRUM / "Spectrum" / "spectrum_default_config.xml"
    tree = ET.parse(config_path)
    root = tree.getroot()

    def text(name: str, default: str = "") -> str:
        node = root.find(name)
        return node.text.strip() if node is not None and node.text else default

    stage_side_lengths = [
        int(node.text)
        for node in root.findall("./stageSideLengths/*")
        if node.text is not None
    ]

    return {
        "source": "spectrum/Spectrum/spectrum_default_config.xml",
        "bar": {
            "infinity_width": int(text("barInfinityWidth", "0")),
            "infinity_length": int(text("barInfinityLength", "0")),
            "runner_length": int(text("barRunnerLength", "0")),
            "routed_through_dome_control_box": 5,
        },
        "stage": {
            "side_count": len(stage_side_lengths),
            "layer_count": 3,
            "side_lengths": stage_side_lengths,
            "logical_pixel_count": sum(stage_side_lengths) * 3,
        },
        "known_unverified": [
            "Stage display projection is simulator art and must be checked separately from OPC layout.",
        ],
    }


def write_opc_fixture() -> None:
    packet = bytes([2, 0, 0, 6, 0x12, 0x34, 0x56, 0xAA, 0xBB, 0xCC])
    path = OUT / "spectrum-csharp" / "opc_packets" / "two_pixels_channel_2.bin"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(packet)
    write_json(
        OUT / "spectrum-csharp" / "opc_packets" / "two_pixels_channel_2.json",
        {
            "source": "spectrum/LEDs/OPCAPI.cs",
            "description": "Non-standard Spectrum OPC header without 0xff prefix.",
            "channel": 2,
            "pixels": ["0x123456", "0xaabbcc"],
            "hex": packet.hex(),
        },
    )


def write_orientation_fixture() -> None:
    write_json(
        OUT / "orientation" / "datagram_lengths.json",
        {
            "source": "spectrum/Spectrum/DatagramHandler.cs",
            "layout": {
                "device_id_byte": 0,
                "timestamp_bytes": [1, 2, 3, 4],
                "device_type_byte": 5,
            },
            "types": {
                "1": {"name": "wand_v1", "length": 15},
                "2": {"name": "poi", "length": 17},
                "3": {"name": "wand_v2", "length": 15},
                "4": {"name": "wristband", "length": 15},
            },
        },
    )


def write_visualizer_fixture_manifest() -> None:
    cases = []
    for visualizer in VISUALIZER_CLASSES:
        source_path = SPECTRUM / visualizer["source"]
        source = read(source_path)
        cases.append(
            {
                "name": visualizer["name"],
                "classification": visualizer["classification"],
                "source": f"spectrum/{visualizer['source']}",
                "source_sha256": hashlib.sha256(source.encode("utf-8")).hexdigest(),
                "case": visualizer["case"],
                "input": {
                    "volume": 0.7,
                    "beat_progress": 0.25,
                    "flash_active": True,
                    "palette_slot": 0,
                    "diagnostic_state": 1,
                    "diagnostic_step": 4,
                },
                "expected": {
                    "kind": "frame_hash",
                    "value": None,
                    "status": "pending_csharp_execution",
                },
                "known_unverified": [
                    "Frame hash is source-traceable but not captured until the C# fixture runner executes this case.",
                ],
            }
        )

    write_json(
        OUT / "spectrum-csharp" / "visualizer_frame_cases.json",
        {
            "source": "spectrum/Spectrum/Visualizers",
            "capture_tool": "tools/extract_spectrum_fixtures.py",
            "description": "Deterministic visualizer cases that gate future Spectrum frame golden captures.",
            "cases": cases,
        },
    )


def copy_config_fixture() -> None:
    target = OUT / "config" / "spectrum_default_config.xml"
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(SPECTRUM / "Spectrum" / "spectrum_default_config.xml", target)


def main() -> None:
    write_json(OUT / "spectrum-csharp" / "dome_mapping.json", extract_dome_mapping())
    write_json(OUT / "spectrum-csharp" / "dome_geometry.json", extract_dome_geometry())
    write_json(OUT / "spectrum-csharp" / "bar_stage_topology.json", extract_topology())
    write_opc_fixture()
    write_orientation_fixture()
    write_visualizer_fixture_manifest()
    copy_config_fixture()
    write_json(
        OUT / "manifest.json",
        {
            "source_repo": "spectrum",
            "capture_tool": "tools/extract_spectrum_fixtures.py",
            "known_unverified": [
                "Hardware wiring must still be checked with diagnostic patterns.",
                "Visualizer frame cases are source-traceable; pending hashes require the C# fixture runner.",
            ],
            "fixture_groups": [
                "spectrum-csharp/dome_mapping.json",
                "spectrum-csharp/dome_geometry.json",
                "spectrum-csharp/bar_stage_topology.json",
                "spectrum-csharp/opc_packets/",
                "spectrum-csharp/visualizer_frame_cases.json",
                "config/spectrum_default_config.xml",
                "orientation/datagram_lengths.json",
            ],
        },
    )


if __name__ == "__main__":
    main()
