# Stills phase (GPU) — reference color grade

Do this only when heli v5.1 and any cv90 capture are idle. Disk is tight.

## Lock

```bash
COMMON="$(git -C ~/tmp/color-grade rev-parse --path-format=absolute --git-common-dir)"
LOCK="$(dirname "$COMMON")/throwaway/gpu.lock"
# refuse if siblings hold GPU
ps -axo pid=,command= | grep -E '/native-client( |$)|[/ ]ffmpeg( |$)|hdr-capture|native-parity-capture' | grep -v grep || true
```

Use `lockf -k "$LOCK"` around every GPU command. Release as soon as stills finish (lockf exits).

## Build once

```bash
cd ~/tmp/color-grade
export CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2
cargo build --release -p native-client
bun run build:wasm   # if recording new parity runs
```

## Maps / captures

| Shot | Capture / map | Notes |
| --- | --- | --- |
| CV90 hills | `cv90-hill` | map+capture copied from cv90-hill-v2 for proof; camera settle ~7500–10730 ms |
| Paris city | `paris-corner` (or graphics-test / publication) | existing fixture |
| NY town | `market-town` | existing fixture |
| Desert/plain | `ground` or `ambush` | open terrain; use a mid orbit still |

## Before / after protocol

New light is in `fixtures/presentation.json`. Baseline saved at
`throwaway/color-grade/light-before.json`; new at
`throwaway/color-grade/AFTER_LIGHT.json`.

```bash
ROOT=~/tmp/color-grade
OUT=$ROOT/throwaway/color-grade/stills
mkdir -p "$OUT"/{before,after,compare,phone}

# BEFORE: swap light
python3 - <<'PY'
import json
from pathlib import Path
root=Path('/Users/server/tmp/color-grade')
d=json.loads((root/'fixtures/presentation.json').read_text())
d['light']=json.loads((root/'throwaway/color-grade/light-before.json').read_text())
(root/'fixtures/presentation.json').write_text(json.dumps(d, indent=2)+'\n')
PY
# record parity + still for each map into $OUT/before/ ...
# AFTER: restore AFTER_LIGHT.json the same way, still into $OUT/after/

python3 - <<'PY'
import json
from pathlib import Path
root=Path('/Users/server/tmp/color-grade')
d=json.loads((root/'fixtures/presentation.json').read_text())
d['light']=json.loads((root/'throwaway/color-grade/AFTER_LIGHT.json').read_text())
(root/'fixtures/presentation.json').write_text(json.dumps(d, indent=2)+'\n')
PY
```

Per map (example CV90):

```bash
PARITY_CAPTURE=cv90-hill PARITY_INDICATORS=0 PARITY_CARDS=0 PARITY_VIEWPORT=1620x1080 \
  lockf -k "$LOCK" bun run --cwd web scene -- native-parity-capture
RUN=$(ls -1dt throwaway/native-parity/*/cv90-hill | head -1 | xargs dirname)
lockf -k "$LOCK" target/release/native-client --run "$RUN" --scene cv90-hill \
  --out /tmp/cg-still --fps 1 --from-ms 9370 --to-ms 10370 --window offscreen --pace 0 \
  --no-indicators --no-cards --no-fog
```

## Compare / critique

- Side-by-side CV90 after vs `refs/norwegian-tank-afghanistan.jpg` via compare-screenshots skill.
- Before/after montages per map; phone crops ~390×844 center.
- screenshot-critique on after stills (color, haze, sky blue, unit/UI/fog readability).
- Copy proofs to box `/workspace/armata/posts/color-grade/`.

## Do not

- Do not push/PR.
- Do not leave presentation.json on light-before after the phase — restore AFTER_LIGHT.
- Do not touch other worktrees.
