# Domers

Rust rewrite of Camp Mindshark Spectrum lighting control.

## Current Status

Initial scaffold only. The first implementation increments establish:

- documented porting inventory
- C# fixture capture layout
- Rust workspace and CI gates
- scheduler, OPC, input, simulator, and migration test seams

## Development

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
node ui/check.mjs
```

## Configuration

Domers uses TOML for native configuration. Spectrum XML is supported only as a legacy import source:

```sh
cargo run --bin domers-config -- import-spectrum-xml \
  fixtures/config/spectrum_default_config.xml \
  domers.toml
```

The importer writes a new Domers TOML file and prints warnings for stale Spectrum fields, inert v1 cuts, and invalid MIDI binding targets.

Example TOML output shape:

```toml
[dome]
enabled = true
simulation_enabled = false
opc_address = "192.168.1.69:7890"
active_visualizer = 0
test_pattern = 0
brightness = 0.356915762888129

[tempo]
source = "human"
flash_speed = 0.0

[madmom]
command = "DBNBeatTracker"
audio_input_index = 0
```

See [`docs/configuration.md`](docs/configuration.md) for the full config/import contract.
Intentional differences from Spectrum are tracked in
[`docs/intentional-deviations.md`](docs/intentional-deviations.md).

## Madmom

The old Windows app searched for a bundled Python 3.7 virtualenv at `Madmom/env/Scripts/python.exe` and spawned `DBNBeatTracker`, then parsed `BEAT:{seconds}` lines from stdout. Domers preserves that sidecar protocol for compatibility, but the command/path is config-driven in TOML instead of being hard-coded to the Windows bundle. A future native Rust beat tracker can replace the sidecar behind the same beat input contract.

## UI Reference

The first browser shell exposes engine start/stop, dome visualizer selection, flash speed, and a dome simulator canvas. See [`docs/ui-expectations.md`](docs/ui-expectations.md) for expected UI states and image placeholders.

TODO: Add image of the initial Domers operator page here.

- Capture: full browser window at desktop size.
- Expected: title, start/stop buttons, dome visualizer selector, flash speed slider, and black simulator canvas are visible.
- Suggested file: `docs/images/readme-operator-shell.png`.

Local Docker/Rust may not be installed on every workstation; GitHub Actions is the merge-blocking source of truth.
