const status = document.querySelector('#engine-status');
const streamStatus = document.querySelector('#stream-status');
const hardwareDome = document.querySelector('#hardware-dome');
const hardwareStage = document.querySelector('#hardware-stage');
const activeVisualizer = document.querySelector('#dome-active-vis');
const flashSpeed = document.querySelector('#flash-speed');
const flashSpeedValue = document.querySelector('#flash-speed-value');
const domeTestPattern = document.querySelector('#dome-test-pattern');
const barTestPattern = document.querySelector('#bar-test-pattern');
const stageTestPattern = document.querySelector('#stage-test-pattern');
const simVolume = document.querySelector('#sim-volume');
const simVolumeValue = document.querySelector('#sim-volume-value');
const simBeatProgress = document.querySelector('#sim-beat-progress');
const simBeatProgressValue = document.querySelector('#sim-beat-progress-value');
const simFlashActive = document.querySelector('#sim-flash-active');
const paletteIndex = document.querySelector('#palette-index');
const palettePrimary = document.querySelector('#palette-primary');
const paletteSecondary = document.querySelector('#palette-secondary');
const paletteAccent = document.querySelector('#palette-accent');
const sandboxActiveVisualizer = document.querySelector('#sandbox-dome-active-vis');
const sandboxVolume = document.querySelector('#sandbox-volume');
const sandboxVolumeValue = document.querySelector('#sandbox-volume-value');
const sandboxBeatProgress = document.querySelector('#sandbox-beat-progress');
const sandboxBeatProgressValue = document.querySelector('#sandbox-beat-progress-value');
const sandboxFlashActive = document.querySelector('#sandbox-flash-active');
const sandboxPalettePrimary = document.querySelector('#sandbox-palette-primary');
const sandboxPaletteSecondary = document.querySelector('#sandbox-palette-secondary');
const sandboxPaletteAccent = document.querySelector('#sandbox-palette-accent');
const metricFrames = document.querySelector('#metric-frames');
const metricSimulatorFrames = document.querySelector('#metric-simulator-frames');
const previewDrawer = document.querySelector('#preview-drawer');
const canvas = document.querySelector('#dome-simulator');
const context = canvas?.getContext('2d');
const isDedicatedSimulatorPage = document.body?.dataset.page === 'simulator';
const SPECTRUM_CANVAS_SIZE = 750;
const SPECTRUM_PROJECTION_OFFSET = 10;
const SPECTRUM_PROJECTION_SPAN = 690;
let domeLayout;
let domeLedPoints = [];
let latestSimulatorFrame;
let simulatorStarted = false;
let simulatorSocket;

async function request(path, options = {}) {
  const response = await fetch(path, {
    headers: { 'content-type': 'application/json' },
    ...options,
  });
  if (!response.ok) {
    throw new Error(`${path} failed with ${response.status}`);
  }
  return response.json();
}

async function loadDomeLayout() {
  const [geometry, mapping] = await Promise.all([
    request('/api/dome/geometry'),
    request('/api/dome/mapping'),
  ]);
  domeLayout = { geometry, mapping };
  rebuildDomeLedPoints();
}

function updateSnapshot(snapshot) {
  if (status) {
    status.textContent = snapshot.running ? 'running' : 'stopped';
  }
  if (metricFrames) {
    metricFrames.textContent = String(snapshot.metrics.frames);
  }
  if (metricSimulatorFrames) {
    metricSimulatorFrames.textContent = String(snapshot.metrics.simulator_frames);
  }
  updateHardwareStatus(hardwareDome, snapshot.hardware?.dome);
  updateHardwareStatus(hardwareStage, snapshot.hardware?.stage);
  if (activeVisualizer) {
    activeVisualizer.value = String(snapshot.config.dome_active_vis);
  }
  if (flashSpeed) {
    flashSpeed.value = String(snapshot.config.flash_speed);
  }
  if (flashSpeedValue) {
    flashSpeedValue.textContent = String(snapshot.config.flash_speed);
  }
  if (paletteIndex) {
    paletteIndex.value = String(snapshot.config.color_palette_index);
  }
  if (domeTestPattern) {
    domeTestPattern.value = String(snapshot.diagnostics?.dome_test_pattern ?? 0);
  }
  if (barTestPattern) {
    barTestPattern.value = String(snapshot.diagnostics?.bar_test_pattern ?? 0);
  }
  if (stageTestPattern) {
    stageTestPattern.value = String(snapshot.diagnostics?.stage_test_pattern ?? 0);
  }
  if (simVolume) {
    simVolume.value = String(snapshot.simulator.volume);
  }
  if (simVolumeValue) {
    simVolumeValue.textContent = String(snapshot.simulator.volume);
  }
  if (simBeatProgress) {
    simBeatProgress.value = String(snapshot.simulator.beat_progress);
  }
  if (simBeatProgressValue) {
    simBeatProgressValue.textContent = String(snapshot.simulator.beat_progress);
  }
  if (simFlashActive) {
    simFlashActive.checked = snapshot.simulator.flash_active;
  }
  if (palettePrimary) {
    palettePrimary.value = toColorInput(paletteColor(snapshot, 0));
  }
  if (paletteSecondary) {
    paletteSecondary.value = toColorInput(paletteColor(snapshot, 1));
  }
  if (paletteAccent) {
    paletteAccent.value = toColorInput(paletteColor(snapshot, 2));
  }
}

function updateHardwareStatus(element, target) {
  if (!element || !target) {
    return;
  }

  const address = target.address ?? 'no address configured';
  if (!target.enabled) {
    element.textContent = `${address} - disabled`;
    return;
  }

  const state = target.connected ? 'connected' : 'not connected';
  const frames = `${target.frames_sent ?? 0} frames sent`;
  const error = target.last_error ? ` - last error: ${target.last_error}` : '';
  element.textContent = `${address} - ${state}, ${frames}${error}`;
}

function toColorInput(color) {
  return `#${color.toString(16).padStart(6, '0')}`;
}

function fromColorInput(color) {
  return Number.parseInt(color.replace('#', ''), 16);
}

function paletteColor(snapshot, relativeIndex) {
  const absoluteIndex = snapshot.config.color_palette_index * 8 + relativeIndex;
  return snapshot.config.color_palette.colors[absoluteIndex]?.color1 ?? 0;
}

function clearCanvas() {
  if (!context) {
    return;
  }
  context.fillStyle = '#000000';
  context.fillRect(0, 0, canvas.width, canvas.height);
}

function drawFrame(colors) {
  if (!context || !colors.length) {
    return;
  }

  resizeSimulatorCanvas();
  clearCanvas();
  context.lineWidth = 1;

  colors.forEach((color, index) => {
    const point = domeLedPoints[index] ?? fallbackDomePoint(index, colors.length);
    drawLed(point.x, point.y, color, point.size);
  });
}

function drawPixel(command) {
  if (!context) {
    return;
  }

  resizeSimulatorCanvas();
  const index = command.strut_index * 3 + command.led_index;
  const point = domeLedPoints[index] ?? fallbackDomePoint(index, 190);
  drawLed(point.x, point.y, command.color, point.size * 2);
}

function drawLed(x, y, color, radius) {
  context.fillStyle = toColorInput(color);
  const size = Math.max(1, radius);
  context.fillRect(Math.round(x), Math.round(y), size, size);
}

function fallbackDomePoint(index, total) {
  const center = canvas.width / 2;
  const maxRadius = canvas.width * 0.46;
  const normalized = total <= 1 ? 0 : index / (total - 1);
  const ring = Math.floor(normalized * 7);
  const ringStart = ring / 7;
  const ringEnd = (ring + 1) / 7;
  const inRing = (normalized - ringStart) / Math.max(0.001, ringEnd - ringStart);
  const angle = inRing * Math.PI * 2 + ring * 0.43;
  const radius = maxRadius * Math.sqrt((ring + 0.45) / 7);

  return {
    x: center + Math.cos(angle) * radius,
    y: center + Math.sin(angle) * radius,
    size: Math.max(4, canvas.width / 95),
  };
}

function buildDomeLedPoints(geometry, mapping) {
  const ledCounts = domeStrutLedCounts(mapping);
  const points = [];
  const scale = canvas.width / SPECTRUM_CANVAS_SIZE;
  const offset = SPECTRUM_PROJECTION_OFFSET * scale;
  const span = SPECTRUM_PROJECTION_SPAN * scale;
  const ledSize = Math.max(1, Math.round(scale));

  for (let strut = 0; strut < geometry.lines.length; strut += 1) {
    const line = geometry.lines[strut];
    const start = geometry.hand_drawn_points[line.start];
    const end = geometry.hand_drawn_points[line.end];
    const leds = ledCounts[strut] ?? 0;
    for (let led = 0; led < leds; led += 1) {
      const d = (led + 1) / (leds + 2);
      points.push({
        x: offset + ((end.normalized_x - start.normalized_x) * d + start.normalized_x) * span,
        y: offset + ((end.normalized_y - start.normalized_y) * d + start.normalized_y) * span,
        size: ledSize,
      });
    }
  }

  return points;
}

function domeStrutLedCounts(mapping) {
  return mapping.strut_positions.map(position => {
    let strutsLeft = position.control_box_strut_index;
    let strand = 0;
    while (mapping.control_box_strut_order[strand].length <= strutsLeft) {
      strutsLeft -= mapping.control_box_strut_order[strand].length;
      strand += 1;
    }
    const strutType = mapping.control_box_strut_order[strand][strutsLeft];
    return mapping.strut_lengths[strutType];
  });
}

function rebuildDomeLedPoints() {
  if (!domeLayout) {
    return;
  }
  domeLedPoints = buildDomeLedPoints(domeLayout.geometry, domeLayout.mapping);
}

function resizeSimulatorCanvas() {
  if (!canvas || !context) {
    return false;
  }

  const rect = canvas.getBoundingClientRect();
  const size = Math.max(320, Math.round(rect.width || SPECTRUM_CANVAS_SIZE));
  if (canvas.width === size && canvas.height === size) {
    return false;
  }

  canvas.width = size;
  canvas.height = size;
  rebuildDomeLedPoints();
  return true;
}

function redrawLatestSimulatorFrame() {
  if (latestSimulatorFrame) {
    handleSimulatorFrame(latestSimulatorFrame);
  } else {
    resizeSimulatorCanvas();
    clearCanvas();
  }
}

function handleSimulatorFrame(frame) {
  latestSimulatorFrame = frame;
  if (metricFrames) {
    metricFrames.textContent = String(frame.metrics.frames);
  }
  if (metricSimulatorFrames) {
    metricSimulatorFrames.textContent = String(frame.metrics.simulator_frames);
  }
  resizeSimulatorCanvas();
  clearCanvas();

  for (const command of frame.commands) {
    if (command.kind === 'frame') {
      drawFrame(command.colors);
    } else if (command.kind === 'pixel') {
      drawPixel(command);
    }
  }
}

async function refreshState() {
  const snapshot = await request('/api/state');
  updateSnapshot(snapshot);
  if (simulatorStarted) {
    handleSimulatorFrame(await request('/api/simulator/frame'));
  }
}

async function patchSimulatorControls() {
  const snapshot = await request('/api/simulator', {
    method: 'PATCH',
    body: JSON.stringify({
      volume: Number(simVolume.value),
      beat_progress: Number(simBeatProgress.value),
      flash_active: simFlashActive.checked,
    }),
  });
  updateSnapshot(snapshot);
  await refreshPreviewFrame();
}

async function patchRuntimeControls() {
  const snapshot = await request('/api/config/dome', {
    method: 'PATCH',
    body: JSON.stringify({
      active_visualizer: Number(activeVisualizer.value),
      flash_speed: Number(flashSpeed.value),
      color_palette_index: Number(paletteIndex.value),
    }),
  });
  updateSnapshot(snapshot);
  await refreshPreviewFrame();
}

async function patchDiagnosticControls() {
  const snapshot = await request('/api/config/diagnostics', {
    method: 'PATCH',
    body: JSON.stringify({
      dome_test_pattern: Number(domeTestPattern?.value ?? 0),
      bar_test_pattern: Number(barTestPattern?.value ?? 0),
      stage_test_pattern: Number(stageTestPattern?.value ?? 0),
    }),
  });
  updateSnapshot(snapshot);
  await refreshPreviewFrame();
}

async function patchPaletteColor(relativeIndex, colorInput) {
  const snapshot = await request('/api/config/palette', {
    method: 'PATCH',
    body: JSON.stringify({
      relative_index: relativeIndex,
      color1: fromColorInput(colorInput.value),
      color2_enabled: false,
    }),
  });
  updateSnapshot(snapshot);
  await refreshPreviewFrame();
}

async function refreshPreviewFrame() {
  if (simulatorStarted) {
    if (isDedicatedSimulatorPage) {
      await refreshSandboxFrame();
    } else {
      handleSimulatorFrame(await request('/api/simulator/frame'));
    }
  }
}

async function refreshSandboxFrame() {
  if (!canvas) {
    return;
  }
  updateSandboxControlLabels();
  handleSimulatorFrame(await request('/api/simulator/sandbox-frame', {
    method: 'POST',
    body: JSON.stringify({
      active_visualizer: Number(sandboxActiveVisualizer?.value ?? 0),
      volume: Number(sandboxVolume?.value ?? 0.7),
      beat_progress: Number(sandboxBeatProgress?.value ?? 0.25),
      flash_active: sandboxFlashActive?.checked ?? true,
      primary: fromColorInput(sandboxPalettePrimary?.value ?? '#00ff00'),
      secondary: fromColorInput(sandboxPaletteSecondary?.value ?? '#0080ff'),
      accent: fromColorInput(sandboxPaletteAccent?.value ?? '#ff4080'),
    }),
  }));
}

function updateSandboxControlLabels() {
  if (sandboxVolumeValue && sandboxVolume) {
    sandboxVolumeValue.textContent = sandboxVolume.value;
  }
  if (sandboxBeatProgressValue && sandboxBeatProgress) {
    sandboxBeatProgressValue.textContent = sandboxBeatProgress.value;
  }
}

async function ensureSimulatorStarted() {
  if (simulatorStarted || !canvas) {
    return;
  }
  simulatorStarted = true;
  if (!domeLayout) {
    await loadDomeLayout();
  } else {
    rebuildDomeLedPoints();
  }
  if (isDedicatedSimulatorPage) {
    if (streamStatus) {
      streamStatus.textContent = 'simulator sandbox';
    }
    await refreshSandboxFrame();
  } else {
    handleSimulatorFrame(await request('/api/simulator/frame'));
    connectSimulatorStream();
  }
}

function stopSimulatorPreview() {
  simulatorStarted = false;
  if (simulatorSocket) {
    simulatorSocket.close();
    simulatorSocket = undefined;
  }
  if (streamStatus) {
    streamStatus.textContent = 'stream disconnected';
  }
}

function connectSimulatorStream() {
  if (simulatorSocket) {
    return;
  }
  const scheme = window.location.protocol === 'https:' ? 'wss' : 'ws';
  const socket = new WebSocket(`${scheme}://${window.location.host}/ws/simulator`);
  simulatorSocket = socket;
  if (streamStatus) {
    streamStatus.textContent = 'stream connecting';
  }

  socket.addEventListener('open', () => {
    if (streamStatus) {
      streamStatus.textContent = 'stream connected';
    }
  });

  socket.addEventListener('message', event => {
    handleSimulatorFrame(JSON.parse(event.data));
  });

  socket.addEventListener('close', () => {
    simulatorSocket = undefined;
    if (streamStatus) {
      streamStatus.textContent = 'stream disconnected';
    }
    if (simulatorStarted) {
      window.setTimeout(connectSimulatorStream, 1000);
    }
  });
}

document.querySelector('#start-engine')?.addEventListener('click', async () => {
  updateSnapshot(await request('/api/start', { method: 'POST' }));
});

document.querySelector('#stop-engine')?.addEventListener('click', async () => {
  updateSnapshot(await request('/api/stop', { method: 'POST' }));
});

for (const input of [activeVisualizer, flashSpeed, paletteIndex]) {
  input?.addEventListener('input', async () => {
    if (flashSpeedValue) {
      flashSpeedValue.textContent = flashSpeed.value;
    }
    await patchRuntimeControls();
  });
}

for (const input of [domeTestPattern, barTestPattern, stageTestPattern]) {
  input?.addEventListener('input', patchDiagnosticControls);
}

for (const [relativeIndex, input] of [
  [0, palettePrimary],
  [1, paletteSecondary],
  [2, paletteAccent],
]) {
  input?.addEventListener('input', async () => {
    await patchPaletteColor(relativeIndex, input);
  });
}

for (const input of [
  simVolume,
  simBeatProgress,
  simFlashActive,
]) {
  input?.addEventListener('input', async () => {
    if (simVolumeValue) {
      simVolumeValue.textContent = simVolume.value;
    }
    if (simBeatProgressValue) {
      simBeatProgressValue.textContent = simBeatProgress.value;
    }
    await patchSimulatorControls();
  });
}

for (const input of [
  sandboxActiveVisualizer,
  sandboxVolume,
  sandboxBeatProgress,
  sandboxFlashActive,
  sandboxPalettePrimary,
  sandboxPaletteSecondary,
  sandboxPaletteAccent,
]) {
  input?.addEventListener('input', refreshSandboxFrame);
}

previewDrawer?.addEventListener('toggle', async () => {
  if (previewDrawer.open) {
    await ensureSimulatorStarted();
  } else {
    stopSimulatorPreview();
  }
});

window.addEventListener('resize', redrawLatestSimulatorFrame);

await refreshState();
if (isDedicatedSimulatorPage) {
  await ensureSimulatorStarted();
}
