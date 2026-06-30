const status = document.querySelector('#engine-status');
const streamStatus = document.querySelector('#stream-status');
const activeVisualizer = document.querySelector('#dome-active-vis');
const flashSpeed = document.querySelector('#flash-speed');
const flashSpeedValue = document.querySelector('#flash-speed-value');
const metricFrames = document.querySelector('#metric-frames');
const metricSimulatorFrames = document.querySelector('#metric-simulator-frames');
const canvas = document.querySelector('#dome-simulator');
const context = canvas?.getContext('2d');

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

function updateSnapshot(snapshot) {
  status.textContent = snapshot.running ? 'running' : 'stopped';
  metricFrames.textContent = String(snapshot.metrics.frames);
  metricSimulatorFrames.textContent = String(snapshot.metrics.simulator_frames);
  activeVisualizer.value = String(snapshot.config.dome_active_vis);
}

function drawFrame(colors) {
  if (!context || !colors.length) {
    return;
  }

  const cellCount = Math.ceil(Math.sqrt(colors.length));
  const cellSize = canvas.width / cellCount;
  context.clearRect(0, 0, canvas.width, canvas.height);

  colors.forEach((color, index) => {
    const x = (index % cellCount) * cellSize;
    const y = Math.floor(index / cellCount) * cellSize;
    context.fillStyle = `#${color.toString(16).padStart(6, '0')}`;
    context.fillRect(x, y, Math.ceil(cellSize), Math.ceil(cellSize));
  });
}

function handleSimulatorFrame(frame) {
  metricFrames.textContent = String(frame.metrics.frames);
  metricSimulatorFrames.textContent = String(frame.metrics.simulator_frames);

  for (const command of frame.commands) {
    if (command.kind === 'frame') {
      drawFrame(command.colors);
    }
  }
}

async function refreshState() {
  updateSnapshot(await request('/api/state'));
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

activeVisualizer?.addEventListener('change', async () => {
  updateSnapshot(
    await request('/api/config/dome', {
      method: 'PATCH',
      body: JSON.stringify({ active_visualizer: Number(activeVisualizer.value) }),
    }),
  );
});

flashSpeed?.addEventListener('input', () => {
  flashSpeedValue.textContent = flashSpeed.value;
});

await refreshState();
connectSimulatorStream();
