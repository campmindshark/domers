# Testing

The automated suite covers deterministic runtime behavior, protocol encoding, config migration, and the browser shell. Hardware validation is a release checklist item.

## PR Fast

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd ui && npm install && npm run build && cd ..
node ui/check.mjs
```

## Covered

- TOML config round-trips and Spectrum XML import warnings.
- Scheduler priority rules: priority `0`, priority ties, diagnostics, disabled inputs, and output activation.
- OPC non-standard frame encoding and persistent sparse flush behavior.
- Simulator command emission with hardware disabled, including dome/bar/stage preview streams.
- Fake audio, MIDI, orientation, and Madmom beat inputs.
- Live UDP audio/MIDI/orientation adapter parsing and runtime lifecycle ingestion.
- Audio device identity/all-endpoint index mapping and Madmom audio-index derivation.
- MIDI device-scoped command state, wildcard/exact binding actions, MIDI log state, and Spectrum knob math.
- Orientation datagram parsing, device state, calibration, action flags, poi speed, and stale-device removal.
- Madmom sidecar launch argument, disabled-lifecycle behavior, managed stdout ingestion path, derived audio input index, and fake-sidecar runtime test.
- Spectrum-compatible 64-entry palette indexing and gradient blending.
- Shared-entry palette TOML serialization, parsing, and XML import golden output.
- Visualizer simulator-frame harness, local frame-hash snapshots for live dome modes, captured headless Spectrum C# visualizer goldens, and manifest coverage that rejects missing visualizer hashes.
- Server state contract for full config reload/apply, runtime config patching, palette patching, start/stop, metrics, input status, hardware status, and simulator frames.
- HTTP adapter smoke coverage for the browser shell, simulator page, start/stop, tap tempo, dome config patching, palette patching, and sandbox simulator frames.
- UI smoke markers for API/WebSocket wiring, pixel rendering, full palette controls, structured config controls, input status controls, and simulator controls.
- OPC loopback write and reconnect tests.

Tests for intentional behavior changes cite
[`intentional-deviations.md`](intentional-deviations.md) so preservation and
replacement decisions stay visible.

## Example Local Run

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd ui && npm install && npm run build && cd ..
node ui/check.mjs
```

Live local smoke:

```sh
cargo run --bin domers -- run --config examples/domers.toml --bind 127.0.0.1:3000
```

Preflight the same config without starting outputs:

```sh
cargo run --bin domers -- doctor --config examples/domers.toml --bind 127.0.0.1:3000
```

Then open `http://127.0.0.1:3000`, click `Start`, and confirm the metrics advance.

## PR Full And Nightly

```sh
python3 tools/build_spectrum_csharp.py
python3 tools/capture_spectrum_visualizer_frames.py
python3 tools/check_visualizer_goldens.py
cargo test -p domers-visualizers rust_visualizer_hashes_match_spectrum_csharp_goldens -- --ignored --nocapture
```

`build_spectrum_csharp.py` is the Windows/.NET gate for executable Spectrum
fixture capture. It initializes the Spectrum `Madmom` submodule if needed and
builds `../spectrum/Spectrum/Spectrum.csproj` directly because the legacy
solution's `Madmom/Madmom.pyproj` requires Visual Studio Python Tools.
`capture_spectrum_visualizer_frames.py` executes the old Spectrum visualizers
headlessly with simulation-only output, and `check_visualizer_goldens.py` ensures
all captured hashes are present. The ignored Rust-vs-Spectrum test is the active
exactness ledger while live visualizer ports and the Stage Depth TODO remain
open.

Browser screenshots, load tests, and physical hardware sign-off artifacts are release evidence, not prerequisites for the no-hardware test suite.

## Manual Hardware Checklist

Use [`hardware-readiness.md`](hardware-readiness.md) for release sign-off.

- Dome flash-by-strut, strut iteration, strand test, and full-color flash.
- Bar control box 5 and runner/corner diagnostics.
- Stage side/layer diagnostics.
- OPC reconnect after interruption.
- MIDI board, audio volume, Madmom beat tracking, tap tempo, and orientation paintbrush.
