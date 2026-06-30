# dome-rs

`dome-rs` is a Rust lighting control server for the MindShark dome. It ports the active parts of [Spectrum](https://github.com/campmindshark/spectrum) into a headless runtime with a browser control surface, a dome simulator, Spectrum-compatible OPC encoding, TOML configuration, and input support for MIDI, audio, orientation, tap tempo, and Madmom beat protocol handling.

## Features

- Browser control page served by the Rust binary at `/`.
- Dedicated simulator page served at `/simulator`.
- Runtime controls for engine start/stop, active dome visualizer, flash speed, active palette slot, and runtime palette colors.
- Dome simulator canvas streamed over WebSocket from engine frame data.
- Spectrum-compatible dome topology and projection data for the full 7,580-LED dome layout.
- Spectrum-compatible OPC packet encoding.
- Native TOML configuration with a Spectrum XML import command.
- Core input support for MIDI replay, audio volume replay, orientation datagram classification, tap tempo, and Madmom `BEAT:{seconds}` protocol parsing.
- Managed Madmom sidecar launch wrapper using `DBNBeatTracker --host_api --audio_input=<index> online`.

## Quick Start

```sh
cargo run --bin domers -- --config examples/domers.toml --bind 127.0.0.1:3000
```

Open `http://127.0.0.1:3000` and use **MindShark Dome Controls**.

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
- `GET /api/state`: current runtime state, config, metrics, and simulator inputs
- `POST /api/start`: start the engine loop
- `POST /api/stop`: stop the engine loop
- `PATCH /api/config/dome`: update dome runtime controls
- `PATCH /api/config/palette`: update one runtime palette color
- `GET /api/dome/geometry`: dome projection geometry
- `GET /api/dome/mapping`: dome strut and LED mapping
- `PATCH /api/simulator`: update simulator-only preview inputs
- `GET /api/simulator/frame`: render one simulator frame
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
- Expected: title, start/stop buttons, runtime controls, simulator preview inputs, metrics, stream status, and simulator canvas are visible.
- Suggested file: `docs/images/readme-operator-shell.png`.