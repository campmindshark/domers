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
    <details id="config-drawer" aria-label="Config Editor">
      <summary>Config Editor</summary>
      <p>
        Edit the full native configuration as JSON. Applying config restarts
        live input adapters when the engine is running.
      </p>
      <fieldset>
        <legend>Input And Tempo Config</legend>
        <label>
          Audio UDP bind
          <input id="config-audio-bind" name="configAudioBind" type="text" />
        </label>
        <label>
          Audio device ID
          <input id="config-audio-device-id" name="configAudioDeviceId" type="text" />
        </label>
        <label>
          MIDI UDP bind
          <input id="config-midi-bind" name="configMidiBind" type="text" />
        </label>
        <label>
          Orientation UDP bind
          <input id="config-orientation-bind" name="configOrientationBind" type="text" />
        </label>
        <label>
          Tempo source
          <select id="config-tempo-source" name="configTempoSource">
            <option value="human">Human</option>
            <option value="madmom">Madmom</option>
            <option value="link_unsupported">Link unsupported</option>
          </select>
        </label>
        <label>
          Madmom command
          <input id="config-madmom-command" name="configMadmomCommand" type="text" />
        </label>
        <label>
          Madmom tracker
          <input id="config-madmom-tracker" name="configMadmomTracker" type="text" />
        </label>
        <label>
          Madmom audio input index
          <input id="config-madmom-audio-index" name="configMadmomAudioIndex" type="number" min="0" />
        </label>
        <button id="apply-structured-config" type="button">
          Apply Structured Config
        </button>
        <p>
          MIDI bindings: <span id="config-midi-bindings">none</span>
        </p>
      </fieldset>
      <fieldset>
        <legend>Output And Layout Config</legend>
        <label>
          Dome hardware enabled
          <input id="config-dome-enabled" name="configDomeEnabled" type="checkbox" />
        </label>
        <label>
          Dome simulator enabled
          <input id="config-dome-simulation-enabled" name="configDomeSimulationEnabled" type="checkbox" />
        </label>
        <label>
          Dome OPC address
          <input id="config-dome-opc-address" name="configDomeOpcAddress" type="text" />
        </label>
        <label>
          Dome brightness
          <input id="config-dome-brightness" name="configDomeBrightness" type="number" min="0" max="1" step="0.01" />
        </label>
        <label>
          Bar hardware enabled
          <input id="config-bar-enabled" name="configBarEnabled" type="checkbox" />
        </label>
        <label>
          Bar simulator enabled
          <input id="config-bar-simulation-enabled" name="configBarSimulationEnabled" type="checkbox" />
        </label>
        <label>
          Bar infinity length
          <input id="config-bar-infinity-length" name="configBarInfinityLength" type="number" min="0" step="1" />
        </label>
        <label>
          Bar infinity width
          <input id="config-bar-infinity-width" name="configBarInfinityWidth" type="number" min="0" step="1" />
        </label>
        <label>
          Bar runner length
          <input id="config-bar-runner-length" name="configBarRunnerLength" type="number" min="0" step="1" />
        </label>
        <label>
          Bar brightness
          <input id="config-bar-brightness" name="configBarBrightness" type="number" min="0" max="1" step="0.01" />
        </label>
        <label>
          Stage hardware enabled
          <input id="config-stage-enabled" name="configStageEnabled" type="checkbox" />
        </label>
        <label>
          Stage simulator enabled
          <input id="config-stage-simulation-enabled" name="configStageSimulationEnabled" type="checkbox" />
        </label>
        <label>
          Stage OPC address
          <input id="config-stage-opc-address" name="configStageOpcAddress" type="text" />
        </label>
        <label>
          Stage brightness
          <input id="config-stage-brightness" name="configStageBrightness" type="number" min="0" max="1" step="0.01" />
        </label>
        <label>
          Stage side lengths
          <textarea id="config-stage-side-lengths" className="config-editor" name="configStageSideLengths" rows={3} spellCheck={false} />
        </label>
      </fieldset>
      <div>
        <button id="reload-config" type="button">
          Reload Config
        </button>
        <button id="apply-config" type="button">
          Apply Config
        </button>
        <span id="config-status">not loaded</span>
      </div>
      <textarea id="config-editor" className="config-editor" spellCheck={false} rows={16} />
    </details>
  );
}

function RuntimeControls() {
  return (
    <section aria-label="Runtime Controls">
      <h2>Runtime Controls</h2>
      <p>These controls update the active engine configuration used by hardware output and the simulator.</p>
      <label>
        Dome visualizer
        <VisualizerSelect id="dome-active-vis" name="domeActiveVis" />
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
    <details id="palette-drawer" aria-label="Palettes">
      <summary>Palettes</summary>
      <p>Select the active palette, or edit any palette slot directly. The drawer shows all eight Spectrum palette banks at once.</p>
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
      <section id="palette-grid" aria-label="Palette colors" className="palette-grid" data-palette-editor="64-entry-gradient" />
    </details>
  );
}

function InputsDrawer() {
  return (
    <details id="inputs-drawer" aria-label="Inputs">
      <summary>Inputs</summary>
      <button id="tap-tempo" type="button">
        Tap Tempo
      </button>
      <button id="reset-tempo" type="button">
        Reset Tempo
      </button>
      <button id="orientation-calibrate" type="button">
        Calibrate Orientation
      </button>
      <p>
        BPM: <span id="tempo-bpm">[none]</span> Tap counter: <span id="tap-counter">Tap</span>
      </p>
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
          <strong>Orientation</strong>
          <span id="input-orientation">disabled</span>
        </p>
        <p className="target-status">
          <strong>Madmom</strong>
          <span id="input-madmom">disabled</span>
        </p>
      </div>
      <section aria-label="Orientation Devices">
        <h3>Orientation Devices</h3>
        <ul id="orientation-devices" className="device-list">
          <li>none</li>
        </ul>
      </section>
      <section aria-label="MIDI Log">
        <h3>MIDI Log</h3>
        <ol id="midi-log" className="device-list">
          <li>none</li>
        </ol>
      </section>
    </details>
  );
}

function DebugVisualsDrawer() {
  return (
    <details id="debug-visuals-drawer" aria-label="Debug Visuals">
      <summary>Debug Visuals</summary>
      <p>Debug visuals are hardware-check patterns. Dome debug visuals override the selected dome visualizer while active.</p>
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
    </details>
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
        <h3>Bar Simulator</h3>
        <ol id="bar-simulator" className="device-list">
          <li>none</li>
        </ol>
        <h3>Stage Simulator</h3>
        <ol id="stage-simulator" className="device-list">
          <li>none</li>
        </ol>
      </section>
    </>
  );
}

function PreviewDrawer() {
  return (
    <details id="preview-drawer">
      <summary>Preview</summary>
      <p>
        This preview mirrors the live runtime frame stream used for hardware output. Open the independent sandbox at{' '}
        <a href="/simulator">/simulator</a>.
      </p>
      <SimulatorFrameView streamText="stream disconnected" />
    </details>
  );
}

function OpcTargetsFooter() {
  return (
    <footer className="opc-targets-footer" aria-label="OPC Targets">
      <h2>OPC Targets</h2>
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
    </footer>
  );
}

function ControlApp() {
  return (
    <main data-domers-app>
      <h1>MindShark Dome Control Panel</h1>
      <section aria-label="Engine">
        <button id="start-engine" type="button">Start</button>
        <button id="stop-engine" type="button">Stop</button>
        <span id="engine-status">stopped</span>
      </section>
      <ConfigEditor />
      <RuntimeControls />
      <PaletteDrawer />
      <InputsDrawer />
      <DebugVisualsDrawer />
      <PreviewDrawer />
      <OpcTargetsFooter />
    </main>
  );
}

function SimulatorOnlyControls() {
  return (
    <section aria-label="Simulator-Only Controls">
      <h2>Simulator-Only Controls</h2>
      <p>These controls affect this simulator page only. They do not patch live runtime config or hardware output.</p>
      <label>
        Dome visualizer
        <VisualizerSelect id="sandbox-dome-active-vis" name="sandboxDomeActiveVis" />
      </label>
      <p>Stage Depth is a stage-output visualizer; it will appear with the stage simulator controls, not the dome canvas selector.</p>
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
      <SimulatorOnlyControls />
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
