# domers-core

Shared domain types for the `dome-rs` workspace.

## Responsibilities

- Native `DomersConfig` and imported Spectrum XML configuration.
- Engine-facing `EngineConfig` projection.
- Spectrum-compatible color, palette, and gradient math.
- Beat timing primitives for wall-clock tap tempo, Madmom beats, DJ Link tempo, BPM display, flash gates, and reset behavior.
- Spectrum-derived Carabiner, native input, and level-driver configuration.
- Migration warnings for stale, inert, or invalid Spectrum config fields.

## Key Files

- `src/config.rs`: native TOML schema, Spectrum XML import, and config serialization.
- `src/color.rs`: RGB, palette entries, 64-slot palette model, and Spectrum gradient behavior.
- `src/beat.rs`: deterministic beat broadcaster and clock semantics.
- `src/migration.rs`: migration analyzer and warning categories.

## Tests

Run this crate with:

```sh
cargo test -p domers-core
```
