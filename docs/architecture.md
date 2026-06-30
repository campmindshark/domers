# Architecture

Domers is structured as a headless Rust engine with browser control and simulator views.

## Crates

- `domers-core`: shared colors, beat timing, config types.
- `domers-engine`: scheduler and frame orchestration.
- `domers-outputs`: dome/bar/stage commands, topology, OPC encoding.
- `domers-inputs`: Madmom, MIDI, audio, and orientation seams.
- `domers-visualizers`: visualizer inventory and eventual ports.
- `domers-server`: HTTP/WebSocket API.
- `domers-test-support`: fake clocks and no-hardware test utilities.

## Timing Contracts

- Engine target: Spectrum-compatible 400 Hz compute cap.
- OPC target: independent 200 Hz send cap.
- Browser simulator: throttled stream derived from engine frames, not hardware sockets.

## Concurrency Contract

The Rust implementation should prefer frame-local config snapshots and event channels over shared mutable UI state. Stress tests must cover concurrent config updates, MIDI replay, and visualizer frames.
