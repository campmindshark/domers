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
  './src/main.tsx',
  './src/styles.css',
  './main.mjs',
  './dist/index.html',
  './dist/simulator.html',
  './dist/assets/main.js',
  './dist/assets/styles.css',
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
const app = await readFile(new URL('./src/main.tsx', import.meta.url), 'utf8');
const css = await readFile(new URL('./src/styles.css', import.meta.url), 'utf8');
const builtHtml = await readFile(new URL('./dist/index.html', import.meta.url), 'utf8');
for (const marker of [
  'MindShark Dome Control Panel',
  'id="root"',
  '/src/main.tsx',
]) {
  if (!html.includes(marker)) {
    console.error(`Missing required control page marker: ${marker}`);
    process.exit(1);
  }
}

for (const marker of [
  'data-domers-app',
  'className="app-shell-header"',
  'className="app-shell-content"',
  'className="operator-tabs"',
  'data-tab-target="runtime-panel"',
  'data-tab-target="simulator-panel"',
  'data-tab-target="palettes-panel"',
  'data-tab-target="config-panel"',
  'className="operator-tab-panel is-active runtime-panel-grid"',
  'aria-label="Runtime Controls"',
  'id="preview-drawer"',
  'Simulator Preview',
  'href="/simulator"',
  'id="palette-drawer"',
  'Palette colors',
  'id="palette-grid"',
  'data-palette-editor="64-entry-gradient"',
  'id="config-drawer"',
  'id="config-editor"',
  'id="reload-config"',
  'id="apply-config"',
  'id="apply-structured-config"',
  'id="config-status"',
  'id="config-audio-bind"',
  'id="config-audio-native-enabled"',
  '<select id="config-audio-device-id"',
  'id="config-midi-bind"',
  'id="config-midi-native-enabled"',
  'id="config-midi-device-id"',
  'id="config-orientation-bind"',
  'id="config-tempo-source"',
  'id="config-madmom-command"',
  'id="config-madmom-tracker"',
  'id="config-madmom-audio-index"',
  'id="config-carabiner-command"',
  'id="config-carabiner-args"',
  'id="config-midi-bindings"',
  'id="config-dome-enabled"',
  'id="config-dome-simulation-enabled"',
  'id="config-dome-opc-address"',
  'id="config-dome-brightness-slider"',
  'id="config-dome-brightness"',
  'id="config-bar-enabled"',
  'id="config-bar-simulation-enabled"',
  'id="config-bar-infinity-length"',
  'id="config-bar-infinity-width"',
  'id="config-bar-runner-length"',
  'id="config-bar-brightness-slider"',
  'id="config-bar-brightness"',
  'id="config-stage-enabled"',
  'id="config-stage-simulation-enabled"',
  'id="config-stage-opc-address"',
  'id="config-stage-brightness-slider"',
  'id="config-stage-brightness"',
  'aria-label="Stage layout config"',
  'id="config-stage-side-lengths"',
  'id="config-stage-side-lengths-summary"',
  'id="config-stage-side-lengths-grid"',
  'id="inputs-drawer"',
  'id="tap-tempo"',
  'id="reset-tempo"',
  'id="tempo-bpm"',
  'id="tap-counter"',
  'id="orientation-calibrate"',
  'id="input-audio"',
  'id="input-midi"',
  'id="input-midi-levels"',
  'id="input-orientation"',
  'id="input-madmom"',
  'id="input-link"',
  'id="orientation-devices"',
  'id="midi-log"',
  'id="start-engine"',
  'id="stop-engine"',
  'aria-label="OPC Targets and Device Status"',
  'aria-label="OPC Targets"',
  'aria-label="Device Status"',
  'id="hardware-dome"',
  'id="hardware-stage"',
  'id="debug-visuals-drawer"',
  'id="dome-test-pattern"',
  'id="bar-test-pattern"',
  'id="stage-test-pattern"',
  'name="domeActiveVis"',
  'TV Static',
  'id="flash-speed"',
  'preview WebSocket disconnected',
  'Open simulator sandbox',
  'id="palette-index"',
  'id="stream-status"',
  'id="metric-frames"',
  'id="metric-simulator-frames"',
  'id="dome-simulator"',
  'id="bar-simulator"',
  'id="stage-simulator"',
  'data-frame-source="websocket"',
]) {
  if (!app.includes(marker)) {
    console.error(`Missing required React app marker: ${marker}`);
    process.exit(1);
  }
}

for (const marker of [
  'className="opc-targets-footer"',
  '.operator-tabs',
  '.runtime-panel-grid',
  'max-width: calc(100vw - 2rem)',
  'width: 100%',
  'grid-template-rows: auto minmax(0, 1fr) auto',
  'overflow: auto',
  '.slider-number-field',
  '.side-lengths-grid',
  '.palette-color-input',
  'height: 3rem',
  '@media (max-width: 720px)',
  '@media (max-width: 420px)',
]) {
  if (!css.includes(marker) && !app.includes(marker)) {
    console.error(`Missing required shared CSS marker: ${marker}`);
    process.exit(1);
  }
}

for (const marker of [
  '/assets/main.js',
  '/assets/styles.css',
]) {
  if (!builtHtml.includes(marker)) {
    console.error(`Missing required built HTML marker: ${marker}`);
    process.exit(1);
  }
}

const simulatorHtml = await readFile(new URL('./simulator.html', import.meta.url), 'utf8');
const builtSimulatorHtml = await readFile(new URL('./dist/simulator.html', import.meta.url), 'utf8');
for (const marker of [
  'MindShark Dome Simulator',
  'data-page="simulator"',
  'id="root"',
]) {
  if (!simulatorHtml.includes(marker)) {
    console.error(`Missing required simulator HTML marker: ${marker}`);
    process.exit(1);
  }
}

for (const marker of [
  'data-domers-simulator',
  'href="/"',
  'id="stream-status"',
  'id="dome-simulator"',
  'id="bar-simulator"',
  'id="stage-simulator"',
  'aria-label="Simulator Controls"',
  'id="sandbox-dome-active-vis"',
  'id="sandbox-volume"',
  'id="sandbox-beat-progress"',
  'id="sandbox-flash-active"',
  'id="sandbox-palette-primary"',
  'id="sandbox-palette-secondary"',
  'id="sandbox-palette-accent"',
]) {
  if (!app.includes(marker)) {
    console.error(`Missing required simulator page marker: ${marker}`);
    process.exit(1);
  }
}

for (const marker of [
  '/assets/main.js',
  '/assets/styles.css',
]) {
  if (!builtSimulatorHtml.includes(marker)) {
    console.error(`Missing required built simulator marker: ${marker}`);
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
  'updateConfigFromStructuredFields',
  '/api/config/dome',
  '/api/config/diagnostics',
  '/api/config/palette',
  '/api/input/tap',
  '/api/input/tempo/reset',
  '/api/input/orientation/calibrate',
  'renderPaletteEditor',
  "className = 'palette-color-input'",
  "labelWithText('Color 1', color1, 'palette-color-field')",
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
