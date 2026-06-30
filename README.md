# dome-rs

`dome-rs` is a Rust lighting control server for the MindShark dome. It ports the active parts of [Spectrum](https://github.com/campmindshark/spectrum) into a headless runtime with a browser control surface, a dome simulator, Spectrum-compatible OPC encoding, TOML configuration, and input support for MIDI, audio, orientation, tap tempo, and Madmom beat protocol handling.

## Features

- Browser control page served by the Rust binary at `/`.
- Dedicated simulator page served at `/simulator`.
- Runtime controls for engine start/stop, active dome visualizer, flash speed, active palette slot, and runtime palette colors.
- Live Preview drawer that mirrors the runtime frame stream used for hardware output.
- Independent `/simulator` sandbox with local controls that do not change runtime config or hardware output.
- Dome simulator canvas streamed over WebSocket from engine frame data.
- Spectrum-compatible dome topology and projection data for the full 7,580-LED dome layout.
- Spectrum-compatible OPC packet encoding.
- Native TOML configuration with a Spectrum XML import command and shared-entry palette format.
- Core input support for MIDI replay, audio volume replay, orientation datagram classification, tap tempo, and Madmom `BEAT:{seconds}` protocol parsing.
- Managed Madmom sidecar launch wrapper using `DBNBeatTracker --host_api --audio_input=<index> online`.

## Quick Start

Start the operator server with the checked example config:

```sh
cargo run --bin domers -- run --config examples/domers.toml --bind 127.0.0.1:3000
```

Open `http://127.0.0.1:3000` and use **MindShark Dome Controls**.

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

Then start with the same config, click `Start`, and check the **OPC Targets** panel on the controls page. It shows each configured target address, enabled state, TCP connection state, successful frame count, and the last connection/write error.

## Configuration

Copy the starter config:

```sh
cp examples/domers.toml domers.toml
```

Import an existing Spectrum XML config:

```sh
cargo run --bin domers-config -- import-spectrum-xml \
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
- `PATCH /api/config/palette`: update one runtime palette color
- `POST /api/input/tap`: record one tap-tempo input
- `GET /api/dome/geometry`: dome projection geometry
- `GET /api/dome/mapping`: dome strut and LED mapping
- `PATCH /api/simulator`: update shared simulator input state used by runtime preview rendering
- `GET /api/simulator/frame`: render one runtime preview frame
- `POST /api/simulator/sandbox-frame`: render a simulator page frame without changing runtime state
- `GET /ws/simulator`: stream simulator frames and metrics

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
node ui/check.mjs
```

## Documentation

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
- [UI expectations](docs/ui-expectations.md)
- [Testing](docs/testing.md)
- [Hardware mapping](docs/hardware-mapping.md)
- [Hardware readiness](docs/hardware-readiness.md)
- [Porting inventory](docs/porting-inventory.md)
- [Intentional deviations from Spectrum](docs/intentional-deviations.md)

## TODO Images

TODO: Add image of the MindShark Dome Controls page here.

- Capture: full browser window at desktop size.
- Expected: title, start/stop buttons, OPC Targets panel, runtime controls, metrics, stream status, and a closed Preview drawer are visible.
- Suggested file: `docs/images/readme-operator-shell.png`.
