# Domers

Rust rewrite of Camp Mindshark Spectrum lighting control.

This repository is being built fixture-first: no hardware should be required to validate ordinary pull requests. Hardware checks are release gates only.

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

Local Docker/Rust may not be installed on every workstation; GitHub Actions is the merge-blocking source of truth.
