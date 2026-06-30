#!/usr/bin/env python3
"""Build the legacy Spectrum C# project with Windows .NET.

The Spectrum solution includes a Madmom Python project supplied by the
`campmindshark/madmom` submodule. Plain `dotnet build Spectrum.sln` cannot build
that `.pyproj` without Visual Studio Python Tools, so the default fixture gate
builds the WPF entry project directly while ensuring the submodule is present.
"""

from __future__ import annotations

import argparse
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPECTRUM = ROOT.parent / "spectrum"
DEFAULT_PROJECT = SPECTRUM / "Spectrum" / "Spectrum.csproj"
MADMOM_PROJECT = SPECTRUM / "Madmom" / "Madmom.pyproj"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--project",
        type=Path,
        default=DEFAULT_PROJECT,
        help="Spectrum project or solution to build.",
    )
    parser.add_argument(
        "--configuration",
        default="Debug",
        choices=("Debug", "Release"),
        help="Build configuration.",
    )
    parser.add_argument(
        "--dotnet",
        type=Path,
        default=None,
        help="Explicit dotnet executable. Overrides SPECTRUM_DOTNET.",
    )
    return parser.parse_args()


def is_wsl() -> bool:
    return "microsoft" in platform.release().lower()


def wslpath(path: Path) -> str:
    result = subprocess.run(
        ["wslpath", "-w", str(path)],
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    return result.stdout.strip()


def powershell_dotnet_command(
    dotnet: str | None, project: Path, configuration: str
) -> list[str]:
    spectrum_win = wslpath(SPECTRUM)
    project_win = wslpath(project)
    if dotnet is None:
        dotnet_setup = "$dotnet = Join-Path $env:USERPROFILE '.dotnet\\dotnet.exe'; "
        dotnet_invoke = "$dotnet"
    else:
        dotnet_setup = ""
        dotnet_invoke = quote_ps(dotnet)
    command = (
        "$env:DOTNET_CLI_TELEMETRY_OPTOUT = '1'; "
        f"{dotnet_setup}"
        f"Set-Location {quote_ps(spectrum_win)}; "
        f"& {dotnet_invoke} build {quote_ps(project_win)} "
        f"--configuration {configuration} --nologo"
    )
    return ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", command]


def quote_ps(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


def ensure_madmom_submodule() -> None:
    if MADMOM_PROJECT.exists():
        return
    gitmodules = SPECTRUM / ".gitmodules"
    if not gitmodules.exists() or "Madmom" not in gitmodules.read_text(encoding="utf-8"):
        return
    subprocess.run(
        ["git", "-C", str(SPECTRUM), "submodule", "update", "--init", "Madmom"],
        check=True,
    )


def default_windows_dotnet() -> str | None:
    env_dotnet = os.environ.get("SPECTRUM_DOTNET")
    if env_dotnet:
        return env_dotnet
    return None


def local_dotnet(args_dotnet: Path | None) -> str:
    if args_dotnet is not None:
        return str(args_dotnet)
    env_dotnet = os.environ.get("SPECTRUM_DOTNET")
    if env_dotnet:
        return env_dotnet
    found = shutil.which("dotnet")
    if found:
        return found
    return "dotnet"


def main() -> int:
    args = parse_args()
    ensure_madmom_submodule()
    project = args.project.resolve()
    if not project.exists():
        print(f"Spectrum project not found: {project}", file=sys.stderr)
        return 2

    env = os.environ.copy()
    env["DOTNET_CLI_TELEMETRY_OPTOUT"] = "1"

    if is_wsl():
        dotnet = str(args.dotnet) if args.dotnet else default_windows_dotnet()
        command = powershell_dotnet_command(dotnet, project, args.configuration)
    else:
        dotnet = local_dotnet(args.dotnet)
        command = [
            dotnet,
            "build",
            str(project),
            "--configuration",
            args.configuration,
            "--nologo",
        ]

    return subprocess.run(command, env=env, check=False).returncode


if __name__ == "__main__":
    sys.exit(main())
