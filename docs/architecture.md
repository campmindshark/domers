# Architecture

`dome-rs` is a headless Rust lighting control runtime with browser controls and simulator views.

## Crates

- `domers-core`: shared colors, Spectrum palette semantics, beat timing, config types, TOML config import, and migration warnings.
- `domers-engine`: scheduler and frame orchestration.
- `domers-outputs`: dome/bar/stage commands, topology, simulator sinks, and OPC encoding.
- `domers-inputs`: Madmom protocol parsing and sidecar launch wrapper, MIDI replay, audio replay, and orientation datagram classification.
- `domers-visualizers`: visualizer inventory and deterministic simulator frame harness.
- `domers-server`: HTTP/WebSocket contract surface and state semantics.
- `domers-test-support`: fake clocks and deterministic test utilities.

## Runtime Shape

```text
Browser UI -> Server contract -> Engine scheduler -> Inputs + Visualizers -> Outputs
                                                        |                  |
                                                        |                  +-> OPC hardware
                                                        +--------------------> Simulator frames
```

The browser simulator is driven by engine frame data. It does not read back from OPC hardware sockets.

## Runtime Surface

The server crate implements both the in-process `ServerState` contract and the runnable HTTP/WebSocket adapter. The `domers` binary loads TOML config, serves the browser shell, exposes JSON API endpoints, and streams simulator frames over WebSocket.

HTTP and WebSocket surface:

- `GET /`: browser operator shell
- `GET /simulator`: dedicated simulator page
- `GET /main.mjs`: browser control script
- `GET /api/health`: health JSON
- `GET /api/state`: running state, engine config, simulator inputs, and metrics
- `POST /api/start`: start the engine loop
- `POST /api/stop`: stop the engine loop
- `PATCH /api/config/dome`: patch runtime dome controls: active visualizer, flash speed, and palette slot
- `PATCH /api/config/palette`: patch one runtime palette color in the active palette bank
- `GET /api/dome/geometry`: Spectrum-derived dome projection geometry
- `GET /api/dome/mapping`: Spectrum-derived dome strut/LED mapping
- `PATCH /api/simulator`: patch simulator-only preview inputs: volume, beat phase, and flash-active state
- `GET /api/simulator/frame`: produce one simulator frame
- `GET /ws/simulator`: simulator frame and metrics stream

## Timing Contracts

- Engine target: 400 Hz compute cap.
- OPC target: independent 200 Hz send cap.
- Browser simulator: throttled stream derived from engine frames.

Timing tests use this shape:

```text
fake clock -> scheduler frame -> visualizer render -> simulator frame -> metrics update
```

## State And Concurrency

Each engine frame uses a stable config snapshot plus a drained batch of input/control events. Browser config edits, MIDI commands, audio samples, orientation datagrams, and Madmom beat reports enter through explicit event paths instead of mutating shared UI state mid-frame.

Stress tests cover:

- config updates during frame production
- MIDI replay during visualizer rendering
- simulator frame production during input bursts
- metrics updates after each frame

## Configuration

`dome-rs` native configuration is TOML. Runtime code loads TOML, not XML. Legacy Spectrum XML is handled only by the import command documented in [`configuration.md`](configuration.md).

## Simulator Preview

The live control page keeps simulator work lazy. It fetches runtime state on load, then starts geometry/mapping requests, one preview frame request, and the simulator WebSocket only when the `Preview` drawer opens. The dedicated `/simulator` page starts the simulator immediately.

## Beat Input

The beat engine accepts beat events from tap tempo, fake tests, and Madmom-compatible `BEAT:{seconds}` lines. `domers-inputs` includes a managed sidecar wrapper for the Spectrum launch contract. Server-side wiring from that child process into live beat state is separate runtime work.

## Intentional Deviations

Spectrum compatibility decisions and explicit differences are tracked in [`intentional-deviations.md`](intentional-deviations.md). Keep this file focused on `dome-rs` architecture; put historical comparisons and deliberate departures there.

## TODO Images

TODO: Add architecture diagram image.

- Capture: rendered diagram of browser UI, server, engine, inputs, visualizers, outputs, OPC, and simulator frame stream.
- Expected: clearly shows simulator frames are separate from OPC hardware output.
- Suggested file: `docs/images/architecture-runtime-flow.png`.

TODO: Add screenshot of server metrics.

- Capture: browser/devtool/API output showing frame counters and simulator frame counters.
- Expected: `frames` and `simulator_frames` are visible.
- Suggested file: `docs/images/architecture-server-metrics.png`.
