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
  'aria-label="Runtime Controls"',
  'id="preview-drawer"',
  '<summary>Preview</summary>',
  'href="/simulator"',
  'Runtime palette colors',
  'id="start-engine"',
  'id="stop-engine"',
  'aria-label="OPC Targets"',
  'id="hardware-dome"',
  'id="hardware-stage"',
  'name="domeActiveVis"',
  'TV Static',
  'id="flash-speed"',
  'mirrors the live runtime frame stream used for hardware output',
  'id="palette-index"',
  'id="palette-primary"',
  'id="palette-secondary"',
  'id="palette-accent"',
  'id="stream-status"',
  'id="metric-frames"',
  'id="metric-simulator-frames"',
  'id="dome-simulator"',
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
  '/api/config/dome',
  '/api/config/palette',
  '/api/dome/geometry',
  '/api/dome/mapping',
  '/api/simulator',
  '/api/simulator/frame',
  '/api/simulator/sandbox-frame',
  '/ws/simulator',
  'command.kind === \'pixel\'',
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
