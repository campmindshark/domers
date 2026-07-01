#!/bin/bash
# Apply known-good fixes after split_visualizers_v2.py
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/crates/visualizers/src"
BAK=$(git -C "$ROOT" show HEAD:crates/visualizers/src/lib.rs)

# Fresh split from git source
echo "$BAK" > "$SRC/lib.rs.bak"
cp "$SRC/lib.rs.bak" "$SRC/lib.rs"
python3 "$ROOT/scripts/split_visualizers_v2.py"

# Append VisualizerRuntime (v2 script bug: body missing)
git -C "$ROOT" show HEAD:crates/visualizers/src/lib.rs | sed -n '370,465p' | python3 -c "
import sys
for line in sys.stdin:
    s=line.lstrip()
    if s.startswith(('struct ','enum ','fn ','const ','static ')) and not s.startswith('pub'):
        indent=line[:len(line)-len(s)]
        line=f'{indent}pub(crate) {s}'
    sys.stdout.write(line)
" >> "$SRC/runtime/mod.rs"

# Fix runtime mod doc + pub struct
python3 - <<'PY'
from pathlib import Path
p = Path("/home/twoshark/repo/campmindshark/domers/crates/visualizers/src/runtime/mod.rs")
t = p.read_text()
t = t.replace(
    "/// Persistent per-visualizer runtime driving the live and sandbox render loops.\n///\n/// Unlike",
    "/// Persistent per-visualizer runtime driving the live and sandbox render loops.\n#[derive(Clone, Debug, Default)]\n/// Unlike",
)
t = t.replace("#[derive(Clone, Debug, Default)]\n/// Unlike [`render_dome_visualizer`]", 
              "#[derive(Clone, Debug, Default)]\npub struct VisualizerRuntime_PLACEHOLDER")
# simpler: ensure derive before pub struct
if "#[derive(Clone, Debug, Default)]" not in t:
    t = t.replace("pub struct VisualizerRuntime", "#[derive(Clone, Debug, Default)]\npub struct VisualizerRuntime", 1)
# remove duplicate derive if any
while t.count("#[derive(Clone, Debug, Default)]") > 1:
    t = t.replace("#[derive(Clone, Debug, Default)]\n#[derive(Clone, Debug, Default)]", "#[derive(Clone, Debug, Default)]", 1)
p.write_text(t)
PY

python3 "$ROOT/scripts/apply_visualizer_fixes.py" || true
echo "Done — run cargo check"
