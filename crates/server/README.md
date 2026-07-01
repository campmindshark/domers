# domers-server

Runnable operator runtime and HTTP/WebSocket server.

## Responsibilities

- Own `ServerState` and `AppRuntime` lifecycle for engine, inputs, simulator frames, and hardware outputs.
- Serve the browser control page, simulator page, API routes, and WebSocket simulator stream.
- Expose runtime snapshots for config, metrics, hardware status, input status, MIDI logs, orientation devices, and simulator controls.
- Manage live UDP input tasks, optional native capture tasks, Madmom sidecar ingestion, and DJ Link/Carabiner tempo ingestion.
- Send scheduled dome, bar, and stage frames to OPC targets with loopback-testable reconnect behavior.

## Key Files

- `src/lib.rs`: runtime state, routes, frame rendering, input tasks, hardware output, and integration tests.
- `Cargo.toml`: Axum, Tokio, serde, and workspace crate dependencies.

## Tests

Run this crate with:

```sh
cargo test -p domers-server
```
