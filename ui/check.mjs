import { readFile } from 'node:fs/promises';

const required = [
  '../README.md',
  '../docs/architecture.md',
  '../docs/configuration.md',
  '../docs/fixture-capture.md',
  '../docs/hardware-mapping.md',
  '../docs/hardware-readiness.md',
  '../docs/images/.gitkeep',
  '../docs/intentional-deviations.md',
  '../docs/parity-closure.md',
  '../docs/porting-inventory.md',
  '../docs/testing.md',
  '../docs/ui-expectations.md',
  '../examples/domers.toml',
  './index.html',
  './simulator.html',
  './main.mjs',
];

for (const path of required) {
  const full = new URL(path, import.meta.url);
  try {
    await import('node:fs/promises').then(fs => fs.access(full));
  } catch {
    console.error(`Missing required UI-adjacent document: ${path}`);
    process.exit(1);
  }
}

const html = await readFile(new URL('./index.html', import.meta.url), 'utf8');
for (const marker of [
  'data-domers-app',
  'MindShark Dome Control Panel',
  'aria-label="Runtime Controls"',
  'id="preview-drawer"',
  '<summary>Preview</summary>',
  'href="/simulator"',
  'id="palette-drawer"',
  '<summary>Palettes</summary>',
  'Palette colors',
  'id="palette-grid"',
  'data-palette-editor="64-entry-gradient"',
  'id="config-drawer"',
  'id="config-editor"',
  'id="reload-config"',
  'id="apply-config"',
  'id="config-status"',
  'id="inputs-drawer"',
  '<summary>Inputs</summary>',
  'id="tap-tempo"',
  'id="orientation-calibrate"',
  'id="input-audio"',
  'id="input-midi"',
  'id="input-orientation"',
  'id="input-madmom"',
  'id="orientation-devices"',
  'id="midi-log"',
  'id="start-engine"',
  'id="stop-engine"',
  'class="opc-targets-footer"',
  'position: fixed',
  'aria-label="OPC Targets"',
  'id="hardware-dome"',
  'id="hardware-stage"',
  'id="debug-visuals-drawer"',
  '<summary>Debug Visuals</summary>',
  'id="dome-test-pattern"',
  'id="bar-test-pattern"',
  'id="stage-test-pattern"',
  'name="domeActiveVis"',
  'TV Static',
  'id="flash-speed"',
  'mirrors the live runtime frame stream used for hardware output',
  'id="palette-index"',
  'id="stream-status"',
  'id="metric-frames"',
  'id="metric-simulator-frames"',
  'id="dome-simulator"',
  'id="bar-simulator"',
  'id="stage-simulator"',
  'data-frame-source="websocket"',
  'max-width: calc(100vw - 2rem)',
  'width: 100%',
]) {
  if (!html.includes(marker)) {
    console.error(`Missing required UI marker: ${marker}`);
    process.exit(1);
  }
}

const simulatorHtml = await readFile(new URL('./simulator.html', import.meta.url), 'utf8');
for (const marker of [
  'data-domers-simulator',
  'MindShark Dome Simulator',
  'href="/"',
  'id="stream-status"',
  'id="dome-simulator"',
  'id="bar-simulator"',
  'id="stage-simulator"',
  'data-page="simulator"',
  'aria-label="Simulator-Only Controls"',
  'id="sandbox-dome-active-vis"',
  'Stage Depth is a stage-output visualizer',
  'id="sandbox-volume"',
  'id="sandbox-beat-progress"',
  'id="sandbox-flash-active"',
  'id="sandbox-palette-primary"',
  'id="sandbox-palette-secondary"',
  'id="sandbox-palette-accent"',
  'do not patch live runtime config or hardware output',
]) {
  if (!simulatorHtml.includes(marker)) {
    console.error(`Missing required simulator page marker: ${marker}`);
    process.exit(1);
  }
}

const js = await readFile(new URL('./main.mjs', import.meta.url), 'utf8');
for (const marker of [
  '/api/state',
  'updateHardwareStatus',
  '/api/start',
  '/api/stop',
  '/api/config',
  'loadFullConfig',
  'applyFullConfig',
  '/api/config/dome',
  '/api/config/diagnostics',
  '/api/config/palette',
  '/api/input/tap',
  '/api/input/orientation/calibrate',
  'renderPaletteEditor',
  "paletteInput(paletteSlot, colorIndex, 'color2')",
  'color2_enabled',
  'updateInputStatus',
  'updateOrientationDevices',
  'updateMidiLog',
  '/api/dome/geometry',
  '/api/dome/mapping',
  '/api/simulator',
  '/api/simulator/frame',
  '/api/simulator/sandbox-frame',
  '/ws/simulator',
  'command.kind === \'pixel\'',
  'bar_commands',
  'stage_commands',
  'formatBarCommand',
  'formatStageCommand',
  'buildDomeLedPoints',
  'domeStrutLedCounts',
  'function drawLed',
  'SPECTRUM_CANVAS_SIZE',
  'resizeSimulatorCanvas',
  'ensureSimulatorStarted',
  'refreshSandboxFrame',
  'stopSimulatorPreview',
  'window.addEventListener(\'resize\'',
]) {
  if (!js.includes(marker)) {
    console.error(`Missing required API marker: ${marker}`);
    process.exit(1);
  }
}

console.log('ui smoke check ok');
