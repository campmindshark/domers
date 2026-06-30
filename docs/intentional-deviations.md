# Intentional Deviations From Spectrum

This document separates deliberate Domers design choices from the architecture of the Rust app itself. If Domers intentionally differs from the C# Spectrum app, record the reason and the validation path here.

## Native Config Format

**Spectrum:** Runtime configuration is XML (`spectrum_config.xml` / `spectrum_default_config.xml`).

**Domers:** Runtime configuration is TOML. XML is import-only via:

```sh
cargo run --bin domers-config -- import-spectrum-xml <spectrum.xml> <domers.toml>
```

**Reason:** TOML is idiomatic in Rust, easy to diff, and strict enough for operator-edited config. The importer preserves migration from existing show configs while preventing stale XML fields from becoming runtime state.

**Validation:** `domers-core` tests TOML round-trip behavior and XML import warnings. `tests/integration/config_cli.rs` verifies the CLI writes TOML.

## Madmom Sidecar Path

**Spectrum:** The Windows app searches upward from the assembly for `Madmom/env/Scripts/python.exe`, starts `DBNBeatTracker --host_api --audio_input=<index> online`, and parses `BEAT:{seconds}` stdout lines.

**Domers:** The beat protocol remains `BEAT:{seconds}`, but the command/path is configured in TOML under `[madmom]`. Domers does not assume the Windows virtualenv layout.

**Reason:** Keeping the stdout protocol preserves compatibility with Python Madmom or a wrapper script, while making the app portable and replaceable with a future native Rust beat detector.

**Validation:** `domers-inputs` parses valid and malformed `BEAT:` lines; `domers-core` tests Madmom beat timing windows.

## Browser Simulator Source

**Spectrum:** WPF simulator windows consume command queues in-process and redraw on timer ticks. The dome simulator timer runs every 10 ms in the WPF window.

**Domers:** Browser simulator frames come from the engine/server simulator stream. The browser does not read OPC hardware sockets.

**Reason:** The simulator should be testable without hardware and should display intended engine output, not network side effects.

**Validation:** `domers-outputs` tests simulation-only `WriteBuffer` command emission. `domers-server` tests simulator frame production from server state.

## Config State Handling

**Spectrum:** WPF controls, MIDI callbacks, timers, and operator code share a mutable config object with property-change notifications.

**Domers:** The engine should use frame-local config snapshots and event channels for UI edits and input events.

**Reason:** Snapshot/event flow avoids torn config reads when browser controls, MIDI events, and visualizer rendering happen concurrently.

**Validation:** Current tests cover deterministic scheduler and server state behavior. Future stress tests should exercise concurrent config patching, MIDI replay, and simulator frame generation.

## Timing Targets Kept From Spectrum

Domers keeps the 400 Hz engine compute cap and 200 Hz OPC send cap as compatibility targets, but implements them in Rust with explicit tests rather than copying WPF/Thread behavior.

**Source citations:**

- `spectrum/Spectrum/Operator.cs` defines `MaxFramesPerSecond = 400` and comments that the operator loop runs no faster than 400 Hz.
- `spectrum/LEDs/OPCAPI.cs` defines `MaxRefreshRateHz = 200` and comments that OPC wire rate is independent of engine compute.

**Validation:** Current scheduler tests are deterministic; future timing tests should use fake clocks for frame flow and measured soak tests for runtime limits.
