# Testing

Every ordinary test must run without lighting hardware. Hardware validation gates releases, not normal PRs.

## PR Fast

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
node ui/check.mjs
```

## What The Current Suite Covers

- TOML config round-trips and Spectrum XML import warnings.
- Scheduler priority rules: priority `0`, priority ties, diagnostics, disabled inputs, and output activation.
- OPC non-standard frame encoding and persistent sparse flush behavior.
- Simulator command emission with hardware disabled.
- Fake audio, MIDI, orientation, and Madmom beat inputs.
- Visualizer simulator-frame harness for initial live dome modes.
- Server state contract for config patching, start/stop, metrics, and simulator frames.
- UI smoke markers for the initial browser shell.

Tests for intentional behavior changes should cite
[`intentional-deviations.md`](intentional-deviations.md) so it is clear whether
Domers is preserving Spectrum behavior or deliberately replacing it.

## Example Local Run

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
node ui/check.mjs
```

## PR Full And Nightly

Later increments add deeper fixture golden tests, real Docker Compose OPC loopback services, fake Madmom sidecar process tests, fake orientation sender services, visualizer frame hash snapshots, server e2e, UI e2e, and load tests.

## Manual Hardware Checklist

Use [`hardware-readiness.md`](hardware-readiness.md) for release sign-off.

- Dome flash-by-strut, strut iteration, strand test, and full-color flash.
- Bar control box 5 and runner/corner diagnostics.
- Stage side/layer diagnostics.
- OPC reconnect after interruption.
- MIDI board, audio volume, Madmom beat tracking, tap tempo, and orientation paintbrush.

## TODO Images

TODO: Add image of a passing GitHub Actions run.

- Capture: Actions page for `campmindshark/domers` with `rust-fast`, `docs-and-ui-smoke`, and `docker-loopback-placeholder` green.
- Expected: latest `main` run is green.
- Suggested file: `docs/images/testing-github-actions-green.png`.

TODO: Add image of local test output.

- Capture: terminal after running the PR fast commands.
- Expected: cargo tests, clippy, and UI smoke all pass.
- Suggested file: `docs/images/testing-local-fast-gate.png`.
