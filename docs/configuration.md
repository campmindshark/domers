# Configuration

Domers uses TOML as its native runtime configuration format. XML is not a runtime configuration format for the Rust app. Spectrum XML is only accepted by the import command so existing show/laptop configs can be migrated intentionally.

Intentional differences from Spectrum runtime config are tracked in
[`intentional-deviations.md`](intentional-deviations.md).

## Native TOML

A Domers config is organized into fixture and subsystem sections:

```toml
[dome]
enabled = true
simulation_enabled = false
opc_address = "192.168.1.69:7890"
active_visualizer = 0
test_pattern = 0
brightness = 0.356915762888129

[bar]
enabled = true
simulation_enabled = false
infinity_width = 50
infinity_length = 50
runner_length = 50
brightness = 0.25
test_pattern = 0

[stage]
enabled = true
simulation_enabled = false
opc_address = "192.168.1.70:7890"
side_lengths = [18, 19, 19]
brightness = 0.8
test_pattern = 0

[tempo]
source = "human"
flash_speed = 0.0

[madmom]
command = "DBNBeatTracker"
audio_input_index = 0
```

Use TOML because it is common in Rust projects, easy to diff, and strict enough for operator-facing config. Avoid YAML for runtime config unless a future feature needs comments with richer nested operator-authored structures that TOML cannot express cleanly.

## Import Existing Spectrum XML

Use `domers-config` to convert a Spectrum XML file into a Domers TOML file:

```sh
cargo run --bin domers-config -- import-spectrum-xml   /path/to/spectrum_config.xml   domers.toml
```

The command:

- reads the legacy Spectrum XML
- maps live fields into the native TOML schema
- writes a new Domers TOML file
- prints warnings for stale Spectrum fields, inert v1 cuts, and invalid MIDI binding targets

Example warnings:

```text
warning: StaleField: huesEnabled
warning: StaleField: kickT
warning: InvalidMidiBindingTarget: snareT
warning: InertField: domeAutoFlashDelay
```

## Madmom Config

Domers keeps the beat sidecar protocol from Spectrum, but not the hard-coded Windows path.

Old Spectrum behavior:

```text
Madmom/env/Scripts/python.exe DBNBeatTracker --host_api --audio_input=<index> online
```

Domers behavior:

```toml
[madmom]
command = "DBNBeatTracker"
audio_input_index = 0
```

The command can point at a Python wrapper, a virtualenv executable, a script, or a future native replacement. The stable contract is stdout lines shaped like:

```text
BEAT:12.345
```

See [`intentional-deviations.md`](intentional-deviations.md) for the rationale.

## TODO Images

TODO: Add image of the config import command running successfully.

- Capture: terminal after running `domers-config import-spectrum-xml`.
- Expected: command exits successfully and prints migration warnings.
- Suggested file: `docs/images/config-import-success.png`.

TODO: Add image of an imported TOML config in the editor.

- Capture: editor with `[dome]`, `[tempo]`, and `[madmom]` sections visible.
- Expected: values are readable and no XML remains in the runtime config.
- Suggested file: `docs/images/imported-domers-toml.png`.
