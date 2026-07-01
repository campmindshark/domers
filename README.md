# dome-rs

`dome-rs` is a Rust lighting control server for the MindShark dome. It ports the active parts of [Spectrum](https://github.com/campmindshark/spectrum) into a headless runtime with a browser control surface, a dome simulator, Spectrum-compatible OPC encoding, TOML configuration, and input support for MIDI, audio, orientation, tap tempo, and Madmom beat protocol handling.

## Features

- Browser control page served by the Rust binary at `/`.
- Dedicated simulator page served at `/simulator`.
- Runtime controls for engine start/stop, active dome visualizer, flash speed, active palette slot, and full palette colors/gradients.
- Inputs drawer for tap tempo controls, MIDI log, and orientation calibration.
- Debug Visuals drawer for dome, bar, and stage hardware-check patterns.
- Live Preview drawer that mirrors the runtime frame stream used for hardware output.
- Independent `/simulator` sandbox with local controls that do not change runtime config or hardware output.
- Dome simulator canvas streamed over WebSocket from engine frame data.
- Spectrum-compatible dome topology and projection data for the full 7,580-LED dome layout.
- Spectrum-compatible OPC packet encoding.
- Native TOML configuration with a Spectrum XML import command and shared-entry palette format.
- Core input support for MIDI/audio UDP adapters, optional macOS/Linux native capture, orientation datagram ingestion, tap tempo, and Madmom `BEAT:{seconds}` protocol parsing.
- Managed Madmom sidecar lifecycle using `DBNBeatTracker --host_api --audio_input=<index> online`.
- Ableton Link / Carabiner-compatible sidecar tempo sync and Spectrum ADSR MIDI level drivers.

## Quick Start

Install/check local development dependencies:

```sh
tools/install_dev_deps.sh
```

Use `tools/install_dev_deps.sh --check` to verify Rust, Python, ALSA/PortAudio,
and the sibling Spectrum Madmom checkout without changing anything. On Linux the
installer uses `sudo apt-get` for system headers and `python3 -m pip --user` for
Madmom Python packages. Set `SPECTRUM_REPO=/path/to/spectrum` if the Spectrum
checkout is not next to `domers`.

Start the operator server with the checked example config:

```sh
cargo run --bin domers -- run --config examples/domers.toml --bind 127.0.0.1:3000
```

Open `http://127.0.0.1:3000` and use **MindShark Dome Control Panel**.

## Hardware Startup

`examples/domers.toml` is generated from the dome's Spectrum XML, but its OPC hosts are set to `127.0.0.1` so local loopback services can stand in for ledscape during development. To connect to physical controllers, copy the config and set the enabled hardware targets to the show network addresses:

```toml
[dome]
enabled = true
opc_address = "192.168.1.69:7890"

[stage]
enabled = true
opc_address = "192.168.1.70:7890"
```

Check config, bind address, OPC address syntax, and Madmom command availability before starting output:

```sh
cargo run --bin domers -- doctor --config domers.toml --bind 127.0.0.1:3000
```

Then start with the same config, click `Start`, and check the fixed **Runtime Status** footer on the controls page. It shows each configured target address, enabled state, TCP connection state, successful frame count, input adapter status, MIDI level-driver values, orientation devices, and the last connection/write error.

## Configuration

Copy the starter config:

```sh
cp examples/domers.toml domers.toml
```

Import an existing Spectrum XML config:

```sh
cargo run --bin domers -- import-spectrum-xml \
  fixtures/config/spectrum_default_config.xml \
  domers.toml
```

See [docs/configuration.md](docs/configuration.md) for the TOML schema, palette format, and import behavior.

## API Surface

- `GET /`: browser controls
- `GET /simulator`: full simulator view
- `GET /api/health`: health JSON
- `GET /api/state`: current runtime state, config, metrics, simulator inputs, hardware status, and input status
- `POST /api/start`: start the engine loop
- `POST /api/stop`: stop the engine loop
- `PATCH /api/config/dome`: update dome runtime controls
- `PATCH /api/config/diagnostics`: update dome, bar, and stage diagnostic/test-pattern controls
- `PATCH /api/config/palette`: update one runtime palette entry, including gradients
- `POST /api/input/tap`: record one tap-tempo input
- `GET /api/dome/geometry`: dome projection geometry
- `GET /api/dome/mapping`: dome strut and LED mapping
- `PATCH /api/simulator`: update shared simulator input state used by runtime preview rendering
- `GET /api/simulator/frame`: render one runtime preview frame
- `POST /api/simulator/sandbox-frame`: render a simulator page frame without changing runtime state
- `GET /ws/simulator`: stream simulator frames and metrics

## Development

```sh
tools/install_dev_deps.sh --check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd ui && bun install && bun run build && cd ..
node ui/check.mjs
```

## Crates

- [`domers-core`](crates/core/README.md): shared config, color, beat, and migration domain types.
- [`domers-engine`](crates/engine/README.md): deterministic operator scheduling rules.
- [`domers-inputs`](crates/inputs/README.md): fakeable audio, MIDI, Madmom, and orientation input helpers.
- [`domers-outputs`](crates/outputs/README.md): output commands, simulator sinks, topology helpers, and OPC transport.
- [`domers-server`](crates/server/README.md): runtime state, HTTP/WebSocket API, input tasks, simulator frames, and hardware output.
- [`domers-visualizers`](crates/visualizers/README.md): dome/bar/stage visualizer and diagnostic renderers.
- [`domers-test-support`](crates/test-support/README.md): shared test helpers and fixture smoke checks.

## Documentation

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
- [UI expectations](docs/ui-expectations.md)
- [Testing](docs/testing.md)
- [Hardware mapping](docs/hardware-mapping.md)
- [Hardware readiness](docs/hardware-readiness.md)
- [Porting inventory](docs/porting-inventory.md)
- [Intentional deviations from Spectrum](docs/intentional-deviations.md)
