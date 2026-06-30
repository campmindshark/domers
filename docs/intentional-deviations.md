# Intentional Deviations From Spectrum

This document separates deliberate `dome-rs` design choices from the architecture of the Rust app itself. If `dome-rs` intentionally differs from the C# Spectrum app, record the reason and the validation path here.

## Native Config Format

**Spectrum:** Runtime configuration is XML (`spectrum_config.xml` / `spectrum_default_config.xml`).

**dome-rs:** Runtime configuration is TOML. XML is import-only via:

```sh
cargo run --bin domers-config -- import-spectrum-xml <spectrum.xml> <domers.toml>
```

**Reason:** TOML is idiomatic in Rust, easy to diff, and strict enough for operator-edited config. The importer preserves migration from existing show configs while preventing stale XML fields from becoming runtime state.

**Validation:** `domers-core` tests TOML round-trip behavior, shared-entry palette expansion, and XML import warnings. `tests/config_cli.rs` verifies the CLI writes TOML and that the Spectrum XML fixture produces the checked `examples/domers.toml`.

## Palette TOML Shape

**Spectrum:** The XML stores the palette as 64 absolute color entries: eight palette banks with eight slots each.

**dome-rs:** Runtime still expands to the same 64 absolute slots, but TOML stores repeated colors once under `color_palette.entries` and references them from `color_palette.banks`. The old verbose `[[color_palette.colors]]` form still parses for compatibility.

**Reason:** The Spectrum XML contains duplicate palette entries. Shared TOML entries keep the checked config readable without changing runtime palette indexing.

**Validation:** `domers-core` tests shared-entry serialization and parsing. `tests/config_cli.rs` verifies the Spectrum XML fixture regenerates the checked `examples/domers.toml`.

## Checked Example OPC Hosts

**Spectrum:** The dome XML fixture points at the show network, for example `domeBeagleboneOPCAddress = 192.168.1.69:7890`.

**dome-rs:** The checked `examples/domers.toml` rewrites imported OPC hosts to `127.0.0.1` so local loopback services can stand in for ledscape during development. Operators replace those addresses with show-network hosts before physical output.

**Reason:** The example config should be runnable and testable on a development machine without accidentally writing to show hardware.

**Validation:** `domers-core` tests localhost address rewriting during XML import. `tests/config_cli.rs` verifies the checked example is exactly what the importer produces.

## Madmom Sidecar Packaging

**Spectrum:** The Windows app searches upward from the assembly for `Madmom/env/Scripts/python.exe`, starts `DBNBeatTracker --host_api --audio_input=<index> online`, and parses `BEAT:{seconds}` stdout lines.

**dome-rs:** The beat protocol remains `BEAT:{seconds}`. `domers-inputs` provides a sidecar wrapper for the same `DBNBeatTracker --host_api --audio_input=<index> online` launch contract, `domers doctor` validates the configured command when `tempo.source = "madmom"`, and `domers run` starts the sidecar and ingests stdout lines into the beat runtime. Packaging is more flexible: the configured command can point at a bundled Python environment plus `tracker = "DBNBeatTracker"`, wrapper script, system install, Docker launcher, or native replacement. The source repo does not require a Madmom git submodule.

**Reason:** The old virtualenv path is a Windows packaging detail, not the feature. The feature contract is beat tracking that emits parseable `BEAT:{seconds}` lines and feeds the beat engine.

**Validation:** `domers-inputs` parses valid and malformed `BEAT:` lines and tests sidecar launch arguments plus disabled lifecycle behavior. `domers-core` tests Madmom beat timing windows. `domers-server` tests feeding parsed Madmom beat lines into runtime input state.

## Browser Simulator Source

**Spectrum:** WPF simulator windows consume command queues in-process and redraw on timer ticks. The dome simulator timer runs every 10 ms in the WPF window.

**dome-rs:** Browser simulator frames come from engine/server-rendered frames. The live controls page opens a WebSocket only while the `Preview` drawer is open. The dedicated `/simulator` page renders request-local sandbox frames through `POST /api/simulator/sandbox-frame`, so its controls do not patch runtime config or hardware output. The browser does not read OPC hardware sockets.

**Reason:** The simulator needs to run without hardware and display intended engine output, not network side effects.

**Validation:** `domers-outputs` tests simulation-only `WriteBuffer` command emission. `domers-server` tests runtime preview frame production and sandbox frames that do not mutate runtime state. `ui/check.mjs` keeps the live Preview and `/simulator` controls separated.

## Config State Handling

**Spectrum:** WPF controls, MIDI callbacks, timers, and operator code share a mutable config object with property-change notifications.

**dome-rs:** The engine renders each frame from a config/input snapshot. Browser patches and input events enter through explicit server methods instead of arbitrary UI object mutation during rendering.

**Reason:** Snapshot-based rendering avoids torn config reads when browser controls, MIDI events, and visualizer rendering happen concurrently.

**Validation:** Tests cover deterministic scheduler and server state behavior, including config patching during frame production, MIDI replay paths, and simulator frame generation. Remaining stress coverage belongs in longer soak/load tests.

## Timing Targets Kept From Spectrum

`dome-rs` keeps the 400 Hz engine compute cap and 200 Hz OPC send cap as compatibility targets, but implements them in Rust with explicit tests rather than copying WPF/Thread behavior.

**Source citations:**

- `spectrum/Spectrum/Operator.cs` defines `MaxFramesPerSecond = 400` and comments that the operator loop runs no faster than 400 Hz.
- `spectrum/LEDs/OPCAPI.cs` defines `MaxRefreshRateHz = 200` and comments that OPC wire rate is independent of engine compute.

**Validation:** Scheduler tests are deterministic. Remaining timing coverage: fake-clock frame tests and measured soak tests for runtime limits.
