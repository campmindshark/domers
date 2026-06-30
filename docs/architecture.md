# Architecture

Domers is a headless Rust engine with browser control and simulator views.

## Crates

- `domers-core`: shared colors, beat timing, config types, TOML config import, and migration warnings.
- `domers-engine`: scheduler and frame orchestration.
- `domers-outputs`: dome/bar/stage commands, topology, simulator sinks, and OPC encoding.
- `domers-inputs`: Madmom, MIDI, audio, and orientation seams.
- `domers-visualizers`: visualizer inventory and deterministic simulator frame harness.
- `domers-server`: HTTP/WebSocket contract surface and state semantics.
- `domers-test-support`: fake clocks and no-hardware test utilities.

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

Network surface:

- `GET /`: browser operator shell
- `GET /main.mjs`: browser control script
- `GET /api/health`: health JSON
- `GET /api/state`: running state, engine config, and metrics
- `POST /api/start`: start the engine loop
- `POST /api/stop`: stop the engine loop
- `PATCH /api/config/dome`: patch dome visualizer config
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

Domers-native configuration is TOML. Runtime code loads TOML, not XML. Legacy Spectrum XML is handled only by the import command documented in [`configuration.md`](configuration.md).

## Beat Input

The beat engine accepts beat events from tap tempo, fake tests, and the Madmom-compatible sidecar. Domers owns sidecar lifecycle and consumes beat events; packaging decides whether the sidecar is a bundled Python environment, wrapper script, system install, Docker launcher, or native replacement.

## Intentional Deviations

Spectrum compatibility decisions and explicit differences are tracked in [`intentional-deviations.md`](intentional-deviations.md). Keep this file focused on Domers architecture; put historical comparisons and deliberate departures there.

## TODO Images

TODO: Add architecture diagram image.

- Capture: rendered diagram of browser UI, server, engine, inputs, visualizers, outputs, OPC, and simulator frame stream.
- Expected: clearly shows simulator frames are separate from OPC hardware output.
- Suggested file: `docs/images/architecture-runtime-flow.png`.

TODO: Add screenshot of server metrics.

- Capture: browser/devtool/API output showing frame counters and simulator frame counters.
- Expected: `frames` and `simulator_frames` are visible.
- Suggested file: `docs/images/architecture-server-metrics.png`.
