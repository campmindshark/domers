#!/usr/bin/env python3
"""Check whether Spectrum visualizer golden frame hashes are complete."""

from __future__ import annotations

import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CASES = ROOT / "fixtures" / "spectrum-csharp" / "visualizer_frame_cases.json"


def main() -> int:
    manifest = json.loads(CASES.read_text(encoding="utf-8"))
    pending = [
        case["name"]
        for case in manifest["cases"]
        if case.get("expected", {}).get("value") is None
        or case.get("expected", {}).get("status") != "captured"
    ]
    if pending:
        print("pending Spectrum visualizer goldens:")
        for name in pending:
            print(f"- {name}")
        print(
            "\nRun the Spectrum C# visualizer capture on a Windows/.NET machine, "
            "write each expected.value, and set expected.status to 'captured'."
        )
        return 1

    print(f"visualizer goldens complete: {len(manifest['cases'])} cases")
    return 0


if __name__ == "__main__":
    sys.exit(main())
