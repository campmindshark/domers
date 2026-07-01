# domers-inputs

Fakeable input protocol and state helpers for no-hardware parity tests.

## Responsibilities

- Parse live UDP audio volume payloads and model Spectrum audio device identity/index behavior.
- Parse and replay MIDI commands, including device-scoped command state and Spectrum knob math.
- Parse Madmom `BEAT:{seconds}` lines and construct Spectrum-compatible sidecar launch arguments.
- Parse DJ Link / Carabiner-compatible tempo lines.
- Parse Spectrum orientation datagrams and maintain orientation device state, calibration, action flags, poi speed, and stale removal.

## Key Files

- `src/audio.rs`: volume parsing, audio endpoint flow, capture-device filtering, and Madmom index lookup.
- `src/midi.rs`: MIDI payload parsing, replay, state, logs, and knob mapping helpers.
- `src/madmom.rs`: sidecar launch config, lifecycle helper, and beat-line parser.
- `src/link.rs`: DJ Link/Carabiner tempo-line parser.
- `src/orientation.rs`: datagram parser, quaternion/device model, and orientation input state.

## Tests

Run this crate with:

```sh
cargo test -p domers-inputs
```
