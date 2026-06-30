# Spectrum Parity Closure

This file is the working parity ledger. A feature is closed only when the Rust
implementation has code, automated evidence, and, where relevant, hardware
evidence. Green tests alone do not imply full Spectrum parity.

## Current Verdict

`dome-rs` is not yet a full Spectrum replacement if exact visualizer frame
equivalence and physical hardware sign-off are required. For the first
no-hardware version, visualizer frame equivalence and physical LED access are
explicitly out of scope. The remaining operator/runtime surface is now covered by
no-hardware gates for input behavior, config editing, MIDI bindings/logs,
orientation state, Madmom fake-sidecar ingestion, and dome/bar/stage simulator
streams.

## Closure Matrix

| Area | Current status | Closure gate |
| --- | --- | --- |
| Visualizer inventory | 17 used Spectrum visualizer names are tracked and dispatched. | Keep `INVENTORY` and `fixtures/spectrum-csharp/visualizer_frame_cases.json` in lockstep. |
| Visualizer frame parity | Source hashes exist, but expected frame hashes are pending C# execution. | Every visualizer case has a non-null Spectrum frame hash and a Rust golden comparison. |
| Dome visualizer algorithms | Several modes are deterministic approximations. | Volume, Radial, Race, Snakes, Splat, Quaternion, Paintbrush, TV Static, and Flash match Spectrum frame goldens or documented deviations. |
| Diagnostics | Dome/bar/stage diagnostics are wired. | C# frame goldens plus physical dome/bar/stage diagnostic sign-off. |
| Audio input | UDP volume bridge is the no-hardware replacement for native device capture in this first version. | Native audio device selection remains hardware/OS integration work; no-hardware parity uses the documented UDP adapter. |
| MIDI input | UDP command transport feeds configurable bindings, runtime actions, and a MIDI log. | Hardware device discovery is out of scope for the first no-hardware version; binding behavior is covered by tests. |
| Orientation input | Datagram parsing, device map, quaternion state, calibration, action flags, poi speed, and stale-device removal are implemented. | Visualizer-specific orientation frame equivalence remains tied to visualizer parity. |
| Madmom | Launch args support wrapper and Python-style tracker invocation; runtime uses async child ingestion and fake-sidecar tests. | Bundled Windows package discovery remains deployment work; sidecar protocol/lifecycle is covered without hardware. |
| Beat timing | Tap tempo and Madmom median/backwards reset are covered. | Add fixtures for long gap, backwards, double-report, missed-beat, and progress phase behavior. |
| Operator UI | Browser shell has core controls, full config editor, full palette editor, input status, MIDI log, orientation calibration, debug visuals, and preview. | Browser automation/screenshots remain follow-up evidence. |
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
