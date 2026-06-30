# UI Expectations

This document records the browser UI states worth checking. Add screenshots when a state is stable enough for comparison.

## Operator Shell

The shell lives in `ui/index.html` and is checked by `node ui/check.mjs`.

Expected elements:

- `MindShark Dome Controls` heading
- `Start` and `Stop` engine buttons
- engine status text
- WebSocket stream status text
- `domeActiveVis` selector with Volume, Radial, Race, Snakes, Quaternion Test, Quaternion Multi Test, Quaternion Paintbrush, and Splat
- `flashSpeed` slider
- eight palette slots matching Spectrum's VJ HUD selection
- runtime palette color controls for entries 1-3 in the selected palette slot
- closed `Preview` drawer
- link from the preview drawer to `/simulator`

TODO: Add image of the operator shell.

- Capture: browser window at desktop size.
- Expected: runtime controls are visible and the `Preview` drawer is closed.
- Suggested file: `docs/images/ui-operator-shell.png`.

## Running Engine State

Expected behavior after clicking `Start`:

- engine status changes to `running`
- controls remain interactive
- stream status reads `stream connected` once the WebSocket connects
- frame counters advance while the engine is running
- simulator canvas remains visible and receives frame data

TODO: Add image of running state.

- Capture: after clicking `Start`.
- Expected: status reads `running`.
- Suggested file: `docs/images/ui-running-state.png`.

## Dome Visualizer Selection

Expected behavior when selecting each dome visualizer:

- selected value matches the server config field `dome.active_visualizer`
- flash speed and palette slot update server runtime config
- palette color edits patch `config.color_palette` through `/api/config/palette`
- simulator frame stream updates after the selection is applied
- invalid values are rejected after API config validation is tightened

TODO: Add image sequence of the visualizer selector.

- Capture: dropdown open and at least one selected non-default mode.
- Expected: labels match the Spectrum active visualizer map.
- Suggested file: `docs/images/ui-dome-visualizer-selector.png`.

## Simulator Frame View

Expected simulator behavior:

- `/simulator` is a dedicated simulator page
- `/simulator` has simulator-only controls for visualizer, fake audio/beat inputs, flash overlay, and preview colors
- changing `/simulator` controls does not patch live runtime config, shared simulator inputs, or hardware output
- the live controls page starts simulator work only after opening the `Preview` drawer
- dome canvas uses frame data from the server, not direct hardware state
- per-pixel visualizers render visible pixels
- buffer-based visualizers can render with OPC disabled
- display color compensation is applied only to the UI view, never to OPC bytes
- preview drawer shows runtime-backed simulator volume, beat phase, flash-active controls, frame metrics, stream status, and `dome-simulator`

TODO: Add image of a non-empty dome simulator frame.

- Capture: canvas after a deterministic simulator frame is rendered.
- Expected: visible colored points/struts, with the selected visualizer noted in the caption.
- Suggested file: `docs/images/ui-dome-simulator-frame.png`.

## Control Backlog

These controls are not cuts. They need API state, UI binding, simulator evidence, and tests.

- config editor
- MIDI log
- full 64-entry Spectrum color palette editor, including gradient color2 controls
- dome volume rotation speed
- dome gradient speed
- dome global fade speed
- dome global hue speed
- dome twinkle density
- dome ripple controls
- dome radial effect, size, frequency, center angle, and center distance
- tempo/Madmom controls
- orientation calibration panel
- bar simulator
- stage simulator
- diagnostics panel
