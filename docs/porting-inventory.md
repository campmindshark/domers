# Porting Inventory

This document tracks used Spectrum visualizer behavior for the Rust rewrite.
When `dome-rs` intentionally replaces Spectrum behavior rather than porting it,
record the decision in [`intentional-deviations.md`](intentional-deviations.md).

## Live Visualizers

Spectrum registers 19 visualizer classes in `Spectrum/Operator.cs`. `dome-rs` ports the 17 used entries: selectable dome modes, overlay/fallback modes, diagnostics, and stage/bar modes. It does not carry the dead MIDI test visualizer or standalone Stage Tracer visualizer.

Selectable dome modes:

- `LEDDomeVolumeVisualizer`: default dome mode, audio input, per-pixel output.
- `LEDDomeRadialVisualizer`: radial audio mode, buffer output.
- `LEDDomeRaceVisualizer`: audio race mode; constructor accepts MIDI but the implementation does not use it.
- `LEDDomeSnakesVisualizer`: audio snakes mode and triangle graph helpers.
- `LEDDomeQuaternionTestVisualizer`: selectable orientation test mode.
- `LEDDomeQuaternionMultiTestVisualizer`: selectable orientation test mode.
- `LEDDomeQuaternionPaintbrushVisualizer`: orientation paintbrush mode, buffer output.
- `LEDDomeSplatVisualizer`: audio splat mode, buffer output.
- `LEDDomeTVStaticVisualizer`: deterministic static mode, selectable in `dome-rs` for simulator/operator visibility.

Other live modes:

- `LEDDomeFlashVisualizer`: MIDI flash overlay via priority-2 tie.
- `LEDStageDepthLevelVisualizer`: live stage mode, using `TracerLEDIndex` helper.

## Support

Support classification means the visualizer is used for diagnostics, fixtures, or helper behavior rather than the normal dome VJ selector. Support visualizers are not dead code. Operators access the diagnostic support modes from the **Support Diagnostics** controls on the main page, which patch the `test_pattern` config fields.

- `LEDDomeStrutIterationDiagnosticVisualizer`: dome diagnostic pattern.
- `LEDDomeFlashColorsDiagnosticVisualizer`: dome diagnostic pattern.
- `LEDDomeStrandTestDiagnosticVisualizer`: dome diagnostic pattern.
- `LEDDomeFullColorFlashDiagnosticVisualizer`: dome diagnostic pattern.
- `LEDBarFlashColorsDiagnosticVisualizer`: bar diagnostic pattern.
- `LEDStageFlashColorsDiagnosticVisualizer`: stage diagnostic pattern.
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

## Example Porting Entry Template

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

## TODO Images

TODO: Add image of Spectrum UI visualizer selector.

- Capture: old Spectrum UI/VJ HUD showing dome active visualizer choices.
- Expected: labels align with the current `domeActiveVis` selector, including TV Static.
- Suggested file: `docs/images/inventory-spectrum-visualizer-selector.png`.

TODO: Add image of diagnostic pattern selector.

- Capture: old Spectrum diagnostic/test-pattern controls.
- Expected: dome/bar/stage diagnostic patterns are visible.
- Suggested file: `docs/images/inventory-spectrum-diagnostics.png`.
