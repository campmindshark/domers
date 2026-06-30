# Fixture Capture

M0 captures C# references before Rust behavior is trusted.

## Capture Command

```sh
python3 tools/extract_spectrum_fixtures.py
```

Build the legacy C# project before any executable Spectrum fixture capture:

```sh
python3 tools/build_spectrum_csharp.py
```

The helper defaults to `../spectrum/Spectrum/Spectrum.csproj` and initializes
the Spectrum `Madmom` submodule if needed. The full `Spectrum.sln` includes
`Madmom/Madmom.pyproj`, which requires Visual Studio Python Tools and is not
buildable with plain `dotnet`, so the C# app project is the repeatable fixture
gate.

Check whether visualizer frame goldens are complete:

```sh
python3 tools/check_visualizer_goldens.py
```

Capture visualizer frame goldens by running the old Spectrum code headlessly:

```sh
python3 tools/capture_spectrum_visualizer_frames.py
```

The visualizer capture runner references `Spectrum.csproj`, loads Spectrum's
default XML config with the same serializer family as the WPF app, forces
simulation-only/no-hardware output, invokes each visualizer directly, and writes
captured hashes into `visualizer_frame_cases.json`. `check_visualizer_goldens.py`
fails if any visualizer case has `expected.status != "captured"` or a null
`expected.value`.

Generated fixture groups:

- `fixtures/spectrum-csharp/dome_mapping.json`
- `fixtures/spectrum-csharp/dome_geometry.json`
- `fixtures/spectrum-csharp/bar_stage_topology.json`
- `fixtures/spectrum-csharp/executable_capture.json`
- `fixtures/spectrum-csharp/opc_packets/`
- `fixtures/spectrum-csharp/visualizer_frame_cases.json`
- `fixtures/config/spectrum_default_config.xml`
- `fixtures/orientation/datagram_lengths.json`
- `fixtures/madmom/valid-and-invalid.txt`

## Required Captures

- Dome strut table and control-box mapping.
- Dome projection points and simulator coordinates.
- Bar and stage topology.
- Headless C# bar/stage simulator command queue semantics.
- OPC packet bytes for single-pixel, sparse, and full-frame writes.
- Source-traceable visualizer frame cases for every used Spectrum visualizer.
- Captured Spectrum frame hashes for every visualizer case, produced by
  `tools/capture_spectrum_visualizer_frames.py`.
- Default and edge XML configs.
- Orientation datagram examples.
- Madmom stdout examples.

Each fixture file must record source commit, command used to capture it, and whether hardware sign-off is still required.

## Example Fixture Use

```rust
let expected = include_bytes!("../../../fixtures/spectrum-csharp/opc_packets/two_pixels_channel_2.bin");
let encoded = encode_frame(2, &[Rgb::from_u24(0x123456), Rgb::from_u24(0xaabbcc)]);
assert_eq!(encoded.as_slice(), expected);
```

## TODO Images

TODO: Add image of the fixture directory after extraction.

- Capture: editor file tree or terminal `ls -R fixtures` output.
- Expected: config, madmom, orientation, and spectrum-csharp fixture groups are visible.
- Suggested file: `docs/images/fixtures-directory-tree.png`.

TODO: Add image of source-to-fixture mapping.

- Capture: side-by-side view of C# source constant and generated JSON fixture.
- Expected: shows why the fixture is traceable back to Spectrum.
- Suggested file: `docs/images/fixtures-source-to-json.png`.
