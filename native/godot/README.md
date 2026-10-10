# Native Godot client

A second client beside the browser. Godot draws what the browser recorded,
and Rust stays the only authority. It renders a browser capture run frame by
frame, so the two clients can be compared side by side under the same frame
records. The plan it serves is
[`specs/native-rendering-parity`](../../specs/native-rendering-parity/README.md).

The project renders with Forward+ on Metal only. Only macOS is verified.

## The parity run

One run renders one scene that the browser's capture run wrote:

```text
lockf -k <main checkout>/throwaway/gpu.lock \
  godot --path native/godot --window-position 10000,10000 -- \
  --run <dir> --scene <name> [--frames plates|all|<i,j,...>] [--maps <dir>] [--out <dir>]
```

- **Input.** `<dir>/<name>/` holds the browser's `capture.json`
  (`battle-presentation-capture`) and `manifest.json`
  (`native-visual-presentation`). The map comes from `--maps`, by default
  `fixtures/maps`. Relative paths resolve from the repository root.
- **Identity.** The run refuses a battle whose capture or map bytes don't
  match the manifest's sha256, or whose scene, map, encounter or seed differ
  from the capture's.
- **Stepping.** The run steps frame records by index: `plates` (the records the
  browser shot), `all`, or a list. There is no wall clock. Each record sets
  the camera from its eye, target and up, and shows the latest publication
  at or before its tick.
- **Output.** For each record, the run writes `plate-<index>.png`, exactly the
  record's canvas. It also writes a `godot-render-report`. Both go to `--out`,
  by default `<dir>/<name>/native/`, which belongs in scratch.
- **Report.** It names every blocker that stopped the run. Each budget row
  gives the owner, the reason, the source, and the counts admitted, rendered
  and dropped. A plate row carries its measured GPU time, which is null on
  Metal: Godot's Metal driver records no GPU timestamps.
- **Exit codes.** 0 means no blocker, 1 means a blocker, 2 means bad arguments.

Rendering needs a display. `--headless` uses Godot's dummy renderer, which
never draws, so a headless run stops at a `display` blocker. The window is
placed off screen, which still needs a logged-in macOS session.

Only the layers the manifest lists are drawn, under the browser's layer names.
A listed layer that native can't draw is reported as dropped. Fog is decoded
from every publication but painted only when `fog` is listed.

## Assets

The run links each repository asset it draws, one file at a time, into the
ignored `staging/` folder. It then imports whatever is new with one
`godot --import`. Linking a directory instead would make Godot write `.import`
files into the real `assets/` tree.

A missing file, a Git LFS pointer or a failed import is a named blocker, never
a stand-in. Fetch what a scene needs with `git lfs pull --include=<path>`.

## Legacy map layer

Until phase 2 ports the browser's model owners, the map is drawn roughly:
- a flat ground with surfaces, road boxes and forest patches;
- catalog grass tufts;
- building shells picked from a catalog kit by template token;
- catalog props or boxes;
- units as catalog models or capsules.

Its coordinates are mirrored. Every category is reported as an approximation,
and rows farther than 1 km from every frame target are reported as dropped.

## Checks

- `native/godot/tests/run.sh [tests/<name>_test.gd ...]` runs the headless
  tests. A test passes only when Godot exits 0 and prints its result line,
  because a GDScript parse error also exits 0. Script-mode tests can't use
  `assert`, because a failing assert hangs the run.
- The fixture run is the runner's display-backed check. It renders the
  hand-written capture under `tests/fixtures/`:
  `--run native/godot/tests/fixtures/run --scene fixture-town --maps native/godot/tests/fixtures/maps --out throwaway/native-runner/fixture`.
  A fixture edit changes its bytes, so update the hashes in its manifest.
- `tests/forward_plus_probe.gd` is the backend's display-backed probe. It
  renders a real market-town building from its staged kit, a grass MultiMesh,
  a shadowed sun and the sky. Run it with
  `godot --path native/godot --window-position 10000,10000 --script res://tests/forward_plus_probe.gd`.
  It needs `assets/source/city/homes/kit.glb` and
  `assets/source/grass/grass_meadow.glb` from Git LFS.
- The [binding smoke](binding_smoke/) checks the Rust GDExtension against the
  offline `native_binding_probe` in a scratch project. It has its own
  `.gdignore`d folder, so this project never loads the binding:
  `binding_smoke/binding_smoke.sh <godot> <scenario.json>`.
