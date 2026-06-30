# UI Expectations

This document records the browser UI states worth checking. Add screenshots when a state is stable enough for comparison.

## Operator Shell

The shell is rendered by React components in `ui/src/main.tsx`, styled by shared CSS in `ui/src/styles.css`, built into `ui/dist`, and checked by `node ui/check.mjs`.

Expected elements:

- `MindShark Dome Control Panel` heading
- `Start` and `Stop` engine buttons
- engine status text
- `Config Editor` drawer with full native JSON config reload/apply controls plus structured input, tempo, Madmom, output, and layout controls
- floating `OPC Targets` footer showing dome/bar and stage target addresses, enabled state, connection state, successful frame count, and last error
- WebSocket stream status text
- `domeActiveVis` selector with Volume, Radial, Race, Snakes, Quaternion Test, Quaternion Multi Test, Quaternion Paintbrush, Splat, and TV Static
- `flashSpeed` slider
- closed `Palettes` drawer with the active palette selector
- opening `Palettes` shows all eight palette slots at once, each with eight editable entries, `color1`, `color2`, and gradient enablement
- closed `Inputs` drawer with tap tempo, orientation calibration, audio/MIDI/orientation/Madmom adapter status, active orientation devices, and MIDI log
- closed `Debug Visuals` drawer with selectors for dome, bar, and stage test patterns
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
- enabled OPC targets show `connected` and an increasing frame count after successful TCP writes
- frame counters advance while the engine is running
- opening the `Preview` drawer connects the simulator stream and shows runtime frame data
- closing the drawer stops requesting browser preview frames

TODO: Add image of running state.

- Capture: after clicking `Start`.
- Expected: status reads `running`.
- Suggested file: `docs/images/ui-running-state.png`.

## Dome Visualizer Selection

Expected behavior when selecting each dome visualizer:

- selected value matches the server config field `dome.active_visualizer`
- flash speed and palette slot update server runtime config
- palette color and gradient edits patch all 64 `config.color_palette` entries through `/api/config/palette`
- Debug Visuals selectors patch test-pattern fields through `/api/config/diagnostics`
- dome debug visuals override the active dome visualizer until switched back to `Off`
- simulator frame stream updates after the selection is applied
- invalid values are currently clamped server-side; tighter validation belongs with the config editor work

TODO: Add image sequence of the visualizer selector.

- Capture: dropdown open and at least one selected non-default mode.
- Expected: labels match the Spectrum active visualizer map.
- Suggested file: `docs/images/ui-dome-visualizer-selector.png`.

## Simulator Frame View

Expected simulator behavior:

- `/simulator` is a dedicated simulator page
- `/simulator` has simulator-only controls for visualizer, fake audio/beat inputs, flash overlay, and preview colors
- changing `/simulator` controls does not patch live runtime config, shared simulator inputs, or hardware output
- TV Static is selectable in the dome visualizer controls
- Stage Depth is a stage-output visualizer and belongs with future stage simulator controls, not the dome canvas selector
- the live controls page starts simulator work only after opening the `Preview` drawer
- the live `Preview` drawer mirrors the runtime frame stream used for hardware output
- dome canvas uses runtime frame data from the server, not direct hardware sockets
- per-pixel visualizers render visible pixels
- buffer-based visualizers can render with OPC disabled
- display color compensation is applied only to the UI view, never to OPC bytes
- preview drawer shows frame metrics, stream status, and `dome-simulator` without simulator-only controls
- live preview and `/simulator` expose bar and stage command previews beside the dome canvas

TODO: Add image of a non-empty dome simulator frame.

- Capture: canvas after a deterministic simulator frame is rendered.
- Expected: visible colored points/struts, with the selected visualizer noted in the caption.
- Suggested file: `docs/images/ui-dome-simulator-frame.png`.

## Control Backlog

These controls are not cuts. They need API state, UI binding, simulator evidence, and tests.

- dome volume rotation speed
- dome gradient speed
- dome global fade speed
- dome global hue speed
- dome twinkle density
- dome ripple controls
- dome radial effect, size, frequency, center angle, and center distance
- diagnostics panel
