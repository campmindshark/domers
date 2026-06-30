# CI Docker Harness

This directory documents the Docker loopback services used for CI expansion.
The Rust suite already covers the first loopback gates without Docker:

```sh
cargo test -p domers-server hardware_outputs_send_mapped_dome_frame_to_loopback_opc
cargo test -p domers-server hardware_outputs_reconnect_after_loopback_opc_returns
cargo test -p domers-server runtime_udp_input_adapters_feed_live_state
```

Those tests verify OPC write visibility, OPC reconnect after a missing listener
returns, and live UDP input ingestion. Docker services can replace the in-process
listeners when the suite needs multi-process failure injection.

Planned Docker services:

- OPC TCP listener
- fake Madmom sidecar
- fake orientation UDP emitter
- integration runner

CI validates the layout now. Docker Compose services should wrap the same gates
before physical hardware sign-off is attempted.

## Example

```sh
docker compose -f docker/ci/docker-compose.yml up --abort-on-container-exit
```

Target services:

```text
server-under-test -> opc-listener
server-under-test -> fake-madmom
fake-orientation  -> server-under-test:5005/udp
integration-runner -> server-under-test
```

## TODO Images

TODO: Add image of Docker e2e run.

- Capture: terminal or GitHub Actions log with all Docker CI services passing.
- Expected: OPC listener, fake sidecars, and integration runner exit successfully.
- Suggested file: `docs/images/docker-ci-e2e-success.png`.
