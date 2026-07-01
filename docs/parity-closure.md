# Spectrum Parity Closure

This file is the working parity ledger. A feature is closed only when the Rust
implementation has code, automated evidence, and, where relevant, hardware
evidence. Green tests alone do not imply full Spectrum parity.

## Current Verdict

**Automated visualizer parity: 17/17 first-frame goldens and 11/11 multi-frame
sequence goldens.** `dome-rs` closes parity with automated evidence wherever
physical hardware is not required. Spectrum C# execution is part of the parity
process: visualizer frame hashes are captured by a headless Windows/.NET runner
that bypasses WPF and hardware output. Physical hardware acceptance is tracked
separately.

## Scorecard

| Golden suite | Manifest | Cases | Rust test |
| --- | --- | ---: | --- |
| First-frame | `fixtures/spectrum-csharp/visualizer_frame_cases.json` | 17/17 captured | `rust_visualizer_hashes_match_spectrum_csharp_goldens` (run via `make test-parity` / `make e2e`) |
| Multi-frame | `fixtures/spectrum-csharp/visualizer_sequence_cases.json` | 11/11 captured, 0 pending | `rust_visualizer_sequences_match_spectrum_csharp_goldens` (default `cargo test`) |

## Closure Matrix

| Area | Current status | Closure gate |
| --- | --- | --- |
| Visualizer inventory | 17 used Spectrum visualizer names are tracked and dispatched. | Keep `INVENTORY` and `fixtures/spectrum-csharp/visualizer_frame_cases.json` in lockstep. |
| Visualizer first-frame parity | All 17 tracked cases have headless Spectrum C# frame hashes in `visualizer_frame_cases.json`; the manifest requires a concrete expected value for every case. | Rust-rendered first-frame hashes match the captured Spectrum hashes, or a mismatch is recorded as an intentional deviation. |
| Visualizer runtime parity | All 11 live dome multi-frame sequences in `visualizer_sequence_cases.json` are captured and matched by `rust_visualizer_sequences_match_spectrum_csharp_goldens`. | Stateful Rust renderers match captured multi-frame Spectrum sequences, including sparse pixel persistence, beat/audio/orientation input, and visualizer switch behavior. |
| Dome visualizer algorithms | Renderers are wired for all used dome modes and consume full active palette banks. Captured first-frame and multi-frame Spectrum C# visualizer goldens match the Rust renderer. | Keep manifests and `INVENTORY` in lockstep when adding or changing visualizers. |
| Diagnostics | Dome/bar/stage diagnostics are wired; throttled at 1 Hz like Spectrum stopwatches. | C# frame goldens plus physical dome/bar/stage diagnostic sign-off. |
| Audio input | UDP volume bridge, native CPAL capture behind the `native-capture` build feature, Spectrum audio device identity, all-endpoint index mapping, XML import, audio level-driver preset/channel import, and Madmom audio-index derivation are covered. | Physical show-device sign-off validates real capture devices and levels. |
| MIDI input | UDP command transport and native midir capture behind the `native-capture` build feature feed device-scoped state, configurable wildcard/exact bindings, runtime actions, knob/note defaults, Spectrum knob math, ADSR level-driver bindings, and a MIDI log. | Physical controller discovery/sign-off remains hardware acceptance. |
| Orientation input | Datagram parsing, device map, quaternion state, calibration, action flags, poi speed, stale-device removal, and live operator wiring into `VisualizerInput.orientation_devices` are implemented. | Quaternion visualizer frame equivalence is covered by captured sequence goldens. |
| Madmom | Launch args support wrapper, Spectrum-style Python working directory, async child ingestion, derived audio input indexes, and fake-sidecar tests are covered. | Shipping a bundled Madmom distribution is release packaging; runtime behavior is no-hardware tested. |
| Beat timing | Wall-clock tap tempo with duplicate touch/click filtering, BPM string, tap counter, reset, Madmom median/backwards reset, DJ Link/Carabiner sidecar tempo ingestion, and Spectrum truncating progress math are covered. | Packaged macOS/Linux DJ Link sidecar remains release integration work. |
| Operator UI | Browser shell has structured input/tempo/Madmom and output/layout config controls, full config editor, full palette editor, input status, MIDI log, orientation calibration, debug visuals, preview, and no-hardware HTTP route coverage for operator flows. | Browser screenshots remain release evidence, not feature parity deferral. |
| Simulators | The live preview shows hardware-bound output only and drives visualizer animation at the emitted preview cadence, not the 400 Hz engine compute cadence. The isolated simulator exposes animation/testing controls, including yaw/pitch/roll overrides, plus dome/bar/stage command previews. | Exact visual artwork remains visualizer/UI polish, not first-version parity. |
| Hardware output | OPC mapping/write/reconnect loopback tests pass. | Physical dome, bar, stage, inputs, and reconnect sign-off are intentionally deferred. |

## Closed (automated)

- **First-frame goldens:** `rust_visualizer_hashes_match_spectrum_csharp_goldens` passes for all 17 tracked cases.
- **Multi-frame goldens:** `rust_visualizer_sequences_match_spectrum_csharp_goldens` passes for all 11 captured live dome sequences (Volume, Radial, Race, Snakes, Splat, TV Static, Quaternion Test/Multi/Paintbrush, Flash, plus Stage Depth on stage output).
- **Stateful runtimes:** `VisualizerRuntime` keeps per-mode state across frames (Snakes, Race, Radial, Splat, TV Static, Volume, Flash, Quaternion Paintbrush) with Spectrum-style deltas, switch-wipe on visualizer change, and server-side dome buffer clearing.
- **Stage Depth:** Rust matches the captured `LEDStageDepthLevelVisualizer` sequence golden.

## Open follow-ups

- **Dome tuning knobs:** Spectrum `domeGlobalFadeSpeed`, `domeRadialSize`, ripple steps, and related XML fields remain hardcoded at Spectrum defaults in the visualizer port; wiring them through `DomersConfig` is deferred until the config schema grows those fields.
- **Hardware acceptance:** Physical dome/bar/stage OPC, inputs, and diagnostic sign-off remain out of scope for automated closure.

## Required Gates

```sh
make e2e
```

This runs the full workspace test suite, lint, UI smoke, manifest completeness
check, and the first-frame Spectrum golden test. The multi-frame sequence
golden test runs as part of `cargo test --workspace` inside `make e2e`.

Run parity tests alone:

```sh
make test-parity
cargo test -p domers-visualizers rust_visualizer_sequences_match_spectrum_csharp_goldens
```

Refresh visualizer goldens with:

```sh
python3 tools/capture_spectrum_visualizer_frames.py
```

Refresh multi-frame sequence goldens with:

```sh
DOMERS_VISUALIZER_CASES=fixtures/spectrum-csharp/visualizer_sequence_cases.json python3 tools/capture_spectrum_visualizer_frames.py
```
