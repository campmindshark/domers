# Testing

The automated suite covers deterministic runtime behavior, protocol encoding, config migration, and the browser shell. Hardware validation is a release checklist item.

## PR Fast

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
node ui/check.mjs
```

## Covered

- TOML config round-trips and Spectrum XML import warnings.
- Scheduler priority rules: priority `0`, priority ties, diagnostics, disabled inputs, and output activation.
- OPC non-standard frame encoding and persistent sparse flush behavior.
- Simulator command emission with hardware disabled.
- Fake audio, MIDI, orientation, and Madmom beat inputs.
- Madmom sidecar launch argument and disabled-lifecycle behavior.
- Spectrum-compatible 64-entry palette indexing and gradient blending.
- Shared-entry palette TOML serialization, parsing, and XML import golden output.
- Visualizer simulator-frame harness and frame-hash snapshots for live dome modes.
- Server state contract for runtime config patching, palette patching, start/stop, metrics, input status, hardware status, and simulator frames.
- HTTP adapter smoke coverage for UI, state, geometry, mapping, and start routes.
- UI smoke markers for API/WebSocket wiring, pixel rendering, runtime palette controls, and simulator controls.

Tests for intentional behavior changes cite
[`intentional-deviations.md`](intentional-deviations.md) so preservation and
replacement decisions stay visible.

## Example Local Run

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
node ui/check.mjs
```

Live local smoke:

```sh
cargo run --bin domers -- run --config examples/domers.toml --bind 127.0.0.1:3000
```

Preflight the same config without starting outputs:

```sh
cargo run --bin domers -- doctor --config examples/domers.toml --bind 127.0.0.1:3000
```

Then open `http://127.0.0.1:3000`, click `Start`, and confirm the metrics advance.

## PR Full And Nightly

Next additions: Docker Compose OPC loopback services, fake orientation sender services, browser automation, load tests, and physical hardware sign-off artifacts.

## Manual Hardware Checklist

Use [`hardware-readiness.md`](hardware-readiness.md) for release sign-off.

- Dome flash-by-strut, strut iteration, strand test, and full-color flash.
- Bar control box 5 and runner/corner diagnostics.
- Stage side/layer diagnostics.
- OPC reconnect after interruption.
- MIDI board, audio volume, Madmom beat tracking, tap tempo, and orientation paintbrush.

## TODO Images

TODO: Add image of a passing GitHub Actions run.

- Capture: Actions page for `campmindshark/dome-rs` with `rust-fast`, `docs-and-ui-smoke`, and `docker-loopback-placeholder` green.
- Expected: latest `main` run is green.
- Suggested file: `docs/images/testing-github-actions-green.png`.

TODO: Add image of local test output.

- Capture: terminal after running the PR fast commands.
- Expected: cargo tests, clippy, and UI smoke all pass.
- Suggested file: `docs/images/testing-local-fast-gate.png`.
