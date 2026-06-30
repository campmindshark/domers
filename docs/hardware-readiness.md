# Hardware Readiness Checklist

Hardware validation gates release tags. Complete this checklist after the automated CI suite is green.

## Preflight

- Confirm `cargo test --workspace` passes.
- Confirm `cargo clippy --workspace --all-targets -- -D warnings` passes.
- Confirm UI smoke check passes.
- Confirm `no_hardware_server_migration_and_simulator_smoke` passes.
- Confirm `domers doctor --config domers.toml --bind 127.0.0.1:3000` passes with the hardware config.
- Record commit SHA, operator laptop OS, and fixture capture date.

## Dome

- Run flash-by-strut diagnostic and verify physical strut order.
- Run strut iteration diagnostic and verify control-box mapping.
- Run strand test diagnostic and verify strand direction.
- Run full-color flash diagnostic and verify brightness limits.
- Confirm buffer-based simulator modes run without OPC enabled.

TODO: Add image of expected dome full-color flash.

- Capture: physical dome or simulator during full-color flash diagnostic.
- Expected: all expected dome LEDs lit consistently.
- Suggested file: `docs/images/readiness-dome-full-color-flash.png`.

## Bar

- Verify bar pixels route through dome control box 5.
- Run bar corner/runner diagnostic and verify reversal rules.

TODO: Add image of expected bar diagnostic.

- Capture: bar corner/runner diagnostic.
- Expected: corners, runner, and reversal direction are identifiable.
- Suggested file: `docs/images/readiness-bar-diagnostic.png`.

## Stage

- Run stage side/layer diagnostic.
- Verify all 48 sides and 3 layers match physical layout.

TODO: Add image of expected stage side/layer diagnostic.

- Capture: stage during side/layer diagnostic.
- Expected: side index/layer mapping is clear.
- Suggested file: `docs/images/readiness-stage-side-layer.png`.

## Inputs

- Verify MIDI board palette, flash, tap tempo, and knob bindings.
- Verify audio volume input with the show audio device.
- Verify Madmom sidecar beat parsing against the selected input.
- Verify orientation devices on UDP port 5005.

TODO: Add image of expected input status panel.

- Capture: UI/API output showing MIDI/audio/Madmom/orientation status.
- Expected: each input source has a clear connected/active state before physical output is trusted.
- Suggested file: `docs/images/readiness-input-status.png`.

## Network And Recovery

- Verify OPC reconnect after controller restart or cable interruption.
- Verify the controls page **OPC Targets** panel shows the expected addresses, `connected`, increasing frame counts, and no last error while output is running.
- Verify server start/stop from browser UI.
- Verify simulator frame stream remains responsive during a 60-second run.

TODO: Add image of expected server/recovery status.

- Capture: UI/API output after reconnect test.
- Expected: OPC reconnect is visible in status or logs.
- Suggested file: `docs/images/readiness-opc-reconnect.png`.

## Sign-Off

- Operator:
- Date:
- Notes:
