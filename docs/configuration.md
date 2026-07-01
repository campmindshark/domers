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

[inputs.audio]

[inputs.midi]

[inputs.orientation]

[madmom]
command = "DBNBeatTracker"
audio_input_index = 0

[carabiner]
command = "carabiner"
args = []
human_link_output = false
madmom_link_output = false

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
- Palette slot `N` uses bank `N` with eight entries; repeated bank references intentionally share one entry definition.
- The old verbose `[[color_palette.colors]]` 64-entry absolute form parses for compatibility.

If `color_palette` is omitted, `dome-rs` creates a default 64-entry palette with visible starter colors in entries 0-2.

## Live Inputs

Live input adapters are optional and disabled by default. Set a UDP bind address
to enable a bridge input when `domers run` starts:

```toml
[inputs.audio]
bind = "127.0.0.1:5001" # text float payload, for example 0.42
native_enabled = false # set true in a native-capture build to use CPAL
device_id = "{0.0.1...}" # optional stable Spectrum endpoint id

[[inputs.audio.devices]]
id = "speaker"
name = "Speaker"
flow = "render"

[[inputs.audio.devices]]
id = "{0.0.1...}"
name = "Show Capture"
flow = "capture"

[inputs.midi]
bind = "127.0.0.1:5002" # note,64,1.0 or cc,1,0.5
native_enabled = false # set true in a native-capture build to use midir
device_id = "Controller Port Name"

[[inputs.midi.bindings]]
command_kind = "note"
index = 64
action = "flash"

[[inputs.midi.bindings]]
command_kind = "control_change"
index = 1
action = "volume"

[[inputs.midi.bindings]]
command_kind = "note"
index = 48 # first note in an 8-note ADSR channel range
action = "adsr_level_driver"

[inputs.orientation]
bind = "127.0.0.1:5005" # raw Spectrum orientation datagrams
```

The fixed **Runtime Status** footer shows each adapter target, accepted event
count, latest value, MIDI level-driver values, active orientation devices, and
last error. The **Inputs** drawer contains tap/reset tempo controls, recent MIDI
commands/actions, and the **Calibrate Orientation** control.

Supported MIDI binding actions are:

- `flash`: set flash overlay from command value.
- `volume`: set normalized volume from command value.
- `tap_tempo`: trigger tap tempo when command value is positive.
- `palette`: set the active palette. Use `target_index` for a fixed palette or omit it to map command value across palettes 0-7.
- `visualizer`: set the active dome visualizer. Use `target_index` for a fixed visualizer or omit it to map command value across visualizers 0-8.
- `adsr_level_driver`: treat `index` as the first note in an 8-note Spectrum ADSR range. Positive note velocity presses the channel; zero velocity releases it.

UDP audio and MIDI remain useful bridge transports. macOS/Linux native audio and
MIDI capture are available when the server is built with the `native-capture`
feature; device identity/index semantics are represented in config and tests.

Native capture is available in builds compiled with:

```sh
cargo run --features native-capture --bin domers -- run --config domers.toml
```

The native path uses `cpal` for audio and `midir` for MIDI. On Linux, install
ALSA development headers (for example `libasound2-dev`) before building with
`native-capture`; macOS uses CoreAudio/CoreMIDI.

`inputs.audio.devices` is optional. When present, it models Spectrum's audio
enumeration rule: all active endpoints receive an index, and capture endpoints
populate the config editor's audio device dropdown. If
`madmom.audio_input_index` is unset, `domers run` derives the Madmom/PortAudio
index from `inputs.audio.device_id`.

## Level Drivers

Spectrum level-driver presets import into `[level_drivers]`. MIDI ADSR presets
drive eight channel envelopes and can temporarily override visualizer volume.
After the release envelope and Spectrum's 5-second idle window, control returns
to audio volume.

```toml
[level_drivers.presets."midi test"]
source = "midi"
attack_time = 10
peak_level = 1.0
decay_time = 20
sustain_level = 0.8
release_time = 10

[level_drivers.presets."full spectrum"]
source = "audio"
filter_range_start = 0.0
filter_range_end = 1.0

[level_drivers.midi_channels]
0 = "midi test"

[level_drivers.audio_channels]
0 = "full spectrum"
```

Audio presets and channel assignments are preserved in config. MIDI ADSR presets
drive runtime level-driver channels; UDP audio samples and native CPAL capture
provide whole-signal volume input.

## Import Existing Spectrum XML

Use `domers import-spectrum-xml` to convert a Spectrum XML file into a `dome-rs` TOML file:

```sh
cargo run --bin domers -- import-spectrum-xml /path/to/spectrum_config.xml domers.toml
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

## Hardware Targets

The checked `examples/domers.toml` uses localhost OPC targets. For physical output, copy the file and replace enabled target addresses with the ledscape hosts on the show network:

```toml
[dome]
enabled = true
opc_address = "192.168.1.69:7890"

[stage]
enabled = true
opc_address = "192.168.1.70:7890"
```

Run `domers doctor` before starting output:

```sh
cargo run --bin domers -- doctor --config domers.toml --bind 127.0.0.1:3000
```

After clicking `Start`, the fixed **Runtime Status** footer shows the configured
addresses, whether each target is enabled, current TCP connection state,
successful frame count, and the most recent connection/write error. A connected
target with an increasing frame count means `dome-rs` is successfully writing
OPC frames to that TCP endpoint; physical LED confirmation is part of hardware
sign-off.

## Madmom Config

Madmom is a managed beat sidecar when `tempo.source = "madmom"`. `domers run`
starts the configured command, reads `BEAT:{seconds}` stdout lines, and feeds the
beat runtime. `domers doctor` validates that the command is runnable before
hardware output starts.

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

The command can point at a bundled tracker executable, a wrapper script, a system
install, a Docker sidecar launcher, or a native replacement. To mirror
Spectrum's Python invocation, set `command` to Python and `tracker` to
`DBNBeatTracker`. If `command` is a path to `python.exe`, `dome-rs` runs the
sidecar from that executable's directory, matching Spectrum's
`Madmom/env/Scripts` working-directory behavior:

```toml
[madmom]
command = "Madmom/env/Scripts/python.exe"
tracker = "DBNBeatTracker"
audio_input_index = 0
```

The repo does not require a Madmom git submodule. With no `tracker`, the sidecar
wrapper launches:

```text
<command> --host_api --audio_input=<index> online
```

With `tracker`, it launches:

```text
<command> <tracker> --host_api --audio_input=<index> online
```

The stable runtime contract is stdout lines shaped like:

```text
BEAT:12.345
```

Bundling Madmom is a release packaging choice, not a runtime path assumption. A packaged `dome-rs` build needs either a working Madmom distribution or a documented one-command installer so operators do not have to assemble the beat tracker by hand.

## Ableton Link / Carabiner Config

Ableton Link sync is enabled with `tempo.source = "link"`. `domers run` starts
the configured macOS/Linux sidecar command and reads tempo lines from stdout.
The sidecar can be a Carabiner-compatible bridge or a native Link bridge.

```toml
[tempo]
source = "link"
flash_speed = 0.0

[carabiner]
command = "carabiner"
args = ["--stdout-tempo"]
human_link_output = false
madmom_link_output = false
```

Accepted stdout examples:

```text
LINK 120 0.25
BPM: 120
tempo=120 phase=0.25
```

`phase` is optional and normalized into `[0.0, 1.0)`.

See [`intentional-deviations.md`](intentional-deviations.md) for the rationale.
