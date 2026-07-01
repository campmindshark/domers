#!/usr/bin/env python3
"""Capture visualizer frame hashes by executing legacy Spectrum C# code.

The runner references Spectrum directly, loads the default XML config, forces
simulation-only output, and invokes visualizers headlessly. It does not start
the WPF UI, audio capture, MIDI devices, orientation UDP listeners, or OPC
hardware output.
"""

from __future__ import annotations

import json
import os
import platform
import subprocess
import sys
from datetime import date
from pathlib import Path

import build_spectrum_csharp


ROOT = Path(__file__).resolve().parents[1]
SPECTRUM = ROOT.parent / "spectrum"
CASES = Path(
    os.environ.get(
        "DOMERS_VISUALIZER_CASES",
        str(ROOT / "fixtures" / "spectrum-csharp" / "visualizer_frame_cases.json"),
    )
)
if not CASES.is_absolute():
    CASES = ROOT / CASES
RUNNER_DIR = ROOT / "target" / "spectrum-visualizer-capture"
RUNNER_CSPROJ = RUNNER_DIR / "SpectrumVisualizerCapture.csproj"
RUNNER_PROGRAM = RUNNER_DIR / "Program.cs"

# FNV-1a offset basis: the hash of a frame with no dome/bar/stage output. A
# sequence that returns only this value emitted nothing on every frame.
EMPTY_FRAME_HASH = str(0xCBF29CE484222325)


RUNNER_PROJECT = """<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net10.0-windows</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>

  <ItemGroup>
    <ProjectReference Include="../../../spectrum/Spectrum/Spectrum.csproj" />
  </ItemGroup>
</Project>
"""


RUNNER_SOURCE = r"""using System.Collections.Concurrent;
using System.Reflection;
using System.Text.Json;
using Spectrum;
using Spectrum.Audio;
using Spectrum.Base;
using Spectrum.LEDs;
using Spectrum.MIDI;
using XSerializer;

record CaptureCase(string Case, string Name, string Classification, CaptureInput Input, List<CaptureInput> InputSequence, bool HasSequence, long FrameDeltaTicks);
record MidiNote(int Index, double Value);
record CaptureInput(double Volume, double BeatProgress, bool FlashActive, int DiagnosticState, int DiagnosticStep, int PaletteSlot, List<MidiNote> MidiNotes);
record CaptureResult(string Case, string Name, string Status, string Value, List<string> Frames, string? Error);

class Program {
static readonly Dictionary<string, string> typeNames = new() {
  ["LEDDomeStrutIterationDiagnosticVisualizer"] = "Spectrum.LEDDomeStrutIterationDiagnosticVisualizer",
  ["LEDDomeFlashColorsDiagnosticVisualizer"] = "Spectrum.LEDDomeFlashColorsDiagnosticVisualizer",
  ["LEDDomeStrandTestDiagnosticVisualizer"] = "Spectrum.LEDDomeStrandTestDiagnosticVisualizer",
  ["LEDDomeFullColorFlashDiagnosticVisualizer"] = "Spectrum.LEDDomeFullColorFlashDiagnosticVisualizer",
  ["LEDDomeVolumeVisualizer"] = "Spectrum.LEDDomeVolumeVisualizer",
  ["LEDDomeRadialVisualizer"] = "Spectrum.LEDDomeRadialVisualizer",
  ["LEDDomeRaceVisualizer"] = "Spectrum.LEDDomeRaceVisualizer",
  ["LEDDomeSnakesVisualizer"] = "Spectrum.LEDDomeSnakesVisualizer",
  ["LEDDomeSplatVisualizer"] = "Spectrum.LEDDomeSplatVisualizer",
  ["LEDDomeQuaternionTestVisualizer"] = "Spectrum.Visualizers.LEDDomeQuaternionTestVisualizer",
  ["LEDDomeQuaternionMultiTestVisualizer"] = "Spectrum.Visualizers.LEDDomeQuaternionMultiTestVisualizer",
  ["LEDDomeQuaternionPaintbrushVisualizer"] = "Spectrum.Visualizers.LEDDomeQuaternionPaintbrushVisualizer",
  ["LEDDomeTVStaticVisualizer"] = "Spectrum.LEDDomeTVStaticVisualizer",
  ["LEDDomeFlashVisualizer"] = "Spectrum.LEDDomeFlashVisualizer",
  ["LEDBarFlashColorsDiagnosticVisualizer"] = "Spectrum.LEDBarFlashColorsDiagnosticVisualizer",
  ["LEDStageFlashColorsDiagnosticVisualizer"] = "Spectrum.LEDStageFlashColorsDiagnosticVisualizer",
  ["LEDStageDepthLevelVisualizer"] = "Spectrum.LEDStageDepthLevelVisualizer",
};

static SpectrumConfiguration LoadConfig(string path) {
  using var stream = File.OpenRead(path);
  var config = new XmlSerializer<SpectrumConfiguration>().Deserialize(stream);
  config.domeEnabled = false;
  config.barEnabled = false;
  config.stageEnabled = false;
  config.domeSimulationEnabled = true;
  config.barSimulationEnabled = true;
  config.stageSimulationEnabled = true;
  config.domeOutputInSeparateThread = false;
  config.barOutputInSeparateThread = false;
  config.stageOutputInSeparateThread = false;
  config.midiInputEnabled = false;
  config.domeBrightness = 1.0;
  config.domeMaxBrightness = 1.0;
  config.barBrightness = 1.0;
  config.stageBrightness = 1.0;
  config.flashSpeed = 0.0;
  config.colorPaletteIndex = 0;
  config.beatBroadcaster.Reset();
  return config;
}

static void SetAudioVolume(AudioInput audio, double volume) {
  var field = typeof(AudioInput).GetField("<Volume>k__BackingField", BindingFlags.Instance | BindingFlags.NonPublic);
  if (field == null) {
    throw new Exception("Could not find AudioInput.Volume backing field");
  }
  field.SetValue(audio, (float)volume);
}

static object CreateVisualizer(string name, SpectrumConfiguration config, AudioInput audio, MidiInput midi, OrientationInput orientation, LEDDomeOutput dome, LEDBarOutput bar, LEDStageOutput stage) {
  var assembly = typeof(SpectrumConfiguration).Assembly;
  var type = assembly.GetType(typeNames[name]) ?? throw new Exception($"Missing type for {name}");
  object?[] args = name switch {
    "LEDDomeStrutIterationDiagnosticVisualizer" or
    "LEDDomeFlashColorsDiagnosticVisualizer" or
    "LEDDomeStrandTestDiagnosticVisualizer" or
    "LEDDomeFullColorFlashDiagnosticVisualizer" or
    "LEDDomeTVStaticVisualizer" => new object?[] { config, dome },
    "LEDDomeVolumeVisualizer" or
    "LEDDomeRadialVisualizer" or
    "LEDDomeSplatVisualizer" or
    "LEDDomeSnakesVisualizer" => new object?[] { config, audio, dome },
    "LEDDomeRaceVisualizer" or
    "LEDDomeFlashVisualizer" => new object?[] { config, audio, midi, dome },
    "LEDDomeQuaternionTestVisualizer" or
    "LEDDomeQuaternionMultiTestVisualizer" => new object?[] { config, orientation, dome },
    "LEDDomeQuaternionPaintbrushVisualizer" => new object?[] { config, audio, orientation, dome },
    "LEDBarFlashColorsDiagnosticVisualizer" => new object?[] { config, bar },
    "LEDStageFlashColorsDiagnosticVisualizer" => new object?[] { config, stage },
    "LEDStageDepthLevelVisualizer" => new object?[] { config, audio, stage },
    _ => throw new Exception($"Unhandled visualizer {name}"),
  };
  return Activator.CreateInstance(type, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic, null, args, null)
    ?? throw new Exception($"Could not instantiate {name}");
}

static void SeedRandomFields(object visualizer) {
  foreach (var field in visualizer.GetType().GetFields(BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)) {
    if (field.FieldType == typeof(Random)) {
      field.SetValue(visualizer, new Random(0));
    }
  }
}

static void ConfigureCase(SpectrumConfiguration config, CaptureCase testCase) {
  config.colorPaletteIndex = testCase.Input.PaletteSlot;
  config.domeTestPattern = 0;
  config.barTestPattern = 0;
  config.stageTestPattern = 0;
  config.domeActiveVis = testCase.Name switch {
    "LEDDomeVolumeVisualizer" => 0,
    "LEDDomeRadialVisualizer" => 1,
    "LEDDomeRaceVisualizer" => 2,
    "LEDDomeSnakesVisualizer" => 3,
    "LEDDomeQuaternionTestVisualizer" => 4,
    "LEDDomeQuaternionMultiTestVisualizer" => 5,
    "LEDDomeQuaternionPaintbrushVisualizer" => 6,
    "LEDDomeSplatVisualizer" => 7,
    _ => config.domeActiveVis,
  };
}

static void DrainQueues(SpectrumConfiguration config) {
  while (config.domeCommandQueue.TryDequeue(out _)) {}
  while (config.barCommandQueue.TryDequeue(out _)) {}
  while (config.stageCommandQueue.TryDequeue(out _)) {}
}

static bool HasQueuedOutput(SpectrumConfiguration config) {
  return !config.domeCommandQueue.IsEmpty ||
    !config.barCommandQueue.IsEmpty ||
    !config.stageCommandQueue.IsEmpty;
}

static void HashByte(ref ulong hash, byte value) {
  hash ^= value;
  hash *= 0x00000100000001b3UL;
}

static void HashUsize(ref ulong hash, int value) {
  foreach (var b in BitConverter.GetBytes((ulong)value)) {
    HashByte(ref hash, b);
  }
}

static void HashColor(ref ulong hash, int color) {
  HashByte(ref hash, (byte)((color >> 16) & 0xff));
  HashByte(ref hash, (byte)((color >> 8) & 0xff));
  HashByte(ref hash, (byte)(color & 0xff));
}

static void HashDomeCommands(ref ulong hash, SpectrumConfiguration config) {
  while (config.domeCommandQueue.TryDequeue(out var command)) {
    if (command.isFlush) {
      HashByte(ref hash, 0);
    } else if (command.frame != null) {
      HashByte(ref hash, 1);
      foreach (var color in command.frame) {
        HashColor(ref hash, color);
      }
    } else {
      HashByte(ref hash, 2);
      HashUsize(ref hash, command.strutIndex);
      HashUsize(ref hash, command.ledIndex);
      HashColor(ref hash, command.color);
    }
  }
}

static bool HashDomeBufferIfPresent(ref ulong hash, object visualizer) {
  var field = visualizer.GetType().GetField("buffer", BindingFlags.Instance | BindingFlags.NonPublic);
  var buffer = field?.GetValue(visualizer);
  if (buffer == null) {
    return false;
  }
  var pixels = (Array)(buffer.GetType().GetField("pixels")?.GetValue(buffer)
    ?? throw new Exception("buffer.pixels missing"));
  HashByte(ref hash, 1);
  foreach (var pixel in pixels) {
    var color = (int)(pixel.GetType().GetProperty("color")?.GetValue(pixel)
      ?? throw new Exception("pixel.color missing"));
    HashColor(ref hash, color);
  }
  HashByte(ref hash, 0);
  return true;
}

static void HashBarCommands(ref ulong hash, SpectrumConfiguration config) {
  while (config.barCommandQueue.TryDequeue(out var command)) {
    HashByte(ref hash, command.isFlush ? (byte)0 : (byte)2);
    if (!command.isFlush) {
      HashByte(ref hash, command.isRunner ? (byte)1 : (byte)0);
      HashUsize(ref hash, command.ledIndex);
      HashColor(ref hash, command.color);
    }
  }
}

static void HashStageCommands(ref ulong hash, SpectrumConfiguration config, bool traceStage) {
  int traceCount = 0;
  while (config.stageCommandQueue.TryDequeue(out var command)) {
    HashByte(ref hash, command.isFlush ? (byte)0 : (byte)2);
    if (!command.isFlush) {
      if (traceStage && traceCount < 24) {
        Console.Error.WriteLine($"stage[{traceCount}] side={command.sideIndex} led={command.ledIndex} layer={command.layerIndex} color={command.color}");
      }
      traceCount++;
      HashUsize(ref hash, command.sideIndex);
      HashUsize(ref hash, command.ledIndex);
      HashUsize(ref hash, command.layerIndex);
      HashColor(ref hash, command.color);
    }
  }
}

static ulong HashVisualizerOutput(SpectrumConfiguration config, object visualizer, CaptureCase testCase) {
  ulong hash = 0xcbf29ce484222325UL;
  var usedBuffer = HashDomeBufferIfPresent(ref hash, visualizer);
  if (!usedBuffer) {
    HashDomeCommands(ref hash, config);
  } else {
    DrainQueues(config);
  }
  HashBarCommands(ref hash, config);
  HashStageCommands(ref hash, config, Environment.GetEnvironmentVariable("DOMERS_TRACE_STAGE") == testCase.Case);
  return hash;
}

// Fixed anchor for the deterministic capture clock. Any constant works; a real
// calendar instant just keeps intermediate values realistic.
static readonly long clockBaseTicks = new DateTime(2020, 1, 1, 0, 0, 0, DateTimeKind.Utc).Ticks;
// Per-frame advance of the deterministic clock. 100000 ticks = 10ms. Override
// with DOMERS_FRAME_DELTA_TICKS. Recorded in the manifest so Rust reproduces it.
static long FrameDeltaTicks() {
  var raw = Environment.GetEnvironmentVariable("DOMERS_FRAME_DELTA_TICKS");
  if (!string.IsNullOrEmpty(raw) && long.TryParse(raw, out var parsed) && parsed > 0) {
    return parsed;
  }
  return 100000;
}
// Fixed synthetic measure length (ms) used when injecting per-frame beat
// progress. ProgressThroughMeasure resolves to exactly the input beat_progress.
const long beatMeasureMs = 1000;
static readonly FieldInfo beatStartField =
  typeof(BeatBroadcaster).GetField("startingTime", BindingFlags.Instance | BindingFlags.NonPublic)
    ?? throw new Exception("BeatBroadcaster.startingTime field missing");
static readonly FieldInfo beatMeasureField =
  typeof(BeatBroadcaster).GetField("measureLength", BindingFlags.Instance | BindingFlags.NonPublic)
    ?? throw new Exception("BeatBroadcaster.measureLength field missing");

// Drive BeatBroadcaster progress deterministically for the current clock value:
// anchor startingTime so that ProgressThroughMeasure == beatProgress at nowTicks.
static void InjectBeat(SpectrumConfiguration config, long nowTicks, double beatProgress) {
  var bb = config.beatBroadcaster;
  long nowMs = nowTicks / TimeSpan.TicksPerMillisecond;
  long startMs = nowMs - (long)Math.Round(beatProgress * beatMeasureMs);
  beatMeasureField.SetValue(bb, (int)beatMeasureMs);
  beatStartField.SetValue(bb, startMs);
}

// Legacy single-frame path: real wall clock plus the throttle-crossing sleep
// used by the captured first-frame manifest. Behavior is preserved so those
// goldens stay reproducible.
static List<string> CaptureSingleFrame(SpectrumConfiguration config, CaptureCase testCase) {
  ConfigureCase(config, testCase);
  var audio = new AudioInput(config);
  SetAudioVolume(audio, testCase.Input.Volume);
  var midi = new MidiInput(config);
  var orientation = new OrientationInput(config);
  var dome = new LEDDomeOutput(config);
  var bar = new LEDBarOutput(config);
  var stage = new LEDStageOutput(config);
  var visualizer = CreateVisualizer(testCase.Name, config, audio, midi, orientation, dome, bar, stage);
  SeedRandomFields(visualizer);
  ((Visualizer)visualizer).Enabled = true;
  DrainQueues(config);
  var values = new List<string>();
  foreach (var input in testCase.InputSequence) {
    SetAudioVolume(audio, input.Volume);
    ((Visualizer)visualizer).Visualize();
    if (!HasQueuedOutput(config) && visualizer.GetType().GetField("buffer", BindingFlags.Instance | BindingFlags.NonPublic)?.GetValue(visualizer) == null) {
      Thread.Sleep(1100);
      ((Visualizer)visualizer).Visualize();
    }
    values.Add(HashVisualizerOutput(config, visualizer, testCase).ToString());
  }
  return values;
}

// Deterministic multi-frame path: keep a single visualizer instance alive across
// the whole input_sequence, advancing the injected clock by a fixed per-frame
// delta and injecting per-frame beat_progress/volume/palette_slot. One Visualize
// call per entry, hashed with the same scheme as the single-frame capture.
static List<string> CaptureSequence(SpectrumConfiguration config, CaptureCase testCase) {
  ConfigureCase(config, testCase);
  long delta = FrameDeltaTicks();
  // Pin the clock before constructing anything so construction-time timestamps
  // (Snakes lastUpdate, Stopwatch starts, Flash animation anchors) are stable.
  DeterministicClock.OverrideTicks = clockBaseTicks;
  try {
    config.beatBroadcaster.Reset();
    var audio = new AudioInput(config);
    SetAudioVolume(audio, testCase.Input.Volume);
    var midi = new MidiInput(config);
    var orientation = new OrientationInput(config);
    var dome = new LEDDomeOutput(config);
    var bar = new LEDBarOutput(config);
    var stage = new LEDStageOutput(config);
    var visualizer = CreateVisualizer(testCase.Name, config, audio, midi, orientation, dome, bar, stage);
    SeedRandomFields(visualizer);
    ((Visualizer)visualizer).Enabled = true;
    DrainQueues(config);
    var values = new List<string>();
    int frameIndex = 0;
    foreach (var input in testCase.InputSequence) {
      long nowTicks = clockBaseTicks + (long)frameIndex * delta;
      DeterministicClock.OverrideTicks = nowTicks;
      InjectBeat(config, nowTicks, input.BeatProgress);
      SetAudioVolume(audio, input.Volume);
      config.colorPaletteIndex = input.PaletteSlot;
      ((Visualizer)visualizer).Visualize();
      values.Add(HashVisualizerOutput(config, visualizer, testCase).ToString());
      frameIndex++;
    }
    return values;
  } finally {
    DeterministicClock.OverrideTicks = null;
  }
}

static List<string> CaptureFrameHashes(SpectrumConfiguration config, CaptureCase testCase) {
  return testCase.HasSequence
    ? CaptureSequence(config, testCase)
    : CaptureSingleFrame(config, testCase);
}

static string CaptureHash(SpectrumConfiguration config, CaptureCase testCase) {
  return CaptureFrameHashes(config, testCase)[0];
}

static int Main(string[] args) {
  var manifestPath = args.Length > 0 ? args[0] : "visualizer_frame_cases.json";
  var configPath = args.Length > 1 ? args[1] : "spectrum_default_config.xml";
  var manifest = System.Text.Json.JsonSerializer.Deserialize<JsonElement>(File.ReadAllText(manifestPath));
  var cases = new List<CaptureCase>();
  foreach (var item in manifest.GetProperty("cases").EnumerateArray()) {
    var input = item.GetProperty("input");
    var sequence = new List<CaptureInput>();
    bool hasSequence = item.TryGetProperty("input_sequence", out var inputSequence);
    if (hasSequence) {
      foreach (var frameInput in inputSequence.EnumerateArray()) {
        sequence.Add(new CaptureInput(
          frameInput.GetProperty("volume").GetDouble(),
          frameInput.GetProperty("beat_progress").GetDouble(),
          frameInput.GetProperty("flash_active").GetBoolean(),
          frameInput.GetProperty("diagnostic_state").GetInt32(),
          frameInput.GetProperty("diagnostic_step").GetInt32(),
          frameInput.GetProperty("palette_slot").GetInt32(),
          ParseMidiNotes(frameInput)
        ));
      }
    }
    if (sequence.Count == 0) {
      sequence.Add(new CaptureInput(
        input.GetProperty("volume").GetDouble(),
        input.GetProperty("beat_progress").GetDouble(),
        input.GetProperty("flash_active").GetBoolean(),
        input.GetProperty("diagnostic_state").GetInt32(),
        input.GetProperty("diagnostic_step").GetInt32(),
        input.GetProperty("palette_slot").GetInt32(),
        ParseMidiNotes(input)
      ));
    }
    long caseDelta = item.TryGetProperty("frame_delta_ticks", out var fdt) && fdt.TryGetInt64(out var fdtVal) && fdtVal > 0
      ? fdtVal
      : FrameDeltaTicks();
    cases.Add(new CaptureCase(
      item.GetProperty("case").GetString()!,
      item.GetProperty("name").GetString()!,
      item.GetProperty("classification").GetString()!,
      new CaptureInput(
        input.GetProperty("volume").GetDouble(),
        input.GetProperty("beat_progress").GetDouble(),
        input.GetProperty("flash_active").GetBoolean(),
        input.GetProperty("diagnostic_state").GetInt32(),
        input.GetProperty("diagnostic_step").GetInt32(),
        input.GetProperty("palette_slot").GetInt32(),
        ParseMidiNotes(input)
      ),
      sequence,
      hasSequence,
      caseDelta
    ));
  }

  var config = LoadConfig(configPath);
  var results = new List<CaptureResult>();
  foreach (var testCase in cases) {
    try {
      var frames = CaptureFrameHashes(config, testCase);
      // Multi-frame sequences must be reproducible. Recapture and compare so a
      // visualizer with inherent non-determinism (e.g. an unseeded local
      // Random the harness cannot seed) is flagged instead of writing a golden
      // that will never reproduce.
      if (testCase.HasSequence) {
        var frames2 = CaptureFrameHashes(config, testCase);
        if (!frames.SequenceEqual(frames2)) {
          results.Add(new CaptureResult(testCase.Case, testCase.Name, "nondeterministic", "", new List<string>(),
            "Sequence hashes differed across repeated captures (visualizer has non-deterministic state the harness cannot control, e.g. an unseeded local Random)."));
          continue;
        }
      }
      results.Add(new CaptureResult(testCase.Case, testCase.Name, "captured", frames.Count > 0 ? frames[0] : "", frames, null));
    } catch (Exception ex) {
      results.Add(new CaptureResult(testCase.Case, testCase.Name, "failed", "", new List<string>(), ex.GetBaseException().Message));
    }
  }
  Console.WriteLine(System.Text.Json.JsonSerializer.Serialize(new { results }, new JsonSerializerOptions { WriteIndented = true }));
  return 0;
}
}
"""


def is_wsl() -> bool:
    return "microsoft" in platform.release().lower()


def write_runner() -> None:
    RUNNER_DIR.mkdir(parents=True, exist_ok=True)
    RUNNER_CSPROJ.write_text(RUNNER_PROJECT, encoding="utf-8")
    RUNNER_PROGRAM.write_text(RUNNER_SOURCE, encoding="utf-8")


def run_capture() -> dict[str, object]:
    env = os.environ.copy()
    env["DOTNET_CLI_TELEMETRY_OPTOUT"] = "1"
    if is_wsl():
        runner_win = build_spectrum_csharp.wslpath(RUNNER_CSPROJ)
        cases_win = build_spectrum_csharp.wslpath(CASES)
        config_win = build_spectrum_csharp.wslpath(
            SPECTRUM / "Spectrum" / "spectrum_default_config.xml"
        )
        command = (
            "$env:DOTNET_CLI_TELEMETRY_OPTOUT = '1'; "
            f"$env:DOMERS_TRACE_STAGE = {build_spectrum_csharp.quote_ps(env.get('DOMERS_TRACE_STAGE', ''))}; "
            "$dotnet = Join-Path $env:USERPROFILE '.dotnet\\dotnet.exe'; "
            f"& $dotnet run --project {build_spectrum_csharp.quote_ps(runner_win)} "
            "--configuration Release --verbosity quiet -- "
            f"{build_spectrum_csharp.quote_ps(cases_win)} "
            f"{build_spectrum_csharp.quote_ps(config_win)}"
        )
        args = [
            "powershell.exe",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            command,
        ]
    else:
        args = [
            build_spectrum_csharp.local_dotnet(None),
            "run",
            "--project",
            str(RUNNER_CSPROJ),
            "--configuration",
            "Release",
            "--verbosity",
            "quiet",
            "--",
            str(CASES),
            str(SPECTRUM / "Spectrum" / "spectrum_default_config.xml"),
        ]
    result = subprocess.run(
        args,
        env=env,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        sys.stderr.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)
    if env.get("DOMERS_TRACE_STAGE"):
        sys.stderr.write(result.stderr)
    return json.loads(result.stdout)


def frame_delta_ticks() -> int:
    raw = os.environ.get("DOMERS_FRAME_DELTA_TICKS", "")
    if raw:
        try:
            parsed = int(raw)
            if parsed > 0:
                return parsed
        except ValueError:
            pass
    return 100000


def update_manifest(capture: dict[str, object]) -> int:
    manifest = json.loads(CASES.read_text(encoding="utf-8"))
    by_case = {result["Case"]: result for result in capture["results"]}
    is_sequence_manifest = any(
        "input_sequence" in case for case in manifest["cases"]
    )
    failures = 0
    for case in manifest["cases"]:
        result = by_case[case["case"]]
        frames = result["Frames"] if "input_sequence" in case else []
        # An all-EMPTY_FRAME_HASH sequence means the visualizer emitted nothing
        # across every frame: its internal time throttle was never crossed at
        # this cadence, or it needs input (e.g. MIDI notes) the harness cannot
        # supply yet. Leave those pending with a reason rather than recording a
        # misleading "captured" no-output golden. This keeps the decision honest
        # and is re-run safe.
        empty_sequence = (
            result["Status"] == "captured"
            and "input_sequence" in case
            and bool(frames)
            and all(frame == EMPTY_FRAME_HASH for frame in frames)
        )
        if result["Status"] == "captured" and not empty_sequence:
            case["expected"]["status"] = "captured"
            if "input_sequence" in case:
                case["expected"]["frames"] = result["Frames"]
                if result["Frames"]:
                    case["expected"]["value"] = result["Frames"][0]
            else:
                case["expected"]["value"] = result["Value"]
            case["known_unverified"] = [
                note
                for note in case.get("known_unverified", [])
                if "not captured" not in note and "pending" not in note.lower()
            ]
        elif empty_sequence:
            case["expected"]["status"] = "pending_csharp_execution"
            case["expected"]["frames"] = []
            case["expected"]["value"] = None
            case["known_unverified"] = [
                "Sequence emitted no Spectrum output on every frame at "
                f"frame_delta_ticks={frame_delta_ticks()}: the visualizer's "
                "internal time throttle was not crossed within the sequence, or "
                "it requires input the headless harness does not yet supply "
                "(e.g. MIDI note state). Left pending until a cadence or input "
                "that produces motion is defined."
            ]
        elif result["Status"] == "nondeterministic":
            case["expected"]["status"] = "pending_csharp_execution"
            if "input_sequence" in case:
                case["expected"]["frames"] = []
            case["expected"]["value"] = None
            case["known_unverified"] = [result["Error"]]
        else:
            failures += 1
            case["expected"]["status"] = "capture_failed"
            if "input_sequence" in case:
                case["expected"]["frames"] = []
            case["expected"]["value"] = None
            case["known_unverified"] = [f"C# capture failed: {result['Error']}"]
    manifest["capture_tool"] = "tools/capture_spectrum_visualizer_frames.py"
    if is_sequence_manifest:
        manifest["capture_metadata"] = {
            "source": str((SPECTRUM / "Spectrum" / "spectrum_default_config.xml").relative_to(ROOT.parent)),
            "runner": str(RUNNER_CSPROJ.relative_to(ROOT)),
            "command": (
                "DOMERS_VISUALIZER_CASES=fixtures/spectrum-csharp/"
                "visualizer_sequence_cases.json "
                "python3 tools/capture_spectrum_visualizer_frames.py"
            ),
            "hardware_required": False,
            "frame_delta_ticks": frame_delta_ticks(),
            "clock_base_ticks": (date(2020, 1, 1).toordinal() - 1) * 864000000000,
            "beat_measure_ms": 1000,
            "description": (
                "Stateful multi-frame Spectrum visualizer sequences. Each case keeps one "
                "Spectrum visualizer instance alive across the whole input_sequence, driven "
                "by an injected deterministic clock (Spectrum.Base.DeterministicClock) that "
                "starts at clock_base_ticks and advances frame_delta_ticks per frame, with "
                "per-frame beat_progress injected via a synthetic beat_measure_ms measure. "
                "Frame hashes use the same FNV-1a scheme as the single-frame capture."
            ),
        }
    else:
        manifest["capture_metadata"] = {
            "source": str((SPECTRUM / "Spectrum" / "spectrum_default_config.xml").relative_to(ROOT.parent)),
            "runner": str(RUNNER_CSPROJ.relative_to(ROOT)),
            "command": "python3 tools/capture_spectrum_visualizer_frames.py",
            "hardware_required": False,
            "description": "Headless visualizer execution through Spectrum C# classes with simulation-only output.",
        }
    CASES.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return failures


def main() -> int:
    build_spectrum_csharp.ensure_madmom_submodule()
    write_runner()
    capture = run_capture()
    failures = update_manifest(capture)
    print(f"wrote {CASES.relative_to(ROOT)}")
    if failures:
        print(f"{failures} visualizer captures failed", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
