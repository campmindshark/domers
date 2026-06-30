# Testing

Every ordinary test must run without lighting hardware. Hardware validation gates releases, not normal PRs.

## PR Fast

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `node ui/check.mjs`

## PR Full And Nightly

Later increments add fixture golden tests, OPC loopback Docker services, fake Madmom, fake orientation, visualizer frame hashes, server e2e, UI e2e, and load tests.

## Manual Hardware Checklist

- Dome flash-by-strut, strut iteration, strand test, and full-color flash.
- Bar control box 5 and runner/corner diagnostics.
- Stage side/layer diagnostics.
- OPC reconnect after interruption.
- MIDI board, audio volume, Madmom beat tracking, tap tempo, and orientation paintbrush.
