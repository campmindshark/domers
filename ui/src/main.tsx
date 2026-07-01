import React from 'react';
import { flushSync } from 'react-dom';
import { createRoot } from 'react-dom/client';

import './styles.css';

const visualizerOptions = [
  'Volume',
  'Radial',
  'Race',
  'Snakes',
  'Quaternion Test',
  'Quaternion Multi Test',
  'Quaternion Paintbrush',
  'Splat',
  'TV Static',
];

function VisualizerSelect({ id, name }: { id: string; name: string }) {
  return (
    <select id={id} name={name}>
      {visualizerOptions.map((label, value) => (
        <option key={label} value={value}>
          {label}
        </option>
      ))}
    </select>
  );
}

function ConfigEditor() {
  return (
    <section id="config-drawer" className="config-drawer" aria-label="Config Editor">
      <h2>Config Editor</h2>
      <p className="drawer-intro">
        Edit the full native configuration as JSON. Applying config restarts
        live input adapters when the engine is running.
      </p>
      <fieldset className="config-panel">
        <legend>Input And Tempo</legend>
        <div className="config-section-grid">
          <section className="config-card" aria-label="Audio input config">
            <h3>Audio</h3>
            <label className="config-field">
              <span className="config-field-label">Audio source</span>
              <select id="config-audio-source" name="configAudioSource" />
              <span id="config-audio-device-hint" className="field-hint" />
            </label>
            <label className="config-field">
              <span className="config-field-label">Bridge address (UDP)</span>
              <span className="field-hint">Where the external bridge or simulator posts volume. Used when the source is "UDP bridge feed".</span>
              <input id="config-audio-bind" name="configAudioBind" type="text" placeholder="127.0.0.1:9001" />
            </label>
          </section>
          <section className="config-card" aria-label="MIDI input config">
            <h3>MIDI</h3>
            <label className="config-field">
              <span className="config-field-label">UDP bind</span>
              <span className="field-hint">Address for MIDI command datagrams.</span>
              <input id="config-midi-bind" name="configMidiBind" type="text" placeholder="127.0.0.1:9002" />
            </label>
            <label className="checkbox-field">
              <input id="config-midi-native-enabled" name="configMidiNativeEnabled" type="checkbox" />
              <span>Native midir capture</span>
            </label>
            <label className="config-field">
              <span className="config-field-label">Native MIDI port</span>
              <span className="field-hint">Optional exact port name. First port is used when blank.</span>
              <input id="config-midi-device-id" name="configMidiDeviceId" type="text" placeholder="Controller Port Name" />
            </label>
            <p className="midi-bindings-summary">
              <span className="config-field-label">Bindings</span>
              <span id="config-midi-bindings">none</span>
            </p>
          </section>
          <section className="config-card" aria-label="Orientation input config">
            <h3>Orientation</h3>
            <label className="config-field">
              <span className="config-field-label">UDP bind</span>
              <span className="field-hint">Address for controller orientation packets.</span>
              <input id="config-orientation-bind" name="configOrientationBind" type="text" placeholder="127.0.0.1:9003" />
            </label>
          </section>
          <section className="config-card" aria-label="Tempo and Madmom config">
            <h3>Tempo / Madmom</h3>
            <label className="config-field">
              <span className="config-field-label">Tempo source</span>
              <select id="config-tempo-source" name="configTempoSource">
                <option value="human">Human</option>
                <option value="madmom">Madmom</option>
                <option value="link">DJ Link / Carabiner</option>
              </select>
            </label>
            <label className="config-field">
              <span className="config-field-label">Command</span>
              <input id="config-madmom-command" name="configMadmomCommand" type="text" placeholder="python BeatTracker.py" />
            </label>
            <label className="config-field">
              <span className="config-field-label">Tracker</span>
              <input id="config-madmom-tracker" name="configMadmomTracker" type="text" placeholder="DBNBeatTrackingProcessor" />
            </label>
            <label className="config-field">
              <span className="config-field-label">Audio input index</span>
              <input id="config-madmom-audio-index" name="configMadmomAudioIndex" type="number" min="0" placeholder="0" />
            </label>
            <label className="config-field">
              <span className="config-field-label">DJ Link sidecar command</span>
              <span className="field-hint">macOS/Linux command that prints tempo lines such as "LINK 120 0.25".</span>
              <input id="config-carabiner-command" name="configCarabinerCommand" type="text" placeholder="carabiner" />
            </label>
            <label className="config-field">
              <span className="config-field-label">DJ Link sidecar args</span>
              <span className="field-hint">Whitespace-separated arguments for the DJ Link sidecar.</span>
              <input id="config-carabiner-args" name="configCarabinerArgs" type="text" placeholder="--stdout-tempo" />
            </label>
          </section>
        </div>
        <button id="apply-structured-config" type="button">
          Apply
        </button>
      </fieldset>
      <fieldset className="config-panel">
        <legend>Output And Layout</legend>
        <div className="config-section-grid output-config-grid">
          <section className="config-card" aria-label="Dome output config">
            <h3>Dome</h3>
            <label className="checkbox-field">
              <input id="config-dome-enabled" name="configDomeEnabled" type="checkbox" />
              <span>Hardware enabled</span>
            </label>
            <label className="checkbox-field">
              <input id="config-dome-simulation-enabled" name="configDomeSimulationEnabled" type="checkbox" />
              <span>Simulator enabled</span>
            </label>
            <label className="config-field">
              <span className="config-field-label">OPC address</span>
              <input id="config-dome-opc-address" name="configDomeOpcAddress" type="text" placeholder="127.0.0.1:7890" />
            </label>
            <label className="config-field">
              <span className="config-field-label">Brightness</span>
              <div className="slider-number-field">
                <input id="config-dome-brightness-slider" name="configDomeBrightnessSlider" type="range" min="0" max="1" step="0.01" />
                <input id="config-dome-brightness" name="configDomeBrightness" type="number" min="0" max="1" step="0.01" />
              </div>
            </label>
          </section>
          <section className="config-card" aria-label="Bar output config">
            <h3>Bar</h3>
            <label className="checkbox-field">
              <input id="config-bar-enabled" name="configBarEnabled" type="checkbox" />
              <span>Hardware enabled</span>
            </label>
            <label className="checkbox-field">
              <input id="config-bar-simulation-enabled" name="configBarSimulationEnabled" type="checkbox" />
              <span>Simulator enabled</span>
            </label>
            <div className="inline-field-grid">
              <label className="config-field">
                <span className="config-field-label">Infinity length</span>
                <input id="config-bar-infinity-length" name="configBarInfinityLength" type="number" min="0" step="1" />
              </label>
              <label className="config-field">
                <span className="config-field-label">Infinity width</span>
                <input id="config-bar-infinity-width" name="configBarInfinityWidth" type="number" min="0" step="1" />
              </label>
              <label className="config-field">
                <span className="config-field-label">Runner length</span>
                <input id="config-bar-runner-length" name="configBarRunnerLength" type="number" min="0" step="1" />
              </label>
            </div>
            <label className="config-field">
              <span className="config-field-label">Brightness</span>
              <div className="slider-number-field">
                <input id="config-bar-brightness-slider" name="configBarBrightnessSlider" type="range" min="0" max="1" step="0.01" />
                <input id="config-bar-brightness" name="configBarBrightness" type="number" min="0" max="1" step="0.01" />
              </div>
            </label>
          </section>
          <section className="config-card" aria-label="Stage output config">
            <h3>Stage</h3>
            <label className="checkbox-field">
              <input id="config-stage-enabled" name="configStageEnabled" type="checkbox" />
              <span>Hardware enabled</span>
            </label>
            <label className="checkbox-field">
              <input id="config-stage-simulation-enabled" name="configStageSimulationEnabled" type="checkbox" />
              <span>Simulator enabled</span>
            </label>
            <label className="config-field">
              <span className="config-field-label">OPC address</span>
              <input id="config-stage-opc-address" name="configStageOpcAddress" type="text" placeholder="127.0.0.1:7891" />
            </label>
            <label className="config-field">
              <span className="config-field-label">Brightness</span>
              <div className="slider-number-field">
                <input id="config-stage-brightness-slider" name="configStageBrightnessSlider" type="range" min="0" max="1" step="0.01" />
                <input id="config-stage-brightness" name="configStageBrightness" type="number" min="0" max="1" step="0.01" />
              </div>
            </label>
          </section>
          <section className="config-card stage-layout-card" aria-label="Stage layout config">
            <h3>Stage Layout</h3>
            <p className="field-hint">These side lengths belong to the stage. They are physical layout values, not dome settings.</p>
            <div className="config-field stage-side-lengths-editor">
              <span className="config-field-label">Side lengths</span>
              <span className="field-hint">One numeric input per stage side. Values update the full config JSON when you apply structured config.</span>
              <input id="config-stage-side-lengths" name="configStageSideLengths" type="hidden" />
              <p id="config-stage-side-lengths-summary" className="side-lengths-summary">no sides configured</p>
              <div id="config-stage-side-lengths-grid" className="side-lengths-grid" aria-label="Stage side lengths" />
            </div>
          </section>
        </div>
      </fieldset>
      <div className="config-actions">
        <button id="reload-config" type="button">
          Reload
        </button>
        <button id="apply-config" type="button">
          Apply
        </button>
        <span id="config-status">not loaded</span>
      </div>
      <label className="config-field json-config-field">
        <span className="config-field-label">Full JSON config</span>
        <textarea id="config-editor" className="config-editor" spellCheck={false} rows={16} />
      </label>
    </section>
  );
}

function RuntimeControls() {
  return (
    <section aria-label="Runtime Controls">
      <h2>Runtime Controls</h2>
      <label>
        Dome visualizer
        <VisualizerSelect id="dome-active-vis" name="domeActiveVis" />
      </label>
      <label>
        Active palette
        <select id="palette-index" name="colorPaletteIndex">
          {Array.from({ length: 8 }, (_, index) => (
            <option key={index} value={index}>
              Palette {index + 1}
            </option>
          ))}
        </select>
      </label>
      <label>
        Flash speed
        <input id="flash-speed" name="flashSpeed" type="range" min="0" max="8" step="0.125" defaultValue="0" />
        <output id="flash-speed-value" htmlFor="flash-speed">
          0
        </output>
      </label>
    </section>
  );
}

function PaletteDrawer() {
  return (
    <section id="palette-drawer" aria-label="Palettes">
      <h2>Palettes</h2>
      <section id="palette-grid" aria-label="Palette colors" className="palette-grid" data-palette-editor="64-entry-gradient" />
    </section>
  );
}

function InputsPanel() {
  return (
    <section id="inputs-drawer" aria-label="Inputs">
      <h2>Inputs</h2>
      <div className="config-section-grid">
        <section className="config-card" aria-label="BPM">
          <h3>BPM</h3>
          <div className="inline-field-grid">
            <button id="tap-tempo" type="button">
              Tap Tempo
            </button>
            <button id="reset-tempo" type="button">
              Reset Tempo
            </button>
          </div>
          <div className="inline-field-grid">
            <label className="config-field">
              <span className="config-field-label">Manual BPM</span>
              <input id="manual-bpm" type="number" min="1" max="600" step="0.1" placeholder="120" />
            </label>
            <button id="apply-bpm" type="button">
              Apply BPM
            </button>
          </div>
          <p>
            BPM: <span id="tempo-bpm">[none]</span>
          </p>
          <p>
            Tap counter: <span id="tap-counter">Tap</span>
          </p>
        </section>
        <section className="config-card" aria-label="Wands">
          <h3>Wands</h3>
          <button id="orientation-calibrate" type="button">
            Calibrate Orientation
          </button>
        </section>
        <section className="config-card" aria-label="MIDI">
          <h3>MIDI</h3>
          <ol id="midi-log" className="device-list">
            <li>none</li>
          </ol>
        </section>
      </div>
    </section>
  );
}

function DebugVisualsPanel() {
  return (
    <section id="debug-visuals-drawer" aria-label="Debug Visuals">
      <h2>Debug Visuals</h2>
      <label>
        Dome diagnostic
        <select id="dome-test-pattern" name="domeTestPattern">
          <option value="0">Off</option>
          <option value="1">Flash Colors</option>
          <option value="2">Strut Iteration</option>
          <option value="3">Strand Test</option>
          <option value="4">Full Color Flash</option>
        </select>
      </label>
      <label>
        Bar diagnostic
        <select id="bar-test-pattern" name="barTestPattern">
          <option value="0">Off</option>
          <option value="1">Flash Colors</option>
        </select>
      </label>
      <label>
        Stage diagnostic
        <select id="stage-test-pattern" name="stageTestPattern">
          <option value="0">Off</option>
          <option value="1">Flash Colors</option>
        </select>
      </label>
    </section>
  );
}

function SimulatorFrameView({ streamText }: { streamText: string }) {
  return (
    <>
      <section aria-label="Metrics">
        <span id="stream-status">{streamText}</span>
        <p>
          Engine frames: <span id="metric-frames" className="metric">0</span>
        </p>
        <p>
          Simulator frames: <span id="metric-simulator-frames" className="metric">0</span>
        </p>
      </section>
      <section aria-label="Simulator">
        <canvas id="dome-simulator" width="750" height="750" data-frame-source="websocket" />
        <div className="support-simulator-grid">
          <section aria-label="Bar Simulator">
            <h3>Bar Simulator</h3>
            <canvas id="bar-simulator" className="support-simulator-canvas" width="640" height="220" data-frame-source="websocket" />
          </section>
          <section aria-label="Stage Simulator">
            <h3>Stage Simulator</h3>
            <canvas id="stage-simulator" className="support-simulator-canvas" width="640" height="360" data-frame-source="websocket" />
          </section>
        </div>
      </section>
    </>
  );
}

function OrientationPreviewControls({ idPrefix, sandbox = false }: { idPrefix: string; sandbox?: boolean }) {
  const fieldNamePrefix = sandbox ? 'sandboxOrientation' : 'orientation';
  return (
    <fieldset className="orientation-preview-controls">
      <legend>Orientation Preview</legend>
      <label className="checkbox-field">
        <input id={`${idPrefix}-orientation-enabled`} name={`${fieldNamePrefix}Enabled`} type="checkbox" />
        <span>Use yaw / pitch / roll override</span>
      </label>
      <div className="inline-field-grid">
        <label>
          Yaw
          <input id={`${idPrefix}-orientation-yaw`} name={`${fieldNamePrefix}Yaw`} type="range" min="-180" max="180" step="1" defaultValue="0" />
          <output id={`${idPrefix}-orientation-yaw-value`} htmlFor={`${idPrefix}-orientation-yaw`}>0 deg</output>
        </label>
        <label>
          Pitch
          <input id={`${idPrefix}-orientation-pitch`} name={`${fieldNamePrefix}Pitch`} type="range" min="-180" max="180" step="1" defaultValue="-90" />
          <output id={`${idPrefix}-orientation-pitch-value`} htmlFor={`${idPrefix}-orientation-pitch`}>-90 deg</output>
        </label>
        <label>
          Roll
          <input id={`${idPrefix}-orientation-roll`} name={`${fieldNamePrefix}Roll`} type="range" min="-180" max="180" step="1" defaultValue="0" />
          <output id={`${idPrefix}-orientation-roll-value`} htmlFor={`${idPrefix}-orientation-roll`}>0 deg</output>
        </label>
      </div>
    </fieldset>
  );
}

function LivePreviewPanel() {
  return (
    <section id="preview-drawer" aria-label="Live Preview">
      <h2>Live Preview</h2>
      <p><a href="/simulator">Open isolated simulator</a></p>
      <SimulatorFrameView streamText="preview WebSocket disconnected" />
    </section>
  );
}

function OpcTargetsFooter() {
  return (
    <footer className="opc-targets-footer" aria-label="OPC Targets and Device Status">
      <h2>Runtime Status</h2>
      <section aria-label="OPC Targets">
        <h3>OPC Targets</h3>
        <div className="status-grid">
          <p className="target-status">
            <strong>Dome / Bar</strong>
            <span id="hardware-dome">no address configured</span>
          </p>
          <p className="target-status">
            <strong>Stage</strong>
            <span id="hardware-stage">no address configured</span>
          </p>
        </div>
      </section>
      <section aria-label="Device Status">
        <h3>Device Status</h3>
        <div className="status-grid">
          <p className="target-status">
            <strong>Audio</strong>
            <span id="input-audio">disabled</span>
          </p>
          <p className="target-status">
            <strong>MIDI</strong>
            <span id="input-midi">disabled</span>
          </p>
          <p className="target-status">
            <strong>MIDI Levels</strong>
            <span id="input-midi-levels">none</span>
          </p>
          <p className="target-status">
            <strong>Orientation</strong>
            <span id="input-orientation">disabled</span>
          </p>
          <p className="target-status">
            <strong>Madmom</strong>
            <span id="input-madmom">disabled</span>
          </p>
          <p className="target-status">
            <strong>DJ Link</strong>
            <span id="input-link">disabled</span>
          </p>
        </div>
        <section aria-label="Orientation Devices">
          <h4>Orientation Devices</h4>
          <ul id="orientation-devices" className="device-list">
            <li>none</li>
          </ul>
        </section>
      </section>
    </footer>
  );
}

function ControlApp() {
  return (
    <main data-domers-app>
      <header className="app-shell-header">
        <h1>MindShark Dome Control Panel</h1>
        <section className="engine-controls" aria-label="Engine">
          <button id="start-engine" type="button">Start</button>
          <button id="stop-engine" type="button">Stop</button>
          <span id="engine-status">stopped</span>
        </section>
      </header>
      <div className="app-shell-content" aria-label="Operator Tabs">
        <nav className="operator-tabs" aria-label="Operator sections" role="tablist">
          <button id="runtime-tab" className="operator-tab is-active" type="button" role="tab" aria-controls="runtime-panel" aria-selected="true" data-tab-target="runtime-panel">Runtime</button>
          <button id="palettes-tab" className="operator-tab" type="button" role="tab" aria-controls="palettes-panel" aria-selected="false" data-tab-target="palettes-panel">Palettes</button>
          <button id="config-tab" className="operator-tab" type="button" role="tab" aria-controls="config-panel" aria-selected="false" data-tab-target="config-panel">Config</button>
        </nav>
        <section id="runtime-panel" className="operator-tab-panel is-active runtime-panel-grid" role="tabpanel" aria-labelledby="runtime-tab">
          <RuntimeControls />
          <InputsPanel />
          <DebugVisualsPanel />
          <LivePreviewPanel />
        </section>
        <section id="palettes-panel" className="operator-tab-panel" role="tabpanel" aria-labelledby="palettes-tab" hidden>
          <PaletteDrawer />
        </section>
        <section id="config-panel" className="operator-tab-panel" role="tabpanel" aria-labelledby="config-tab" hidden>
          <ConfigEditor />
        </section>
      </div>
      <OpcTargetsFooter />
    </main>
  );
}

function SimulatorControls() {
  return (
    <section aria-label="Simulator Controls">
      <h2>Simulator Controls</h2>
      <label>
        Dome visualizer
        <VisualizerSelect id="sandbox-dome-active-vis" name="sandboxDomeActiveVis" />
      </label>
      <label>
        Audio volume preview
        <input id="sandbox-volume" name="sandboxVolume" type="range" min="0" max="1" step="0.01" defaultValue="0.7" />
        <output id="sandbox-volume-value" htmlFor="sandbox-volume">
          0.7
        </output>
      </label>
      <label>
        Beat phase preview
        <input id="sandbox-beat-progress" name="sandboxBeatProgress" type="range" min="0" max="1" step="0.01" defaultValue="0.25" />
        <output id="sandbox-beat-progress-value" htmlFor="sandbox-beat-progress">
          0.25
        </output>
      </label>
      <label>
        <input id="sandbox-flash-active" name="sandboxFlashActive" type="checkbox" /> Flash overlay active preview
      </label>
      <OrientationPreviewControls idPrefix="sandbox" sandbox />
      <fieldset>
        <legend>Simulator palette colors</legend>
        <div className="swatches">
          <label>
            Color 1
            <input id="sandbox-palette-primary" name="sandboxPalettePrimary" type="color" defaultValue="#00ff00" />
          </label>
          <label>
            Color 2
            <input id="sandbox-palette-secondary" name="sandboxPaletteSecondary" type="color" defaultValue="#0080ff" />
          </label>
          <label>
            Color 3 / flash
            <input id="sandbox-palette-accent" name="sandboxPaletteAccent" type="color" defaultValue="#ff4080" />
          </label>
        </div>
      </fieldset>
    </section>
  );
}

function SimulatorApp() {
  return (
    <main data-domers-simulator>
      <h1>MindShark Dome Simulator</h1>
      <p>
        <a href="/">Back to controls</a>
      </p>
      <SimulatorControls />
      <SimulatorFrameView streamText="simulator sandbox" />
    </main>
  );
}

function App() {
  return document.body.dataset.page === 'simulator' ? <SimulatorApp /> : <ControlApp />;
}

const rootElement = document.querySelector('#root');
if (!rootElement) {
  throw new Error('Missing React root element');
}

flushSync(() => {
  createRoot(rootElement).render(<App />);
});

await import('../main.mjs');
