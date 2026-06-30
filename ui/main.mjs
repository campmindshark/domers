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
const configEditor = document.querySelector('#config-editor');
const configStatus = document.querySelector('#config-status');
const configAudioBind = document.querySelector('#config-audio-bind');
const configAudioDeviceId = document.querySelector('#config-audio-device-id');
const configMidiBind = document.querySelector('#config-midi-bind');
const configOrientationBind = document.querySelector('#config-orientation-bind');
const configTempoSource = document.querySelector('#config-tempo-source');
const configMadmomCommand = document.querySelector('#config-madmom-command');
const configMadmomTracker = document.querySelector('#config-madmom-tracker');
const configMadmomAudioIndex = document.querySelector('#config-madmom-audio-index');
const configMidiBindings = document.querySelector('#config-midi-bindings');
const configDomeEnabled = document.querySelector('#config-dome-enabled');
const configDomeSimulationEnabled = document.querySelector('#config-dome-simulation-enabled');
const configDomeOpcAddress = document.querySelector('#config-dome-opc-address');
const configDomeBrightness = document.querySelector('#config-dome-brightness');
const configBarEnabled = document.querySelector('#config-bar-enabled');
const configBarSimulationEnabled = document.querySelector('#config-bar-simulation-enabled');
const configBarInfinityLength = document.querySelector('#config-bar-infinity-length');
const configBarInfinityWidth = document.querySelector('#config-bar-infinity-width');
const configBarRunnerLength = document.querySelector('#config-bar-runner-length');
const configBarBrightness = document.querySelector('#config-bar-brightness');
const configStageEnabled = document.querySelector('#config-stage-enabled');
const configStageSimulationEnabled = document.querySelector('#config-stage-simulation-enabled');
const configStageOpcAddress = document.querySelector('#config-stage-opc-address');
const configStageBrightness = document.querySelector('#config-stage-brightness');
const configStageSideLengths = document.querySelector('#config-stage-side-lengths');
const simVolume = document.querySelector('#sim-volume');
const simVolumeValue = document.querySelector('#sim-volume-value');
const simBeatProgress = document.querySelector('#sim-beat-progress');
const simBeatProgressValue = document.querySelector('#sim-beat-progress-value');
const simFlashActive = document.querySelector('#sim-flash-active');
const paletteIndex = document.querySelector('#palette-index');
const paletteGrid = document.querySelector('#palette-grid');
let paletteControls = [];
const inputAudio = document.querySelector('#input-audio');
const inputMidi = document.querySelector('#input-midi');
const inputOrientation = document.querySelector('#input-orientation');
const inputMadmom = document.querySelector('#input-madmom');
const orientationDevices = document.querySelector('#orientation-devices');
const midiLog = document.querySelector('#midi-log');
const tempoBpm = document.querySelector('#tempo-bpm');
const tapCounter = document.querySelector('#tap-counter');
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
const barSimulator = document.querySelector('#bar-simulator');
const stageSimulator = document.querySelector('#stage-simulator');
const isDedicatedSimulatorPage = document.body?.dataset.page === 'simulator';
const SPECTRUM_CANVAS_SIZE = 750;
const SPECTRUM_PROJECTION_OFFSET = 10;
const SPECTRUM_PROJECTION_SPAN = 690;
let domeLayout;
let domeLedPoints = [];
let latestSimulatorFrame;
let simulatorStarted = false;
let simulatorSocket;

renderPaletteEditor();

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

async function loadFullConfig() {
  if (!configEditor) {
    return;
  }
  const config = await request('/api/config');
  configEditor.value = JSON.stringify(config, null, 2);
  updateStructuredConfigFields(config);
  if (configStatus) {
    configStatus.textContent = 'loaded';
  }
}

function updateStructuredConfigFields(config) {
  if (configAudioBind) {
    configAudioBind.value = config.inputs?.audio?.bind ?? '';
  }
  if (configAudioDeviceId) {
    configAudioDeviceId.value = config.inputs?.audio?.device_id ?? '';
  }
  if (configMidiBind) {
    configMidiBind.value = config.inputs?.midi?.bind ?? '';
  }
  if (configOrientationBind) {
    configOrientationBind.value = config.inputs?.orientation?.bind ?? '';
  }
  if (configTempoSource) {
    configTempoSource.value = config.tempo?.source ?? 'human';
  }
  if (configMadmomCommand) {
    configMadmomCommand.value = config.madmom?.command ?? '';
  }
  if (configMadmomTracker) {
    configMadmomTracker.value = config.madmom?.tracker ?? '';
  }
  if (configMadmomAudioIndex) {
    configMadmomAudioIndex.value = config.madmom?.audio_input_index ?? '';
  }
  if (configMidiBindings) {
    const bindings = config.inputs?.midi?.bindings ?? [];
    configMidiBindings.textContent = bindings.length
      ? bindings.map(binding => `${binding.command_kind}:${binding.index}->${binding.action}`).join(', ')
      : 'none';
  }
  if (configDomeEnabled) {
    configDomeEnabled.checked = Boolean(config.dome?.enabled);
  }
  if (configDomeSimulationEnabled) {
    configDomeSimulationEnabled.checked = Boolean(config.dome?.simulation_enabled);
  }
  if (configDomeOpcAddress) {
    configDomeOpcAddress.value = config.dome?.opc_address ?? '';
  }
  if (configDomeBrightness) {
    configDomeBrightness.value = config.dome?.brightness ?? '';
  }
  if (configBarEnabled) {
    configBarEnabled.checked = Boolean(config.bar?.enabled);
  }
  if (configBarSimulationEnabled) {
    configBarSimulationEnabled.checked = Boolean(config.bar?.simulation_enabled);
  }
  if (configBarInfinityLength) {
    configBarInfinityLength.value = config.bar?.infinity_length ?? '';
  }
  if (configBarInfinityWidth) {
    configBarInfinityWidth.value = config.bar?.infinity_width ?? '';
  }
  if (configBarRunnerLength) {
    configBarRunnerLength.value = config.bar?.runner_length ?? '';
  }
  if (configBarBrightness) {
    configBarBrightness.value = config.bar?.brightness ?? '';
  }
  if (configStageEnabled) {
    configStageEnabled.checked = Boolean(config.stage?.enabled);
  }
  if (configStageSimulationEnabled) {
    configStageSimulationEnabled.checked = Boolean(config.stage?.simulation_enabled);
  }
  if (configStageOpcAddress) {
    configStageOpcAddress.value = config.stage?.opc_address ?? '';
  }
  if (configStageBrightness) {
    configStageBrightness.value = config.stage?.brightness ?? '';
  }
  if (configStageSideLengths) {
    configStageSideLengths.value = (config.stage?.side_lengths ?? []).join(', ');
  }
}

function readConfigEditor() {
  if (!configEditor) {
    return undefined;
  }
  return JSON.parse(configEditor.value);
}

function writeOptionalString(target, key, value) {
  const trimmed = value.trim();
  if (trimmed) {
    target[key] = trimmed;
  } else {
    delete target[key];
  }
}

function numberOrFallback(value, fallback) {
  const trimmed = value.trim();
  if (!trimmed) {
    return fallback;
  }
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function integerOrFallback(value, fallback) {
  const parsed = numberOrFallback(value, fallback);
  return Number.isInteger(parsed) && parsed >= 0 ? parsed : fallback;
}

function parseIntegerList(value, fallback) {
  const trimmed = value.trim();
  if (!trimmed) {
    return fallback;
  }
  const parsed = trimmed.split(/[\s,]+/).filter(Boolean).map(Number);
  if (!parsed.length || parsed.some(item => !Number.isInteger(item) || item < 0)) {
    return fallback;
  }
  return parsed;
}

function updateConfigFromStructuredFields() {
  const config = readConfigEditor();
  config.inputs ??= {};
  config.inputs.audio ??= {};
  config.inputs.midi ??= {};
  config.inputs.orientation ??= {};
  config.tempo ??= {};
  config.madmom ??= {};
  config.dome ??= {};
  config.bar ??= {};
  config.stage ??= {};
  writeOptionalString(config.inputs.audio, 'bind', configAudioBind?.value ?? '');
  writeOptionalString(config.inputs.audio, 'device_id', configAudioDeviceId?.value ?? '');
  writeOptionalString(config.inputs.midi, 'bind', configMidiBind?.value ?? '');
  writeOptionalString(config.inputs.orientation, 'bind', configOrientationBind?.value ?? '');
  config.tempo.source = configTempoSource?.value ?? 'human';
  config.madmom.command = configMadmomCommand?.value?.trim() || 'DBNBeatTracker';
  writeOptionalString(config.madmom, 'tracker', configMadmomTracker?.value ?? '');
  const audioIndex = configMadmomAudioIndex?.value?.trim() ?? '';
  if (audioIndex) {
    config.madmom.audio_input_index = Number(audioIndex);
  } else {
    delete config.madmom.audio_input_index;
  }
  config.dome.enabled = Boolean(configDomeEnabled?.checked);
  config.dome.simulation_enabled = Boolean(configDomeSimulationEnabled?.checked);
  config.dome.opc_address = configDomeOpcAddress?.value?.trim() ?? '';
  config.dome.brightness = numberOrFallback(configDomeBrightness?.value ?? '', config.dome.brightness ?? 0);
  config.bar.enabled = Boolean(configBarEnabled?.checked);
  config.bar.simulation_enabled = Boolean(configBarSimulationEnabled?.checked);
  config.bar.infinity_length = integerOrFallback(configBarInfinityLength?.value ?? '', config.bar.infinity_length ?? 0);
  config.bar.infinity_width = integerOrFallback(configBarInfinityWidth?.value ?? '', config.bar.infinity_width ?? 0);
  config.bar.runner_length = integerOrFallback(configBarRunnerLength?.value ?? '', config.bar.runner_length ?? 0);
  config.bar.brightness = numberOrFallback(configBarBrightness?.value ?? '', config.bar.brightness ?? 0);
  config.stage.enabled = Boolean(configStageEnabled?.checked);
  config.stage.simulation_enabled = Boolean(configStageSimulationEnabled?.checked);
  config.stage.opc_address = configStageOpcAddress?.value?.trim() ?? '';
  config.stage.brightness = numberOrFallback(configStageBrightness?.value ?? '', config.stage.brightness ?? 0);
  config.stage.side_lengths = parseIntegerList(configStageSideLengths?.value ?? '', config.stage.side_lengths ?? []);
  configEditor.value = JSON.stringify(config, null, 2);
  updateStructuredConfigFields(config);
}

async function applyFullConfig() {
  if (!configEditor) {
    return;
  }
  let config;
  try {
    config = JSON.parse(configEditor.value);
  } catch (error) {
    if (configStatus) {
      configStatus.textContent = `invalid JSON: ${error.message}`;
    }
    return;
  }
  updateSnapshot(await request('/api/config', {
    method: 'PATCH',
    body: JSON.stringify(config),
  }));
  if (configStatus) {
    configStatus.textContent = 'applied';
  }
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
  updateInputStatus(snapshot.inputs);
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
  updatePaletteInputs(snapshot);
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

function updateInputStatus(inputs) {
  if (!inputs) {
    return;
  }
  updateInputAdapterStatus(inputAudio, inputs.audio_adapter, `volume ${inputs.volume ?? 'n/a'}`);
  updateInputAdapterStatus(inputMidi, inputs.midi_adapter, `${inputs.midi_commands ?? 0} commands`);
  updateInputAdapterStatus(
    inputOrientation,
    inputs.orientation_adapter,
    `${inputs.orientation_devices?.length ?? 0} devices, last ${inputs.last_orientation ?? 'none'}`,
  );
  updateInputAdapterStatus(inputMadmom, inputs.madmom_adapter, `${inputs.madmom_beats ?? 0} beats`);
  updateOrientationDevices(inputs.orientation_devices ?? []);
  updateMidiLog(inputs.midi_log ?? []);
  if (tempoBpm) {
    tempoBpm.textContent = inputs.bpm ?? '[none]';
  }
  if (tapCounter) {
    tapCounter.textContent = inputs.tap_counter_text ?? 'Tap';
    tapCounter.dataset.active = String(Boolean(inputs.tap_counter_active));
  }
}

function updateInputAdapterStatus(element, adapter, detail) {
  if (!element || !adapter) {
    return;
  }
  if (!adapter.enabled) {
    element.textContent = 'disabled';
    return;
  }
  const target = adapter.target ?? 'configured';
  const error = adapter.last_error ? ` - last error: ${adapter.last_error}` : '';
  element.textContent = `${target} - ${adapter.events ?? 0} events, ${detail}${error}`;
}

function updateOrientationDevices(devices) {
  if (!orientationDevices) {
    return;
  }
  orientationDevices.replaceChildren();
  if (!devices.length) {
    const item = document.createElement('li');
    item.textContent = 'none';
    orientationDevices.append(item);
    return;
  }
  for (const device of devices) {
    const item = document.createElement('li');
    const rotation = device.current_rotation ?? {};
    const speed = device.has_speed ? `, speed ${formatNumber(device.avg_distance_short)}` : '';
    item.textContent = `#${device.device_id} ${device.kind} action ${device.action_flag} rotation (${formatNumber(rotation.x)}, ${formatNumber(rotation.y)}, ${formatNumber(rotation.z)}, ${formatNumber(rotation.w)})${speed}`;
    orientationDevices.append(item);
  }
}

function updateMidiLog(entries) {
  if (!midiLog) {
    return;
  }
  midiLog.replaceChildren();
  if (!entries.length) {
    const item = document.createElement('li');
    item.textContent = 'none';
    midiLog.append(item);
    return;
  }
  for (const entry of entries.slice(-8).reverse()) {
    const item = document.createElement('li');
    const actions = entry.actions?.length ? entry.actions.join(', ') : 'no binding';
    item.textContent = `${entry.timestamp_ms}ms ${entry.kind} ${entry.index}=${formatNumber(entry.value)} -> ${actions}`;
    midiLog.append(item);
  }
}

function formatNumber(value) {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return 'n/a';
  }
  return value.toFixed(3);
}

function toColorInput(color) {
  return `#${color.toString(16).padStart(6, '0')}`;
}

function fromColorInput(color) {
  return Number.parseInt(color.replace('#', ''), 16);
}

function paletteColor(snapshot, paletteSlot, relativeIndex) {
  const absoluteIndex = paletteSlot * 8 + relativeIndex;
  return snapshot.config.color_palette.colors[absoluteIndex]?.color1 ?? 0;
}

function renderPaletteEditor() {
  if (!paletteGrid) {
    return;
  }
  paletteGrid.textContent = '';
  paletteControls = [];
  for (let paletteSlot = 0; paletteSlot < 8; paletteSlot += 1) {
    const card = document.createElement('fieldset');
    card.className = 'palette-card';
    const legend = document.createElement('legend');
    legend.textContent = `Palette ${paletteSlot + 1}`;
    card.append(legend);
    for (let colorIndex = 0; colorIndex < 8; colorIndex += 1) {
      const entry = document.createElement('div');
      entry.className = 'palette-entry';
      const title = document.createElement('strong');
      title.textContent = `Color ${colorIndex + 1}${colorIndex === 2 ? ' / flash' : ''}`;
      const color1 = paletteInput(paletteSlot, colorIndex, 'color1');
      const color2 = paletteInput(paletteSlot, colorIndex, 'color2');
      const gradient = document.createElement('input');
      gradient.type = 'checkbox';
      gradient.id = `palette-${paletteSlot + 1}-color-${colorIndex + 1}-gradient`;
      gradient.dataset.paletteIndex = String(paletteSlot);
      gradient.dataset.colorIndex = String(colorIndex);
      const gradientLabel = document.createElement('label');
      gradientLabel.append('Gradient ', gradient);
      entry.append(
        title,
        labelWithText('Color 1', color1),
        labelWithText('Color 2', color2),
        gradientLabel,
      );
      card.append(entry);
      paletteControls.push({ color1, color2, gradient });
    }
    paletteGrid.append(card);
  }
}

function paletteInput(paletteSlot, colorIndex, role) {
  const input = document.createElement('input');
  input.type = 'color';
  input.value = '#000000';
  input.id = `palette-${paletteSlot + 1}-color-${colorIndex + 1}-${role}`;
  input.dataset.paletteIndex = String(paletteSlot);
  input.dataset.colorIndex = String(colorIndex);
  input.dataset.paletteRole = role;
  return input;
}

function labelWithText(text, input) {
  const label = document.createElement('label');
  label.append(text, input);
  return label;
}

function updatePaletteInputs(snapshot) {
  for (const control of paletteControls) {
    const entry = paletteEntry(
      snapshot,
      Number(control.color1.dataset.paletteIndex),
      Number(control.color1.dataset.colorIndex),
    );
    control.color1.value = toColorInput(entry.color1 ?? 0);
    control.color2.value = toColorInput(entry.color2 ?? entry.color1 ?? 0);
    control.gradient.checked = entry.color2_enabled ?? false;
  }
}

function paletteEntry(snapshot, paletteSlot, relativeIndex) {
  const absoluteIndex = paletteSlot * 8 + relativeIndex;
  return snapshot.config.color_palette.colors[absoluteIndex] ?? {
    color1: 0,
    color2: 0,
    color2_enabled: false,
  };
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
  updateCommandList(barSimulator, frame.bar_commands ?? [], formatBarCommand);
  updateCommandList(stageSimulator, frame.stage_commands ?? [], formatStageCommand);
}

function updateCommandList(element, commands, formatter) {
  if (!element) {
    return;
  }
  element.replaceChildren();
  const visible = commands.filter(command => command.kind !== 'flush').slice(0, 24);
  if (!visible.length) {
    const item = document.createElement('li');
    item.textContent = 'none';
    element.append(item);
    return;
  }
  for (const command of visible) {
    const item = document.createElement('li');
    item.textContent = formatter(command);
    element.append(item);
  }
}

function formatBarCommand(command) {
  return `${command.is_runner ? 'runner' : 'bar'} led ${command.led_index} ${toColorInput(command.color)}`;
}

function formatStageCommand(command) {
  return `side ${command.side_index} layer ${command.layer_index} led ${command.led_index} ${toColorInput(command.color)}`;
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

async function patchPaletteColor(control) {
  const paletteSlot = Number(control.color1.dataset.paletteIndex);
  const relativeIndex = Number(control.color1.dataset.colorIndex);
  const snapshot = await request('/api/config/palette', {
    method: 'PATCH',
    body: JSON.stringify({
      color_palette_index: paletteSlot,
      relative_index: relativeIndex,
      color1: fromColorInput(control.color1.value),
      color2: fromColorInput(control.color2.value),
      color2_enabled: control.gradient.checked,
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

document.querySelector('#reload-config')?.addEventListener('click', loadFullConfig);

document.querySelector('#apply-config')?.addEventListener('click', async () => {
  await applyFullConfig();
  await refreshPreviewFrame();
});

document.querySelector('#apply-structured-config')?.addEventListener('click', async () => {
  try {
    updateConfigFromStructuredFields();
    await applyFullConfig();
    await refreshPreviewFrame();
  } catch (error) {
    if (configStatus) {
      configStatus.textContent = `structured config failed: ${error.message}`;
    }
  }
});

document.querySelector('#tap-tempo')?.addEventListener('click', async () => {
  updateSnapshot(await request('/api/input/tap', { method: 'POST' }));
  await refreshPreviewFrame();
});

document.querySelector('#reset-tempo')?.addEventListener('click', async () => {
  updateSnapshot(await request('/api/input/tempo/reset', { method: 'POST' }));
  await refreshPreviewFrame();
});

document.querySelector('#orientation-calibrate')?.addEventListener('click', async () => {
  updateSnapshot(await request('/api/input/orientation/calibrate', { method: 'POST' }));
  await refreshPreviewFrame();
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

for (const control of paletteControls) {
  for (const input of [control.color1, control.color2, control.gradient]) {
    input.addEventListener('input', async () => {
      await patchPaletteColor(control);
    });
  }
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
await loadFullConfig();
if (isDedicatedSimulatorPage) {
  await ensureSimulatorStarted();
}
