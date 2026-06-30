# Porting Inventory

This document tracks live, support, and dead Spectrum behavior for the Rust rewrite.

## Live Visualizers

- `LEDDomeVolumeVisualizer`: default dome mode, audio input, per-pixel output.
- `LEDDomeQuaternionPaintbrushVisualizer`: orientation paintbrush mode, buffer output.
- `LEDDomeFlashVisualizer`: MIDI flash overlay via priority-2 tie.
- `LEDDomeRadialVisualizer`: radial audio mode, buffer output.
- `LEDDomeRaceVisualizer`: audio race mode; constructor accepts MIDI but current logic does not use it.
- `LEDDomeSnakesVisualizer`: audio snakes mode and triangle graph helpers.
- `LEDDomeSplatVisualizer`: audio splat mode, buffer output.
- `LEDDomeQuaternionTestVisualizer`: selectable orientation test mode.
- `LEDDomeQuaternionMultiTestVisualizer`: selectable orientation test mode.
- `LEDDomeTVStaticVisualizer`: priority-1 dome fallback.
- `LEDStageDepthLevelVisualizer`: live stage mode, using `TracerLEDIndex` helper.

## Support

- Dome diagnostic patterns: flash colors, strut iteration, strand test, full-color flash.
- Bar and stage diagnostic flash patterns.
- Dome/bar/stage command protocols from `LEDCommand.cs`.
- Dome physical mapping and projection data.
- `SimulatorUtils.GetComputerColor` display compensation.
- `LEDStageTracerVisualizer.TracerLEDIndex` helper only.

## Dead Or V1 Cut

- `LEDDomeMidiTestVisualizer`: priority `0`, never selected.
- Standalone `LEDStageTracerVisualizer`: superseded by Stage Depth priority.
- Hue and LED-board XML remnants.
- `domeAutoFlashDelay` unless explicitly redesigned.
- Level-driver runtime behavior until a visualizer consumes it.
- Ableton Link/Carabiner runtime sync until a real beat-sync design exists.
- Standalone bar OPC path; current production routes bar through dome control box 5.

## Scheduler Rules

- Priority `0` is never selected.
- Highest priority `>= 1` wins per output.
- Ties run together, which is how Flash overlays the active dome mode.
- Priority `1000` diagnostics override normal modes.
- Priority `-1` is supported as always-run, although current Spectrum visualizers do not use it.
