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

## Timing Contracts

- Engine target: 400 Hz compute cap.
- OPC target: independent 200 Hz send cap.
- Browser simulator: throttled stream derived from engine frames.

Example future timing test shape:

```text
fake clock -> scheduler frame -> visualizer render -> simulator frame -> metrics update
```

## State And Concurrency

The engine should process each frame from a stable config snapshot plus a drained batch of input/control events. Browser config edits, MIDI commands, audio samples, orientation datagrams, and Madmom beat reports should enter through explicit event paths instead of mutating shared UI state mid-frame.

Stress tests should cover:

- config updates during frame production
- MIDI replay during visualizer rendering
- simulator frame production during input bursts
- metrics updates after each frame

## Configuration

Domers-native configuration is TOML. Runtime code should load TOML, not XML. Legacy Spectrum XML is handled only by the import command documented in [`configuration.md`](configuration.md).

## Beat Input

The beat engine accepts beat events from tap tempo, fake tests, and the configurable Madmom-compatible sidecar protocol. The architecture depends on beat events, not on a specific Python installation path.

## Intentional Deviations

Spectrum compatibility decisions and explicit differences are tracked in [`intentional-deviations.md`](intentional-deviations.md). Keep this file focused on Domers architecture; put historical comparisons and deliberate departures there.

## TODO Images

TODO: Add architecture diagram image.

- Capture: rendered diagram of browser UI, server, engine, inputs, visualizers, outputs, OPC, and simulator frame stream.
- Expected: clearly shows simulator frames are separate from OPC hardware output.
- Suggested file: `docs/images/architecture-runtime-flow.png`.

TODO: Add screenshot of server metrics once the HTTP/WebSocket adapter exists.

- Capture: browser/devtool/API output showing frame counters and simulator frame counters.
- Expected: `frames` and `simulator_frames` are visible.
- Suggested file: `docs/images/architecture-server-metrics.png`.
