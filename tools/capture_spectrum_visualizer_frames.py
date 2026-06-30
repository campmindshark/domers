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
from pathlib import Path

import build_spectrum_csharp


ROOT = Path(__file__).resolve().parents[1]
SPECTRUM = ROOT.parent / "spectrum"
CASES = ROOT / "fixtures" / "spectrum-csharp" / "visualizer_frame_cases.json"
RUNNER_DIR = ROOT / "target" / "spectrum-visualizer-capture"
RUNNER_CSPROJ = RUNNER_DIR / "SpectrumVisualizerCapture.csproj"
RUNNER_PROGRAM = RUNNER_DIR / "Program.cs"


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

record CaptureCase(string Case, string Name, string Classification, CaptureInput Input);
record CaptureInput(double Volume, double BeatProgress, bool FlashActive, int DiagnosticState, int DiagnosticStep, int PaletteSlot);
record CaptureResult(string Case, string Name, string Status, string Value, string? Error);

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

static void HashStageCommands(ref ulong hash, SpectrumConfiguration config) {
  while (config.stageCommandQueue.TryDequeue(out var command)) {
    HashByte(ref hash, command.isFlush ? (byte)0 : (byte)2);
    if (!command.isFlush) {
      HashUsize(ref hash, command.sideIndex);
      HashUsize(ref hash, command.ledIndex);
      HashUsize(ref hash, command.layerIndex);
      HashColor(ref hash, command.color);
    }
  }
}

static string CaptureHash(SpectrumConfiguration config, CaptureCase testCase) {
  ConfigureCase(config, testCase);
  var audio = new AudioInput(config);
  SetAudioVolume(audio, testCase.Input.Volume);
  var midi = new MidiInput(config);
  var orientation = new OrientationInput(config);
  var dome = new LEDDomeOutput(config);
  var bar = new LEDBarOutput(config);
  var stage = new LEDStageOutput(config);
  var visualizer = CreateVisualizer(testCase.Name, config, audio, midi, orientation, dome, bar, stage);
  ((Visualizer)visualizer).Enabled = true;
  DrainQueues(config);
  ((Visualizer)visualizer).Visualize();
  if (!HasQueuedOutput(config) && visualizer.GetType().GetField("buffer", BindingFlags.Instance | BindingFlags.NonPublic)?.GetValue(visualizer) == null) {
    Thread.Sleep(1100);
    ((Visualizer)visualizer).Visualize();
  }
  ulong hash = 0xcbf29ce484222325UL;
  var usedBuffer = HashDomeBufferIfPresent(ref hash, visualizer);
  if (!usedBuffer) {
    HashDomeCommands(ref hash, config);
  } else {
    DrainQueues(config);
  }
  HashBarCommands(ref hash, config);
  HashStageCommands(ref hash, config);
  return hash.ToString();
}

static int Main(string[] args) {
  var manifestPath = args.Length > 0 ? args[0] : "visualizer_frame_cases.json";
  var configPath = args.Length > 1 ? args[1] : "spectrum_default_config.xml";
  var manifest = System.Text.Json.JsonSerializer.Deserialize<JsonElement>(File.ReadAllText(manifestPath));
  var cases = new List<CaptureCase>();
  foreach (var item in manifest.GetProperty("cases").EnumerateArray()) {
    var input = item.GetProperty("input");
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
        input.GetProperty("palette_slot").GetInt32()
      )
    ));
  }

  var config = LoadConfig(configPath);
  var results = new List<CaptureResult>();
  foreach (var testCase in cases) {
    try {
      results.Add(new CaptureResult(testCase.Case, testCase.Name, "captured", CaptureHash(config, testCase), null));
    } catch (Exception ex) {
      results.Add(new CaptureResult(testCase.Case, testCase.Name, "failed", "", ex.GetBaseException().Message));
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
    return json.loads(result.stdout)


def update_manifest(capture: dict[str, object]) -> int:
    manifest = json.loads(CASES.read_text(encoding="utf-8"))
    by_case = {result["Case"]: result for result in capture["results"]}
    failures = 0
    for case in manifest["cases"]:
        result = by_case[case["case"]]
        if result["Status"] == "captured":
            case["expected"]["status"] = "captured"
            case["expected"]["value"] = result["Value"]
            case["known_unverified"] = [
                note
                for note in case.get("known_unverified", [])
                if "not captured" not in note and "pending" not in note.lower()
            ]
        else:
            failures += 1
            case["expected"]["status"] = "capture_failed"
            case["expected"]["value"] = None
            case["known_unverified"] = [f"C# capture failed: {result['Error']}"]
    manifest["capture_tool"] = "tools/capture_spectrum_visualizer_frames.py"
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
