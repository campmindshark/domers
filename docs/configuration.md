# Configuration

`dome-rs` uses TOML as its native runtime configuration format. XML is not a runtime configuration format for the Rust app. Spectrum XML is accepted by the import command so existing show/laptop configs can be migrated intentionally.

Intentional differences from Spectrum runtime config are tracked in
[`intentional-deviations.md`](intentional-deviations.md).

## Native TOML

A `dome-rs` config is organized into fixture and subsystem sections:

Use `examples/domers.toml` as the checked starter config:

```sh
cp examples/domers.toml domers.toml
```

```toml
color_palette_index = 7

[dome]
enabled = true
simulation_enabled = true
opc_address = "127.0.0.1:7890"
active_visualizer = 0
test_pattern = 0
brightness = 0.356915762888129

[bar]
enabled = true
simulation_enabled = true
infinity_width = 50
infinity_length = 50
runner_length = 50
brightness = 0.814093959731544
test_pattern = 1

[stage]
enabled = false
simulation_enabled = false
opc_address = "127.0.0.1:7890"
side_lengths = [18, 19, 19]
brightness = 0.834591194968554
test_pattern = 0

[tempo]
source = "human"
flash_speed = 0.0

[madmom]
command = "DBNBeatTracker"
audio_input_index = 0

[color_palette]
banks = [
  ["entry_01", "entry_02", "entry_03", "entry_04", "entry_05", "entry_06", "entry_07", "entry_08"],
]

[color_palette.entries.entry_01]
color1 = 65280
color2 = 0
color2_enabled = false
```

TOML fits the project well: it is common in Rust, easy to diff, and strict enough for operator-edited config.

## Color Palette

The runtime palette follows Spectrum's layout: eight palette banks with eight slots each. The TOML keeps that slot layout while defining duplicate color entries only once.

```toml
[color_palette]
banks = [
  ["entry_01", "entry_02", "entry_03", "entry_04", "entry_05", "entry_06", "entry_07", "entry_08"],
]

[color_palette.entries.entry_01]
color1 = 16711680
color2 = 15792383
color2_enabled = true
```

- `color1` and `color2` use Spectrum's `0xRRGGBB` integer convention.
- `color2_enabled = false` makes the entry a solid `color1`.
- `color2_enabled = true` enables Spectrum-compatible gradient blending.
- Runtime palette slot `N` still uses bank `N` with eight entries; repeated bank references intentionally share one entry definition.
- The old verbose `[[color_palette.colors]]` 64-entry absolute form still parses for compatibility.

If `color_palette` is omitted, `dome-rs` creates a default 64-entry palette with visible starter colors in entries 0-2.

## Import Existing Spectrum XML

Use `domers-config` to convert a Spectrum XML file into a `dome-rs` TOML file:

```sh
cargo run --bin domers-config -- import-spectrum-xml /path/to/spectrum_config.xml domers.toml
```

The command:

- reads the legacy Spectrum XML
- maps live fields into the native TOML schema
- writes a new `dome-rs` TOML file
- rewrites Spectrum OPC hosts to localhost in the checked example config so local loopback services can stand in for ledscape during development
- writes palette banks with shared entry definitions instead of repeating duplicate XML colors
- prints warnings for stale Spectrum fields, inert v1 cuts, and invalid MIDI binding targets

Example warnings:

```text
warning: StaleField: huesEnabled
warning: StaleField: kickT
warning: InvalidMidiBindingTarget: snareT
warning: InertField: domeAutoFlashDelay
```

## Madmom Config

Madmom remains a managed beat sidecar. The current input crate parses beat lines and includes a sidecar wrapper for Spectrum's launch contract.

Old Spectrum behavior:

```text
Madmom/env/Scripts/python.exe DBNBeatTracker --host_api --audio_input=<index> online
```

`dome-rs` config:

```toml
[madmom]
command = "DBNBeatTracker"
audio_input_index = 0
```

The command can point at a bundled Python environment, a wrapper script, a system install, a Docker sidecar launcher, or a native replacement. The repo does not require a Madmom git submodule. The sidecar wrapper launches:

```text
<command> --host_api --audio_input=<index> online
```

The stable runtime contract is stdout lines shaped like:

```text
BEAT:12.345
```

Bundling Madmom is a release packaging choice, not a runtime path assumption. A packaged `dome-rs` build needs either a working Madmom distribution or a documented one-command installer so operators do not have to assemble the beat tracker by hand.

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
