# 02 — Native runner

**Contract:** `native/godot` is a Forward+ (Metal) client that reads one scene's capture and manifest, refuses mismatches, steps frame records by index and writes plates and a report. It never falls back, caps or skips anything silently. This slice runs in parallel with slice 01 against a small hand-written capture fixture.

**Seam:**
- **Project:**
  - `git mv native/godot-spike native/godot`. Update every reference: READMEs, specs, the binding smoke and `.gitignore`.
  - `project.godot` uses `forward_plus` only.
  - A project `.gitignore` covers `.godot/`, `*.import`, `staging/` and `reports/`.
- **Modules,** split out of `reel.gd`:
  - `capture/presentation_capture.gd`: the strict parser for the new capture.
  - `capture/visual_manifest.gd`: `load(dir) -> {ok, manifest, blocker}`. It hashes the map and capture bytes (sha256) and refuses any mismatch.
  - `reel/frame_stepper.gd`: `step() -> record | null`. It visits each index once and uses no delta or wall clock.
  - `report/render_report.gd`: `add_budget{owner, reason, sourceId, admitted, rendered, dropped}`, `add_blocker{name, detail}`, `add_plate{index, path, gpuMs}` and `write(path)`.
  - `reel/plate_writer.gd`: awaits `RenderingServer.frame_post_draw`, then reads back the viewport. GPU time comes from `viewport_get_measured_render_time_gpu`. Plate size must equal the record's canvas, or it's a blocker.
  - `parity_run.gd`: the entry point.
- **Command:** `godot --path native/godot --window-position 10000,10000 -- --run <dir> --scene <map> [--frames plates|all|<list>]`. Rendering needs a display; `--headless` cannot render.
- **Assets:** `assets/staging.gd` links one file at a time into the ignored `staging/` folder, then runs one `--import`. Directory symlinks make Godot write `.import` files into the real `assets/` tree. A missing or unimported asset, or an LFS pointer, is a named blocker.
- **Delete:**
  - the authored-orbit fallback (`reel.gd:1242-1259`)
  - wall-clock stepping and `GODOT_REEL_TIME_SCALE`
  - the hard-coded 30 Hz
  - every `GODOT_*` cap and default limit (`reel.gd:27-66, 836`)
  - fog-cell painting: fog is still decoded but drawn only if the manifest lists it
  - the cut-capture lambda, which never sees `drawn` because GDScript closures capture locals by value (`reel.gd:1310-1316`)
  - the per-frame unit material allocation, and unit reuse by index rather than by id (L15)
  - the `break` on a bad prop row (`reel.gd:381`)
- **Scenes:** buildings, props and units keep their current placement code, moved to `world/legacy_map_layer.gd` and reported as approximations, until phase 2.

**Runnable artifact:**
- One display-backed run on the hand-written fixture writes plates and a report.
- A Forward+ probe on real market-town content (one staged building, a grass multimesh, a shadowed sun and the sky) renders without a blocker.

**Visual variable:** none. The coordinates are still mirrored here.

**Focused gates:**
- Headless GDScript tests: the parser accepts the new shape and rejects stale shapes; identity mismatch is refused; a missing layer isn't drawn; the stepper visits every index exactly once; the report round-trips.
- Tests check and call `quit(1)` on failure. A failing `assert` hangs a script-mode Godot run.
- Tests clear `GODOT_*` from their environment.
- Display-backed: the plate is exactly the record's canvas size.

**Out of scope:** coordinates, camera, light, terrain and grass.

**Delegated:** internal module layout below the named files, and the hash library call.

**Stays green:** `binding_smoke` (moved with the rename), and the Rust digest and replay tests.
