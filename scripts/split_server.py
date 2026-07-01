#!/usr/bin/env python3
"""Split server/src/lib.rs into focused modules."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "crates" / "server" / "src"
LIB = SRC / "lib.rs"

SECTIONS: list[tuple[str, list[tuple[int, int]]]] = [
    ("types.rs", [(78, 448)]),
    ("state.rs", [(450, 1096)]),
    ("input.rs", [(1097, 2111)]),
    ("hardware.rs", [(2112, 2254)]),
    ("api.rs", [(2255, 2701)]),
    ("frame.rs", [(2703, 3036)]),
    ("tests/mod.rs", [(3039, 4196)]),
]

HEADERS = {
    "types.rs": """use std::time::Duration;

use domers_core::{ColorPalette, DomersConfig, LevelDriverPresetConfig, PaletteEntry, Rgb, TempoSource};
use domers_inputs::{EnumeratedAudioEndpoint, MidiCommand, OrientationDevice, OrientationQuaternion};
use serde::{Deserialize, Serialize};

use crate::{ENGINE_FRAME_INTERVAL, SIMULATOR_FRAME_STRIDE};

""",
    "state.rs": """use std::time::{Duration, Instant};

use domers_core::{ColorPalette, DomersConfig, LevelDriverPresetConfig, PaletteEntry, Rgb, TempoSource};
use domers_inputs::{EnumeratedAudioEndpoint, MidiCommand, OrientationDevice};
use domers_outputs::{BarCommand, DomeCommand, StageCommand};

use crate::types::*;

""",
    "input.rs": """use std::sync::{Arc, atomic::Ordering};
use std::time::Duration;

use domers_core::{DomersConfig, MidiBindingConfig, TempoSource};
use domers_inputs::{MidiCommand, MidiCommandKind};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::state::ServerState;

""",
    "hardware.rs": """use domers_core::DomersConfig;
use domers_outputs::{OpcAddress, OpcClient, PersistentChannel};

use crate::types::HardwareStatus;

""",
    "api.rs": """use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{State, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::{get, patch, post},
    Json, Router,
};
use domers_core::DomersConfig;
use tokio::{net::TcpListener, sync::Mutex};

use crate::{
    frame::render_operator_frame,
    state::ServerState,
    types::*,
};

""",
    "frame.rs": """use domers_core::{ColorPalette, DomersConfig, Rgb};
use domers_engine::{schedule_operator_frame, FullVisualizerSpec, InputSpec, OutputSpec};
use domers_inputs::OrientationDevice;
use domers_outputs::{BarCommand, DomeCommand, StageCommand};
use domers_visualizers::{
    render_bar_diagnostic, render_dome_diagnostic, render_stage_visualizer,
    render_stage_visualizer_with_input, DiagnosticInput, LiveVisualizer, OrientationDeviceInput,
    OrientationOverride, StageVisualizerInput, VisualizerInput, VisualizerRuntime,
};

use crate::{
    hardware::HardwareOutputs,
    state::ServerState,
    types::*,
};

""",
    "tests/mod.rs": """use std::{
    env, fs,
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpStream, UdpSocket as StdUdpSocket},
    time::Duration,
};

use domers_core::{
    AudioDeviceConfig, AudioDeviceFlowConfig, DomersConfig, LevelDriverPresetConfig,
    MidiBindingAction, MidiBindingCommandKind, MidiBindingConfig, PaletteEntry, TempoSource,
    UdpInputConfig,
};
use domers_inputs::{MidiCommand, MidiCommandKind};
use domers_outputs::DomeCommand;
use tokio::time;
use tokio::{io::AsyncReadExt, net::TcpListener};

use crate::{api::serve_listener, frame::render_operator_frame, state::ServerState, types::*};

""",
}


def extract(lines: list[str], ranges: list[tuple[int, int]]) -> str:
    chunks = []
    for start, end in ranges:
        chunks.append("".join(lines[start - 1 : end]))
    return "\n".join(c.rstrip("\n") for c in chunks) + "\n"


def main() -> None:
    text = LIB.read_text()
    lines = text.splitlines(keepends=True)
    LIB.with_suffix(".rs.bak").write_text(text)

    for rel, ranges in SECTIONS:
        path = SRC / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(HEADERS.get(rel, "") + extract(lines, ranges))

    lib = '''//! Runnable Domers server contract and HTTP/WebSocket adapter.

mod api;
mod frame;
mod hardware;
mod input;
mod state;
mod types;

#[cfg(test)]
mod tests;

pub use types::*;

use std::time::Duration;

/// Engine frame interval for the 400 Hz compute cap.
pub const ENGINE_FRAME_INTERVAL: Duration = Duration::from_micros(2_500);

/// Emit simulator frames every 10 ms, matching Spectrum's WPF simulator timer.
pub const SIMULATOR_FRAME_STRIDE: u64 = 4;

const SANDBOX_PREVIEW_FRAME_MS: u64 = 10;
const DOME_CONTROL_BOX_PIXEL_COUNT: usize = 214 * 8;

/// Health status returned by the early API.
#[must_use]
pub const fn health() -> &'static str {
    "ok"
}

pub use api::{serve, serve_listener};
pub use state::{AppRuntime, ServerState};
'''
    LIB.write_text(lib)
    print("Server split complete.")


if __name__ == "__main__":
    main()
