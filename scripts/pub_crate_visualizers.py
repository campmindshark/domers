#!/usr/bin/env python3
"""Add pub(crate) to top-level fn/struct/enum/const/static in visualizer modules."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "crates" / "visualizers" / "src"

SKIP = {"lib.rs", "prelude.rs", "tests/mod.rs", "inventory.rs", "input.rs", "quaternion.rs"}
SKIP_PREFIXES = ("tests/",)

VIS = re.compile(
    r"^((?:pub(?:\(crate\))? )?(?:struct|enum|fn|const|static|type|impl|trait) )",
    re.MULTILINE,
)


def should_pub(crate: bool, line: str) -> bool:
    stripped = line.lstrip()
    if stripped.startswith("pub ") or stripped.startswith("pub("):
        return False
    if stripped.startswith("#") or stripped.startswith("//"):
        return False
    return stripped.startswith(("struct ", "enum ", "fn ", "const ", "static ", "type "))


def process(path: Path) -> None:
    rel = path.relative_to(ROOT).as_posix()
    if rel in SKIP or any(rel.startswith(p) for p in SKIP_PREFIXES):
        return
    text = path.read_text()
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    for line in lines:
        if should_pub(True, line):
            indent = line[: len(line) - len(line.lstrip())]
            out.append(f"{indent}pub(crate) {line.lstrip()}")
        else:
            out.append(line)
    path.write_text("".join(out))


def main() -> None:
    for path in sorted(ROOT.rglob("*.rs")):
        process(path)


if __name__ == "__main__":
    main()
