# Porting Inventory

This document tracks used Spectrum visualizer behavior for the Rust rewrite.
When `dome-rs` intentionally replaces Spectrum behavior rather than porting it,
record the decision in [`intentional-deviations.md`](intentional-deviations.md).

## Live Visualizers

Spectrum registers 19 visualizer classes in `Spectrum/Operator.cs`. `dome-rs` ports the 17 used entries: selectable dome modes, overlay/fallback modes, diagnostics, and stage/bar modes. It does not carry the dead MIDI test visualizer or standalone Stage Tracer visualizer.

Parity status is tracked in `fixtures/spectrum-csharp/visualizer_frame_cases.json`.
Each entry records the Spectrum source file and source hash for the deterministic
case that must eventually produce a C# frame hash. Entries marked
`pending_csharp_execution` are wired and source-traceable, but not yet proven as
exact frame ports.

Selectable dome modes:

- `LEDDomeVolumeVisualizer`: default dome mode, audio input, per-pixel output. Status: wired deterministic approximation, fixture case pending C# frame hash.
- `LEDDomeRadialVisualizer`: radial audio mode, buffer output. Status: wired deterministic approximation, fixture case pending C# frame hash.
- `LEDDomeRaceVisualizer`: audio race mode; constructor accepts MIDI but the implementation does not use it. Status: wired deterministic approximation, fixture case pending C# frame hash.
- `LEDDomeSnakesVisualizer`: audio snakes mode and triangle graph helpers. Status: wired deterministic approximation, fixture case pending C# frame hash.
- `LEDDomeQuaternionTestVisualizer`: selectable orientation test mode. Status: wired deterministic approximation, orientation payload parity pending.
- `LEDDomeQuaternionMultiTestVisualizer`: selectable orientation test mode. Status: wired deterministic approximation, orientation payload parity pending.
- `LEDDomeQuaternionPaintbrushVisualizer`: orientation paintbrush mode, buffer output. Status: wired deterministic approximation, orientation payload parity pending.
- `LEDDomeSplatVisualizer`: audio splat mode, buffer output. Status: wired deterministic approximation, fixture case pending C# frame hash.
- `LEDDomeTVStaticVisualizer`: deterministic static mode, selectable in `dome-rs` for simulator/operator visibility. Status: wired deterministic approximation, fixture case pending C# frame hash.

Other live modes:

- `LEDDomeFlashVisualizer`: MIDI flash overlay via priority-2 tie. Status: wired deterministic approximation, fixture case pending C# frame hash.
- `LEDStageDepthLevelVisualizer`: live stage mode, using `TracerLEDIndex` helper. Status: strongest partial port; helper progression is tested, full frame hash pending.

## Support

Support classification means the visualizer is used for diagnostics, fixtures, or helper behavior rather than the normal dome VJ selector. Support visualizers are not dead code. Operators access these modes from the **Debug Visuals** drawer on the main page, which patches the `test_pattern` config fields.

- `LEDDomeStrutIterationDiagnosticVisualizer`: dome diagnostic pattern. Status: wired, fixture case pending C# frame hash and hardware sign-off.
- `LEDDomeFlashColorsDiagnosticVisualizer`: dome diagnostic pattern. Status: wired, fixture case pending C# frame hash and hardware sign-off.
- `LEDDomeStrandTestDiagnosticVisualizer`: dome diagnostic pattern. Status: wired, fixture case pending C# frame hash and hardware sign-off.
- `LEDDomeFullColorFlashDiagnosticVisualizer`: dome diagnostic pattern. Status: wired, fixture case pending C# frame hash and hardware sign-off.
- `LEDBarFlashColorsDiagnosticVisualizer`: bar diagnostic pattern. Status: wired, fixture case pending C# frame hash and hardware sign-off.
- `LEDStageFlashColorsDiagnosticVisualizer`: stage diagnostic pattern. Status: wired, fixture case pending C# frame hash and hardware sign-off.
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
