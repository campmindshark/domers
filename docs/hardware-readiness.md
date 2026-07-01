# Hardware Readiness Checklist

Hardware validation gates release tags. Complete this checklist after the automated CI suite is green.

## Preflight

- Confirm `make e2e` passes.
- Confirm `no_hardware_server_migration_and_simulator_smoke` passes.
- Confirm `hardware_outputs_send_mapped_dome_frame_to_loopback_opc` passes.
- Confirm `hardware_outputs_reconnect_after_loopback_opc_returns` passes.
- Confirm `runtime_udp_input_adapters_feed_live_state` passes.
- Confirm `make doctor CONFIG=domers.toml BIND=127.0.0.1:3000` passes with the hardware config.
- Record commit SHA, operator laptop OS, and fixture capture date.

## Dome

- Run flash-by-strut diagnostic and verify physical strut order.
- Run strut iteration diagnostic and verify control-box mapping.
- Run strand test diagnostic and verify strand direction.
- Run full-color flash diagnostic and verify brightness limits.
- Confirm buffer-based simulator modes run without OPC enabled.

## Bar

- Verify bar pixels route through dome control box 5.
- Run bar corner/runner diagnostic and verify reversal rules.

## Stage

- Run stage side/layer diagnostic.
- Verify all 48 sides and 3 layers match physical layout.

## Inputs

- Verify MIDI board palette, flash, tap tempo, and knob bindings.
- Verify audio volume input with the show audio device.
- Verify Madmom sidecar beat parsing against the selected input.
- Verify orientation devices on UDP port 5005.
- Verify the **Inputs** drawer shows audio, MIDI, orientation, and Madmom event counters increasing with no last error.

## Network And Recovery

- Verify OPC reconnect after controller restart or cable interruption.
- Verify the fixed **Runtime Status** footer shows the expected OPC addresses, `connected`, increasing frame counts, input event counters, and no last error while output is running.
- Verify server start/stop from browser UI.
- Verify simulator frame stream remains responsive during a 60-second run.

## Sign-Off

- Operator:
- Date:
- Notes:
