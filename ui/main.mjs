const status = document.querySelector('#engine-status');
const streamStatus = document.querySelector('#stream-status');
const activeVisualizer = document.querySelector('#dome-active-vis');
const flashSpeed = document.querySelector('#flash-speed');
const flashSpeedValue = document.querySelector('#flash-speed-value');
const simVolume = document.querySelector('#sim-volume');
const simVolumeValue = document.querySelector('#sim-volume-value');
const simBeatProgress = document.querySelector('#sim-beat-progress');
const simBeatProgressValue = document.querySelector('#sim-beat-progress-value');
const simFlashActive = document.querySelector('#sim-flash-active');
const paletteIndex = document.querySelector('#palette-index');
const palettePrimary = document.querySelector('#palette-primary');
const paletteSecondary = document.querySelector('#palette-secondary');
const paletteAccent = document.querySelector('#palette-accent');
const metricFrames = document.querySelector('#metric-frames');
const metricSimulatorFrames = document.querySelector('#metric-simulator-frames');
const canvas = document.querySelector('#dome-simulator');
const context = canvas?.getContext('2d');
const SPECTRUM_CANVAS_SIZE = 750;
const SPECTRUM_PROJECTION_OFFSET = 10;
const SPECTRUM_PROJECTION_SPAN = 690;
let domeLayout;
let domeLedPoints = [];
let latestSimulatorFrame;

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
  status.textContent = snapshot.running ? 'running' : 'stopped';
  metricFrames.textContent = String(snapshot.metrics.frames);
  metricSimulatorFrames.textContent = String(snapshot.metrics.simulator_frames);
  activeVisualizer.value = String(snapshot.config.dome_active_vis);
  flashSpeed.value = String(snapshot.config.flash_speed);
  flashSpeedValue.textContent = String(snapshot.config.flash_speed);
  paletteIndex.value = String(snapshot.config.color_palette_index);
  simVolume.value = String(snapshot.simulator.volume);
  simVolumeValue.textContent = String(snapshot.simulator.volume);
  simBeatProgress.value = String(snapshot.simulator.beat_progress);
  simBeatProgressValue.textContent = String(snapshot.simulator.beat_progress);
  simFlashActive.checked = snapshot.simulator.flash_active;
  palettePrimary.value = toColorInput(snapshot.simulator.primary);
  paletteSecondary.value = toColorInput(snapshot.simulator.secondary);
  paletteAccent.value = toColorInput(snapshot.simulator.accent);
}

function toColorInput(color) {
  return `#${color.toString(16).padStart(6, '0')}`;
}

function fromColorInput(color) {
  return Number.parseInt(color.replace('#', ''), 16);
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
  metricFrames.textContent = String(frame.metrics.frames);
  metricSimulatorFrames.textContent = String(frame.metrics.simulator_frames);
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
  handleSimulatorFrame(await request('/api/simulator/frame'));
}

async function patchSimulatorControls() {
  const snapshot = await request('/api/simulator', {
    method: 'PATCH',
    body: JSON.stringify({
      volume: Number(simVolume.value),
      beat_progress: Number(simBeatProgress.value),
      flash_active: simFlashActive.checked,
      primary: fromColorInput(palettePrimary.value),
      secondary: fromColorInput(paletteSecondary.value),
      accent: fromColorInput(paletteAccent.value),
    }),
  });
  updateSnapshot(snapshot);
  handleSimulatorFrame(await request('/api/simulator/frame'));
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
  handleSimulatorFrame(await request('/api/simulator/frame'));
}

function connectSimulatorStream() {
  const scheme = window.location.protocol === 'https:' ? 'wss' : 'ws';
  const socket = new WebSocket(`${scheme}://${window.location.host}/ws/simulator`);
  streamStatus.textContent = 'stream connecting';

  socket.addEventListener('open', () => {
    streamStatus.textContent = 'stream connected';
  });

  socket.addEventListener('message', event => {
    handleSimulatorFrame(JSON.parse(event.data));
  });

  socket.addEventListener('close', () => {
    streamStatus.textContent = 'stream disconnected';
    window.setTimeout(connectSimulatorStream, 1000);
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
    flashSpeedValue.textContent = flashSpeed.value;
    await patchRuntimeControls();
  });
}

for (const input of [
  simVolume,
  simBeatProgress,
  simFlashActive,
  palettePrimary,
  paletteSecondary,
  paletteAccent,
]) {
  input?.addEventListener('input', async () => {
    simVolumeValue.textContent = simVolume.value;
    simBeatProgressValue.textContent = simBeatProgress.value;
    await patchSimulatorControls();
  });
}

window.addEventListener('resize', redrawLatestSimulatorFrame);

await loadDomeLayout();
await refreshState();
connectSimulatorStream();
