# 10: track sag (coordinator)

## Contract

Every tracked belt sags between the supports of its top run (user decision T4).
No call site changes. The belt never enters a wheel.

The renderer's track node contract is unchanged: `track_L/R`,
`track_length_m` and `link_pitch_m` (`vehicle_parts.py:22-24`). The belt still
scrolls its UVs by its length.

## Seam

```python
# running_gear.py
TRACK_SAG = 0.03   # metres of droop per metre of unsupported top run (gentle)
def track_path(wheels, thickness, sag=TRACK_SAG) -> list[tuple[float, float]]
"""Pure. The belt's closed counter-clockwise centre line: the convex hull of
the grown wheel circles (as today), with every top-run tangent segment between
two different wheels (outward normal z > 0.5) replaced by a parabola sampled at
8 points. Mid-span drop = min(sag * L, clearance - thickness / 2 - 0.01), where
clearance is the gap down to the highest wheel circle under the span."""
def track_loop(name, wheels, y, width, pitch, mats, parent, thickness=0.08, sag=TRACK_SAG)
def tracked_running_gear(..., sag=TRACK_SAG)   # passes through
```

- `track_length_m` becomes the length of the sagged path.
- No family overrides `sag` in this pass.
- The clearance clamp is built into `track_path`, because the lint can't catch
  a sag cutting into a wheel: tracks and wheels sit under the same top node.

## Also in this slice

The lint's ground-contact check finds tracks by name (`"track_" in name`), a
second definition of what a track is (slice 01). Make it use the running-gear
node contract this module owns.

## The variable and crop

- **The one variable:** the shape of the top run.
- **The crop:** a low side view of the running gear only. Shoot:
  - the T-72B3 (low return rollers);
  - the Leopard 2A6 (four return rollers);
  - the ZTQ-15 (no return rollers visible).
- **Out of scope:** wheel and skirt shapes.

## Steps

1. Write `track_path` and its clamp.
2. Re-export the three models.
3. **Closeups**, then **`compare-screenshots`** against before.
4. **Last check: an unprimed `screenshot-critique`** of the crop.
5. **Checkpoint,** which never blocks.

## Must stay green

- the GLB node contract;
- lint;
- the rendered belt animation, checked with `asset sheet` on one model under
  the GPU lock.

## Delegated

- samples per span;
- parabola or catenary.

## Feedback that would change it

"More Soviet sag." Slice 17 then passes `sag=0.06` for the T-72 family, and
that is recorded.

## Result (2026-11-11)

**Contract.** The GLB node contract is unchanged: `track_L/R` carrying
`track_length_m` and `link_pitch_m`, with the belt the only mesh under each.
`track_length_m` is now the sagged path's length. In all three exports the
belt's u runs from 0 to `track_length_m / link_pitch_m` on every tier. The
seam is as planned, with one difference: `track_path(wheels, thickness,
sag=TRACK_SAG)` samples a span every `SAG_STEP` (5 cm), not at 8 points. It
does this in two places:
- `_hang`, which handles one span;
- `_clearance`, which finds the nearest wheel under a span.

`track_loop` and `tracked_running_gear` pass `sag` through. No call site
changed, and `abrams.py`'s direct `track_loop` call sags by the default.

**The shape.** Each top-run span from one wheel to another hangs as a vertical
parabola. A top-run span is one whose outward normal is more up than
sideways. Its drop is `TRACK_SAG` (0.03) times its length at mid-span. Belt
lengths grow by 1.4–1.5 cm.

| Model | Spans (front to rear), drop | `track_length_m` before → after |
|---|---|---|
| T-72B3 | 4.1, 4.8, 4.7, 4.1 cm | 13.969 → 13.984 m |
| Leopard 2A6 | 4.0, 4.8, 4.2, 3.7, 2.7 cm | 15.102 → 15.118 m |
| ZTQ-15 | 6.5, 6.0, 6.6 cm | 14.701 → 14.717 m |

**The clamp.** The drop is held so the belt's inner face stays 1 cm
(`SAG_CLEARANCE`) above the nearest wheel under the span. It never engages
on these three models at 0.03.
- **At `sag=0.06`** (the "more Soviet sag" feedback), it engages on the
  T-72's two end spans: 8.2 cm is cut to 6.7 cm at the front and 8.3 cm to
  8.0 cm at the rear, held off the first and last road wheels.
- **On low rollers** (a synthetic rig with rollers barely above the road
  wheels), the rollers fall under the straight idler-to-sprocket line, so
  they aren't supports. That 5.8 m span would sag 17 cm; the clamp holds it
  to 5 cm, 1 cm above the rollers' tops.
- **A span with no room stays straight.**

Two further rules keep the belt out of its wheels:
- **A wheel the straight span touches carries it.** The hull drops collinear
  points, so rollers at equal heights fell inside one span, and the clamp
  then kept that span straight. The Leopard's 0.89 m pair is one such case.
  The span now splits at that wheel.
- **A point that would dip into a support is pushed onto its rim.** As the
  curve leaves a support, it wraps further round it, as a sagging belt does.
  Without this, a steep sag cut up to 9 mm into the idler.

**Proof.**
- `running_gear_test.py` runs in Blender. It checks three things:
  - the top run sags between rollers;
  - no belt vertex sits more than the lint's 5 mm inside any wheel circle,
    on high and low rollers, at 0.03 and at 0.2;
  - the belt's drawn centre line is its `track_length_m` within 0.5 %.

  Both bugs above failed it before they were fixed.
- `vehicle_lint_test.py`: its 15 tests pass, the hovering-belt fixture now
  under its `track_L` node.
- Lint, every issue of the three appearances before and after:
  - T-72B3 48 → 47 issues, Leopard 17 → 17, ZTQ-15 18 → 18, none new;
  - no ground-contact change;
  - `exhaust_L_bore` against `track_L_band` clears;
  - the rest of the movement is depth readings on pairs already in the
    baseline: the T-72's fender against its belt 36.6 → 39.3 mm, and its
    exhaust pipe 28.3 → 14.5 mm. The Leopard's hull against its belt
    23.3 → 27.8 mm, and the ZTQ-15's 120 → 116/113 mm.

**Ground contact.** `check_ground_contact` takes belts from the node contract
(`vehicle_lint_core.TRACKS`, the same reading the overlap grouping uses), and
the belt is everything under the node. Before, it took any object with
`track_` in its name, fenders like `track_guard_L` included.

**Closeups.** These are in `assets/after/10/`. A low side crop of the left
running gear alone (`-gear-before-after`, `-span-after`), and the whole
vehicle (`-hull-after`).

`compare-screenshots` telemetry, before → after:

| Pair | Distance | Pixels changed (32+ grey levels) |
|---|---|---|
| Gear-only full length | 0.023–0.028 | 2.3–2.8 % |
| Gear-only span crop | 0.056–0.068 | about 8 % |
| Whole vehicle | 0.0004–0.0034 | under 0.3 % |

All movement is in the top run. Verdict: **the candidate is less wrong.** The
top run hangs between its supports, rests on each roller and enters no
wheel. On all three whole vehicles, the skirts hide the top run, so the sag
shows only in slivers through skirt gaps. It will read on skirtless
vehicles.

**Critique** (unprimed, the gear crop and whole vehicle, all 24 shots).

| Model | Silhouette | Surface | Fittings | Materials | Cohesion |
|---|---|---|---|---|---|
| T-72B3 | 6 | 4 | 6 | 5 | 6 |
| Leopard 2A6 | 6 | 4 | 5 | 5 | 6 |
| ZTQ-15 | 6 | 4 | 5 | 4 | 5 |

It found no belt passing through any wheel and no gap on the lower run, and
called the sag "a gentle, believable droop". Its fixes, and what became of
them:
1. **A sharp peak at every return roller; replace it with a curve over the
   roller.** Not adopted. The peak is the belt wrapping a 10 cm roller,
   which bends a real track at its link pins. A rounder top would need the
   belt to lift off the roller. The "off-centre" peaks are the camera seeing
   the roller's near face offset from its far face.
2. **The belt reads as a smooth ribbon (no grousers, guide horns or pins).**
   Out of scope. The belt's surface is the track material, and nothing in
   this pass owns it.
3. **The sprocket teeth don't visibly mesh.** Out of scope (wheel shapes).
4. **The ZTQ-15's rear roller nearly touches a road wheel.** It is 7 cm
   clear in its rig. The road wheel is outboard, nearer the camera.
   Family data for slice 19 if it reads in that slice's crop.
5. **Camouflage on the wheels, and faceted rollers and rims.** Family and
   material work, not this slice.

The surface and materials scores sit below the bar for reasons outside this
slice's variable. The shape of the top run is what it was asked to change.

**The animation.** `asset sheet` ran on the T-72B3 under the GPU lock, as a
whole vehicle and as gear only, before and after. The running-gear strip
draws the sagged belt in the production renderer at every travel step
(`-sheet-running-gear.png`: three after frames, then two before), and the
sprocket turns between frames. The link marks are below the strip's
resolution, so the scroll is checked by its contract: on every tier the belt's
u spans `track_length_m / link_pitch_m` (93.224 links on the T-72B3, 83.988
on the Leopard, 98.111 on the ZTQ-15). The renderer's scroll code is
untouched.

The sheet needed more setup in a fresh worktree:
- `bun run build:wasm`;
- `bun run build:mechanics` (the workbench route answers 500 without it);
- `asset pull`, for the runtime catalog;
- `git lfs pull -I assets/third-party/effects` (its two effect sheets).

**Unfinished.**
- No roster-wide survey of which families' spans the clamp holds. The three
  models here, and the low-roller rig, are what was measured.
- The Blender README's running-gear row doesn't mention sag. That README is
  outside this lane's files.
