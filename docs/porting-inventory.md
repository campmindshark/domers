# Porting Inventory

This document tracks used Spectrum visualizer behavior for the Rust rewrite.
When `dome-rs` intentionally replaces Spectrum behavior rather than porting it,
record the decision in [`intentional-deviations.md`](intentional-deviations.md).

## Live Visualizers

Spectrum registers 19 visualizer classes in `Spectrum/Operator.cs`. `dome-rs` ports the 17 used entries: selectable dome modes, overlay/fallback modes, diagnostics, and stage/bar modes. It does not carry the dead MIDI test visualizer or standalone Stage Tracer visualizer.

Parity status is tracked in two manifests:

- `fixtures/spectrum-csharp/visualizer_frame_cases.json` — first-frame FNV-1a hashes (17 cases).
- `fixtures/spectrum-csharp/visualizer_sequence_cases.json` — multi-frame sequences for stateful motion (11 cases).

Each entry records the Spectrum source file, source hash, deterministic input
(and `input_sequence` where applicable), and captured headless Spectrum frame
hashes. Exactness is closed when the Rust renderer produces the same hash on
every frame or the difference is documented in [`intentional-deviations.md`](intentional-deviations.md).

Stateful dome modes run through `VisualizerRuntime`, which preserves per-mode
state across operator frames, advances wall-clock throttled effects (Snakes,
diagnostics), and wipes the dome buffer on visualizer switch.

Selectable dome modes:

- `LEDDomeVolumeVisualizer`: default dome mode, audio input, per-pixel output. Status: first-frame and multi-frame sequence goldens match.
- `LEDDomeRadialVisualizer`: radial audio mode, buffer output. Status: first-frame and sequence goldens match.
- `LEDDomeRaceVisualizer`: audio race mode; constructor accepts MIDI but the implementation does not use it. Status: first-frame and sequence goldens match.
- `LEDDomeSnakesVisualizer`: audio snakes mode and triangle graph helpers. Status: first-frame and sequence goldens match.
- `LEDDomeQuaternionTestVisualizer`: selectable orientation test mode. Status: first-frame and sequence goldens match.
- `LEDDomeQuaternionMultiTestVisualizer`: selectable orientation test mode. Status: first-frame and sequence goldens match.
- `LEDDomeQuaternionPaintbrushVisualizer`: orientation paintbrush mode, buffer output. Status: first-frame and sequence goldens match.
- `LEDDomeSplatVisualizer`: audio splat mode, buffer output. Status: first-frame and sequence goldens match.
- `LEDDomeTVStaticVisualizer`: deterministic static mode, selectable in `dome-rs` for simulator/operator visibility. Status: first-frame and sequence goldens match.

Other live modes:

- `LEDDomeFlashVisualizer`: MIDI flash overlay via priority-2 tie. Status: first-frame and sequence goldens match.
- `LEDStageDepthLevelVisualizer`: live stage mode, using `TracerLEDIndex` helper. Status: first-frame and sequence goldens match.

## Support

Support classification means the visualizer is used for diagnostics, fixtures, or helper behavior rather than the normal dome VJ selector. Support visualizers are not dead code. Operators access these modes from the **Debug Visuals** drawer on the main page, which patches the `test_pattern` config fields.

- `LEDDomeStrutIterationDiagnosticVisualizer`: dome diagnostic pattern. Status: wired with captured Spectrum hash; physical sign-off remains hardware acceptance.
- `LEDDomeFlashColorsDiagnosticVisualizer`: dome diagnostic pattern. Status: wired with captured Spectrum hash; physical sign-off remains hardware acceptance.
- `LEDDomeStrandTestDiagnosticVisualizer`: dome diagnostic pattern. Status: wired with captured Spectrum hash; physical sign-off remains hardware acceptance.
- `LEDDomeFullColorFlashDiagnosticVisualizer`: dome diagnostic pattern. Status: wired with captured Spectrum hash; physical sign-off remains hardware acceptance.
- `LEDBarFlashColorsDiagnosticVisualizer`: bar diagnostic pattern. Status: wired with captured Spectrum hash; physical sign-off remains hardware acceptance.
- `LEDStageFlashColorsDiagnosticVisualizer`: stage diagnostic pattern. Status: wired with captured Spectrum hash; physical sign-off remains hardware acceptance.
- Dome/bar/stage command protocols from `LEDCommand.cs`.
- Dome physical mapping and projection data.
- `SimulatorUtils.GetComputerColor` display compensation.
- `LEDStageTracerVisualizer.TracerLEDIndex` helper only.

Diagnostic selector mapping:

- Dome `test_pattern = 1`: Flash Colors.
- Dome `test_pattern = 2`: Strut Iteration.
- Dome `test_pattern = 3`: Strand Test.
- Dome `test_pattern = 4`: Full Color Flash.
- Bar `test_pattern = 1`: Flash Colors.
- Stage `test_pattern = 1`: Flash Colors.

## Scheduler Rules

- Priority `0` is never selected.
- Highest priority `>= 1` wins per output.
- Ties run together, which is how Flash overlays the active dome mode.
- Priority `1000` diagnostics override normal modes.
- Priority `-1` is supported as always-run, although Spectrum visualizers in the inventory do not use it.

## Porting Entry Fields

```text
Name:
Source file:
Classification: live | support
Inputs:
Outputs:
Config fields:
Simulator proof:
Hardware proof:
Notes:
```
