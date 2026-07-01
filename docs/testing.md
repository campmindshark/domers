# Testing

The automated suite covers deterministic runtime behavior, protocol encoding, config migration, and the browser shell. Hardware validation is a release checklist item.

## PR Fast

```sh
make e2e
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
- Visualizer simulator-frame harness, local frame-hash snapshots for live dome modes, captured headless Spectrum C# first-frame visualizer goldens (17/17), and multi-frame sequence goldens (11/11) replayed through persistent `VisualizerRuntime`.
- Manifest coverage that rejects missing or pending visualizer hashes (`check_visualizer_goldens.py`).
- Server state contract for full config reload/apply, runtime config patching, palette patching, start/stop, metrics, input status, hardware status, and simulator frames.
- HTTP adapter smoke coverage for the browser shell, simulator page, start/stop, tap tempo, duplicate tap filtering, dome config patching, palette patching, and sandbox simulator frames.
- UI smoke markers for API/WebSocket wiring, pixel rendering, full palette controls, structured config controls, input status controls, and simulator controls.
- OPC loopback write and reconnect tests.

Tests for intentional behavior changes cite
[`intentional-deviations.md`](intentional-deviations.md) so preservation and
replacement decisions stay visible.

## Example Local Run

```sh
make e2e
```

Live local smoke:

```sh
make run CONFIG=examples/domers.toml BIND=127.0.0.1:3000
```

Preflight the same config without starting outputs:

```sh
cargo run --bin domers -- doctor --config examples/domers.toml --bind 127.0.0.1:3000
```

Then open `http://127.0.0.1:3000`, click `Start`, and confirm the metrics advance.

## Dependency Setup

Run the installer once on a new Linux development machine:

```sh
tools/install_dev_deps.sh
```

It installs apt packages for native Rust/Python audio builds, user-site Python
packages for Madmom, CPU Torch wheels, and an editable install of the sibling
Spectrum `Madmom` checkout. Use `SPECTRUM_REPO=/path/to/spectrum` if the legacy
checkout is not at `../spectrum`. Use `--check` for a read-only validation pass,
`--python-only` after system packages are already present, or `--system-only`
when you only want the apt packages.

## PR Full And Nightly

```sh
python3 tools/build_spectrum_csharp.py
python3 tools/capture_spectrum_visualizer_frames.py
make test-parity
```

`build_spectrum_csharp.py` is the Windows/.NET gate for executable Spectrum
fixture capture. It initializes the Spectrum `Madmom` submodule if needed and
builds `../spectrum/Spectrum/Spectrum.csproj` directly because the legacy
solution's `Madmom/Madmom.pyproj` requires Visual Studio Python Tools.
`capture_spectrum_visualizer_frames.py` executes the old Spectrum visualizers
headlessly with simulation-only output, and `check_visualizer_goldens.py` ensures all first-frame and sequence captured
hashes are present. Rust-vs-Spectrum visualizer hash tests pass for the full
captured manifests:

- **First-frame:** `rust_visualizer_hashes_match_spectrum_csharp_goldens` (via `make test-parity`)
- **Multi-frame:** `rust_visualizer_sequences_match_spectrum_csharp_goldens` (default `cargo test`)

Recapture sequence goldens on Windows/.NET with:

```sh
DOMERS_VISUALIZER_CASES=fixtures/spectrum-csharp/visualizer_sequence_cases.json python3 tools/capture_spectrum_visualizer_frames.py
```

Browser screenshots, load tests, and physical hardware sign-off artifacts are release evidence, not prerequisites for the no-hardware test suite.

## Manual Hardware Checklist

Use [`hardware-readiness.md`](hardware-readiness.md) for release sign-off.

- Dome flash-by-strut, strut iteration, strand test, and full-color flash.
- Bar control box 5 and runner/corner diagnostics.
- Stage side/layer diagnostics.
- OPC reconnect after interruption.
- MIDI board, audio volume, Madmom beat tracking, tap tempo, and orientation paintbrush.
