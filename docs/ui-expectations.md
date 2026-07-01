# UI Expectations

This document records the browser UI states covered by the built React app and
`node ui/check.mjs`.

## Operator Shell

The shell is rendered by React components in `ui/src/main.tsx`, styled by shared CSS in `ui/src/styles.css`, built into `ui/dist`, and checked by `node ui/check.mjs`.

Expected elements:

- `MindShark Dome Control Panel` heading
- `Start` and `Stop` engine buttons
- engine status text
- `Config Editor` drawer with full native JSON config reload/apply controls plus card-based structured input, tempo, Madmom, output, brightness, and stage-layout controls
- fixed runtime status footer showing dome/bar and stage OPC targets plus audio, MIDI, MIDI level-driver, orientation, Madmom, DJ Link, and orientation-device status
- WebSocket stream status text
- `domeActiveVis` selector with Volume, Radial, Race, Snakes, Quaternion Test, Quaternion Multi Test, Quaternion Paintbrush, Splat, and TV Static
- `flashSpeed` slider
- closed `Palettes` drawer with the active palette selector
- opening `Palettes` shows all eight palette slots at once, each with eight editable entries, `color1`, `color2`, and gradient enablement
- closed `Inputs` drawer split into BPM, Wands, and MIDI sections with tap/reset/manual BPM tempo, orientation calibration, and MIDI log
- closed `Debug Visuals` drawer with selectors for dome, bar, and stage test patterns
- closed `Preview` drawer
- link from the preview drawer to `/simulator`
- responsive mobile layout that stacks dense cards and keeps header/footer controls usable on narrow screens

## Running Engine State

Expected behavior after clicking `Start`:

- engine status changes to `running`
- controls remain interactive
- enabled OPC targets show `connected` and an increasing frame count after successful TCP writes
- frame counters advance while the engine is running
- opening the `Preview` drawer connects the simulator stream and shows runtime frame data
- closing the drawer stops requesting browser preview frames

## Dome Visualizer Selection

Expected behavior when selecting each dome visualizer:

- selected value matches the server config field `dome.active_visualizer`
- flash speed and palette slot update server runtime config
- palette color and gradient edits patch all 64 `config.color_palette` entries through `/api/config/palette`
- Debug Visuals selectors patch test-pattern fields through `/api/config/diagnostics`
- dome debug visuals override the active dome visualizer until switched back to `Off`
- simulator frame stream updates after the selection is applied
- invalid values are clamped server-side

## Simulator Frame View

Expected simulator behavior:

- `/simulator` is a dedicated simulator page
- `/simulator` has simulator controls for visualizer, fake audio/beat inputs, flash overlay, and preview colors
- changing `/simulator` controls does not patch live runtime config, shared simulator inputs, or hardware output
- TV Static is selectable in the dome visualizer controls
- the live controls page starts simulator work after opening the `Preview` drawer
- the live `Preview` drawer mirrors the runtime frame stream used for hardware output
- dome canvas uses runtime frame data from the server, not direct hardware sockets
- per-pixel visualizers render visible pixels
- buffer-based visualizers can render with OPC disabled
- display color compensation is applied to the UI view and never to OPC bytes
- preview drawer shows frame metrics, stream status, and `dome-simulator` without sandbox controls
- live preview and `/simulator` expose bar and stage command previews beside the dome canvas
