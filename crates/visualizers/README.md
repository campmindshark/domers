# domers-visualizers

Spectrum visualizer ports and golden-test harness.

## Module layout

```
src/
  lib.rs           — crate root; public re-exports
  inventory.rs     — `INVENTORY`, `Classification`
  input.rs         — `VisualizerInput`, diagnostic/live enums
  quaternion.rs    — public `Quaternion` type
  render.rs        — stateless `render_*` entry points
  runtime/         — persistent `VisualizerRuntime` + per-visualizer state machines
  dome/            — stateless frame builders (Volume, Flash, Race, …)
  diagnostics.rs   — dome/bar/stage diagnostic visualizers
  buffer.rs        — full-dome pixel buffer (Radial, Splat, Paintbrush)
  geometry.rs      — dome LED point fixtures + distance helpers
  math.rs          — map/wrap, radial effect, animation progress
  color_util.rs    — HSV, scale, diagnostic palette helpers
  rng.rs           — `DotNetRandom` (Spectrum-compatible)
  hash.rs          — golden-test frame hashing (test-only)
  tests/           — golden parity + unit tests
```

## Tests

Default `cargo test` runs unit tests and the **multi-frame sequence** golden suite.

First-frame goldens are `#[ignore]`; run via `make test-parity`.

```sh
make test-parity
cargo test -p domers-visualizers rust_visualizer_sequences_match_spectrum_csharp_goldens
```

See [docs/testing.md](../../docs/testing.md) and [docs/parity-closure.md](../../docs/parity-closure.md).
