# Fixtures

Reference fixtures are captured before Rust behavior is trusted.

Expected fixture groups:

- `spectrum-csharp/`: exported C# topology, OPC packet references, and headless executable captures
- `spectrum-csharp/executable_capture.json`: C#-executed bar/stage simulator command queue semantics
- `spectrum-csharp/visualizer_frame_cases.json`: first-frame visualizer parity (17 captured Spectrum C# hashes)
- `spectrum-csharp/visualizer_sequence_cases.json`: multi-frame stateful visualizer sequences (11 captured)
- `config/`: default and edge XML configs used only for migration tests
- `orientation/`: UDP datagram samples
- `madmom/`: sidecar stdout samples

All fixture captures must document source commit, command, and known unverified hardware behavior.
Visualizer cases include source hashes, deterministic inputs, and headless
Spectrum C# frame hashes.

## Regenerate Fixtures

```sh
python3 tools/extract_spectrum_fixtures.py
python3 tools/capture_spectrum_executable_fixtures.py
python3 tools/capture_spectrum_visualizer_frames.py
DOMERS_VISUALIZER_CASES=fixtures/spectrum-csharp/visualizer_sequence_cases.json python3 tools/capture_spectrum_visualizer_frames.py
```

## Examples

Read an OPC packet fixture from a Rust test:

```rust
let expected = include_bytes!("../../../fixtures/spectrum-csharp/opc_packets/two_pixels_channel_2.bin");
```

Read the legacy Spectrum config fixture for TOML import tests:

```rust
let xml = include_str!("../../../fixtures/config/spectrum_default_config.xml");
```
