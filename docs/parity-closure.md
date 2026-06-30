# Spectrum Parity Closure

This file is the working parity ledger. A feature is closed only when the Rust
implementation has code, automated evidence, and, where relevant, hardware
evidence. Green tests alone do not imply full Spectrum parity.

## Current Verdict

`dome-rs` closes parity with automated evidence wherever physical hardware is
not required. Spectrum C# execution is part of the parity process: visualizer
frame hashes are captured by a headless Windows/.NET runner that bypasses WPF
and hardware output. Physical hardware acceptance is tracked separately.

## Closure Matrix

| Area | Current status | Closure gate |
| --- | --- | --- |
| Visualizer inventory | 17 used Spectrum visualizer names are tracked and dispatched. | Keep `INVENTORY` and `fixtures/spectrum-csharp/visualizer_frame_cases.json` in lockstep. |
| Visualizer frame parity | All 17 tracked cases have headless Spectrum C# frame hashes in `visualizer_frame_cases.json`; the manifest requires a concrete expected value for every case. | Rust-rendered hashes match the captured Spectrum hashes, or a mismatch is recorded as an intentional deviation. |
| Dome visualizer algorithms | Renderers are wired for all used dome modes and consume full active palette banks. | Volume, Radial, Race, Snakes, Splat, Quaternion, Paintbrush, TV Static, and Flash match Spectrum frame goldens or documented deviations. |
| Diagnostics | Dome/bar/stage diagnostics are wired. | C# frame goldens plus physical dome/bar/stage diagnostic sign-off. |
| Audio input | UDP volume bridge, native CPAL capture behind the `native-capture` build feature, Spectrum audio device identity, all-endpoint index mapping, XML import, audio level-driver preset/channel import, and Madmom audio-index derivation are covered. | Physical show-device sign-off validates real capture devices and levels. |
| MIDI input | UDP command transport and native midir capture behind the `native-capture` build feature feed device-scoped state, configurable wildcard/exact bindings, runtime actions, knob/note defaults, Spectrum knob math, ADSR level-driver bindings, and a MIDI log. | Physical controller discovery/sign-off remains hardware acceptance. |
| Orientation input | Datagram parsing, device map, quaternion state, calibration, action flags, poi speed, and stale-device removal are implemented. | Visualizer-specific orientation frame equivalence remains tied to visualizer parity. |
| Madmom | Launch args support wrapper, Spectrum-style Python working directory, async child ingestion, derived audio input indexes, and fake-sidecar tests are covered. | Shipping a bundled Madmom distribution is release packaging; runtime behavior is no-hardware tested. |
| Beat timing | Wall-clock tap tempo, BPM string, tap counter, reset, Madmom median/backwards reset, Link/Carabiner sidecar tempo ingestion, and Spectrum truncating progress math are covered. | Packaged macOS/Linux Link sidecar remains release integration work. |
| Operator UI | Browser shell has structured input/tempo/Madmom and output/layout config controls, full config editor, full palette editor, input status, MIDI log, orientation calibration, debug visuals, preview, and no-hardware HTTP route coverage for operator flows. | Browser screenshots remain release evidence, not feature parity deferral. |
| Simulators | Dome canvas plus bar/stage command previews are exposed on the live preview and sandbox page. | Exact visual artwork remains visualizer/UI polish, not first-version parity. |
| Hardware output | OPC mapping/write/reconnect loopback tests pass. | Physical dome, bar, stage, inputs, and reconnect sign-off are intentionally deferred. |

## Open TODOs

- Stage Depth exactness: the Rust renderer now uses Spectrum palette gradients,
  stage brightness, truncating RGB scale math, and the captured stage command
  order, but its hash still differs from the captured
  `LEDStageDepthLevelVisualizer` golden. Leave this as an explicit visualizer
  parity TODO until the remaining difference in tracer timing, side partitioning,
  or color interpolation is isolated.

## Required Gates

```sh
cargo test --workspace
node ui/check.mjs
cargo clippy --workspace --all-targets -- -D warnings
python3 tools/check_visualizer_goldens.py
```

Refresh visualizer goldens with:

```sh
python3 tools/capture_spectrum_visualizer_frames.py
```
