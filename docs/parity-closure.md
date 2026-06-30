# Spectrum Parity Closure

This file is the working parity ledger. A feature is closed only when the Rust
implementation has code, automated evidence, and, where relevant, hardware
evidence. Green tests alone do not imply full Spectrum parity.

## Current Verdict

`dome-rs` is expected to close all parity that can be proven without physical
hardware. Spectrum C# execution is now part of the parity process: visualizer
frame hashes are captured by a headless Windows/.NET runner that bypasses WPF
and hardware output. The only accepted deferral is physical hardware acceptance.

## Closure Matrix

| Area | Current status | Closure gate |
| --- | --- | --- |
| Visualizer inventory | 17 used Spectrum visualizer names are tracked and dispatched. | Keep `INVENTORY` and `fixtures/spectrum-csharp/visualizer_frame_cases.json` in lockstep. |
| Visualizer frame parity | All 17 tracked cases have headless Spectrum C# frame hashes in `visualizer_frame_cases.json`. | Every visualizer case keeps a non-null Spectrum frame hash and gains a Rust golden comparison. |
| Dome visualizer algorithms | Several modes are deterministic approximations and are not yet expected to match the newly captured Spectrum hashes. | Volume, Radial, Race, Snakes, Splat, Quaternion, Paintbrush, TV Static, and Flash match Spectrum frame goldens or documented deviations. |
| Diagnostics | Dome/bar/stage diagnostics are wired. | C# frame goldens plus physical dome/bar/stage diagnostic sign-off. |
| Audio input | UDP volume bridge, Spectrum audio device identity, all-endpoint index mapping, XML import, and Madmom audio-index derivation are covered. | Windows native capture can be added when OS access is needed; device semantics are represented and tested. |
| MIDI input | UDP command transport feeds device-scoped state, configurable wildcard/exact bindings, runtime actions, knob/note defaults, Spectrum knob math, and a MIDI log. | Physical controller discovery is hardware acceptance; no-hardware MIDI behavior must remain tested. |
| Orientation input | Datagram parsing, device map, quaternion state, calibration, action flags, poi speed, and stale-device removal are implemented. | Visualizer-specific orientation frame equivalence remains tied to visualizer parity. |
| Madmom | Launch args support wrapper, Spectrum-style Python working directory, async child ingestion, derived audio input indexes, and fake-sidecar tests are covered. | Shipping a bundled Madmom distribution is release packaging; runtime behavior is no-hardware tested. |
| Beat timing | Tap tempo, BPM string, tap counter, reset, Madmom median/backwards reset, and Spectrum truncating progress math are covered. | Any further mismatch must be backed by old Spectrum execution evidence. |
| Operator UI | Browser shell has structured input/tempo/Madmom and output/layout config controls, full config editor, full palette editor, input status, MIDI log, orientation calibration, debug visuals, and preview. | Browser automation/screenshots are evidence work, not feature parity deferral. |
| Simulators | Dome canvas plus bar/stage command previews are exposed on the live preview and sandbox page. | Exact visual artwork remains visualizer/UI polish, not first-version parity. |
| Hardware output | OPC mapping/write/reconnect loopback tests pass. | Physical dome, bar, stage, inputs, and reconnect sign-off are intentionally deferred. |

## Execution Order

1. Keep no-hardware runtime and UI gates green.
2. Port visualizer algorithms against the captured Spectrum C# frame hashes.
3. Add browser automation/screenshots for the now-wired operator panels.
4. When hardware is available, run physical sign-off.

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

The flash-overlay visualizer currently captures an empty no-MIDI-trigger frame;
the next exactness slice should add fixture input for a MIDI flash event before
using that case as an algorithm comparison.
