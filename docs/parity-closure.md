# Spectrum Parity Closure

This file is the working parity ledger. A feature is closed only when the Rust
implementation has code, automated evidence, and, where relevant, hardware
evidence. Green tests alone do not imply full Spectrum parity.

## Current Verdict

`dome-rs` is expected to close all parity that can be proven without physical
hardware or executing the old Spectrum app. The only accepted deferrals are
physical hardware acceptance and evidence that requires running Spectrum on
Windows. The no-hardware operator/runtime surface is covered by gates for input
behavior, config editing, MIDI state/bindings/logs, orientation state, Madmom
fake-sidecar ingestion, tempo surfaces, and dome/bar/stage simulator streams.

## Closure Matrix

| Area | Current status | Closure gate |
| --- | --- | --- |
| Visualizer inventory | 17 used Spectrum visualizer names are tracked and dispatched. | Keep `INVENTORY` and `fixtures/spectrum-csharp/visualizer_frame_cases.json` in lockstep. |
| Visualizer frame parity | Source hashes exist, but expected frame hashes are pending C# execution. | Every visualizer case has a non-null Spectrum frame hash and a Rust golden comparison. |
| Dome visualizer algorithms | Several modes are deterministic approximations. | Volume, Radial, Race, Snakes, Splat, Quaternion, Paintbrush, TV Static, and Flash match Spectrum frame goldens or documented deviations. |
| Diagnostics | Dome/bar/stage diagnostics are wired. | C# frame goldens plus physical dome/bar/stage diagnostic sign-off. |
| Audio input | UDP volume bridge, Spectrum audio device identity, all-endpoint index mapping, XML import, and Madmom audio-index derivation are covered. | Windows native capture can be added when OS access is needed; device semantics are represented and tested. |
| MIDI input | UDP command transport feeds device-scoped state, configurable wildcard/exact bindings, runtime actions, knob/note defaults, Spectrum knob math, and a MIDI log. | Physical controller discovery is hardware acceptance; no-hardware MIDI behavior must remain tested. |
| Orientation input | Datagram parsing, device map, quaternion state, calibration, action flags, poi speed, and stale-device removal are implemented. | Visualizer-specific orientation frame equivalence remains tied to visualizer parity. |
| Madmom | Launch args support wrapper and Python-style tracker invocation; runtime uses async child ingestion, derived audio input indexes, and fake-sidecar tests. | Old bundled Windows discovery can only be proven when running Spectrum/Windows packaging. |
| Beat timing | Tap tempo, BPM string, tap counter, reset, Madmom median/backwards reset, and Spectrum truncating progress math are covered. | Any further mismatch must be backed by old Spectrum execution evidence. |
| Operator UI | Browser shell has structured input/tempo/Madmom and output/layout config controls, full config editor, full palette editor, input status, MIDI log, orientation calibration, debug visuals, and preview. | Browser automation/screenshots are evidence work, not feature parity deferral. |
| Simulators | Dome canvas plus bar/stage command previews are exposed on the live preview and sandbox page. | Exact visual artwork remains visualizer/UI polish, not first-version parity. |
| Hardware output | OPC mapping/write/reconnect loopback tests pass. | Physical dome, bar, stage, inputs, and reconnect sign-off are intentionally deferred. |

## Execution Order

1. Keep no-hardware runtime and UI gates green.
2. Add browser automation/screenshots for the now-wired operator panels.
3. When hardware is available, run physical sign-off.
4. Separately, when visualizer equivalence matters, capture Spectrum C# goldens and port against them.

## Required Gates

```sh
cargo test --workspace
node ui/check.mjs
cargo clippy --workspace --all-targets -- -D warnings
python3 tools/check_visualizer_goldens.py
```

The visualizer golden check is expected to fail until the Spectrum C# capture has
been run and the pending frame hashes have been filled in.
