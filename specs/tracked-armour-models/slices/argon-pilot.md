# Argon pilot: the T-72B3 and the Abrams SEPv2 in the game

The pilot took Gemini Argon's round-2 builds of the T-72B3 and the M1A2 SEPv2
all the way into the game, and wrote down the recipe the other tracked units
follow. Its decisions are P1 to P16 in [`decisions.md`](../decisions.md)
("Argon pilot decisions").

## Contracts

- **A unit's build script is a source.** `build.py` lives with its rounds in
  `assets/source/skins/<family>/argon-r2/`. Run alone
  (`OUT_DIR=<dir> blender -b -P build.py`), it renders and exports. Imported,
  its `build()` builds the vehicle into the caller's scene. It names each
  part by what moves it: `hull`, `turret`, `gun`, `hmg`, `hmg_gun`, `wheel_*`,
  `track_L`/`track_R` and `dressing_*`. It paints with the seven slots: `armor_paint`,
  `steel`, `rubber`, `optics`, `fittings`, `lamps` and `recesses`.
- **`scripted_body.wear(v, path, link_pitch)`** (new,
  `packages/scene-assets/blender/scripted_body.py`) runs the script and fits
  it to the export contract every procedural build keeps. `scripted_body.wreck`
  is a worn body's damage.
- **The family receipt** hashes every repository source the export loaded,
  the build script included (`vehicle_export._receipt`). Its `authoring`
  line names scripted bodies.
- **`vehicle_export.Vehicle.looks`** carries the family's `materials`
  keywords, and `vehicle_export.CHIP` names the default edge wear.
- **Unchanged:**
  - the GLB node contract (`turret`, `gun`, `muzzle`, `hmg`, `hmg_gun`,
    `hmg_muzzle`, `wheel_*` with `radius_m`, `track_L/R` with `track_length_m`
    and `link_pitch_m`, `dressing_*`, `_LOD0..3`);
  - frames, fixtures and the simulation, so no battle digest can move;
  - the lint baseline, except that its walk drops the two procedural models'
    entries (P13).

## What shipped

| | T-72B3 | Abrams SEPv2 |
|---|---|---|
| Tiers (live) | 78,364 / 31,656 / 7,722 / 3,078 | 57,682 / 32,358 / 10,240 / 3,720 |
| Tiers (wreck) | 97,468 / 34,228 / 7,358 / 2,878 | 77,352 / 35,502 / 9,362 / 3,392 |
| Caps (`tracked_heavy`) | 120k / 45k / 13k / 4k | same |
| Lint | 0 issues | 0 issues |
| `asset validate` | 0 findings | 0 findings |
| Local `build.py` edits | 13 (folder README) | 8 (folder README) |

- `asset bake` and `asset check` pass, and so do the Blender-side Python
  tests. All 276 tests in `web/tests/sceneAssets` pass, `unitFit` among them.
- **Catalog load:** 598.1 MiB for the game page, and 636.8 MiB for every
  unit's art against the 640 MiB limit. Slice 16 owns raising the limit (Q3).
- **To make the bake green,** the families whose slice-15 frames were never
  re-exported were exported (P14). The Abrams family's two procedural
  siblings were also re-exported, because the family exports as one (P12).

**Shots.** The game renderer, through the model workbench (`asset sheet`):
- [`assets/after/argon-pilot/t72-compare.png`](../assets/after/argon-pilot/t72-compare.png)
- [`assets/after/argon-pilot/abrams-compare.png`](../assets/after/argon-pilot/abrams-compare.png)

Each sheet shows the before (procedural), the after and the reference, from
the three-quarter front, the side, the front and the three-quarter rear.

## Critique (unprimed, after the fixes below)

| | silhouette | surface | fittings | materials | cohesion |
|---|---|---|---|---|---|
| T-72B3 | 6 | 6 | 6 | 5 | 5 |
| Abrams SEPv2 | 7 | 6 | 6 | 4 | 5 |

Neither model reaches the bar yet (no score under 6, an average of at least
7). What the critique found, and what came of each:

- **Fixed, ours:**
  - the T-72's V splash board lay pitched by the whole glacis slope, like a
    loose plank (build.py edit);
  - the Abrams' gun bands read as bare black rings (now paint).
- **Fixed before the critique, found in the game shots:**
  - Argon's convex edges chipped whole small blocks pale. The edge wear is
    now scaled to the procedural parts' (P8).
  - Painted hardware read as bare steel (P6).
- **Not ours here, the shared look:**
  - The paint's mottle and dust come from the scheme's texture recipe, the
    same as on every procedural tank. "Clean factory paint" is a recipe
    question for the whole roster.
  - The olive cast of shaded tan faces is the workbench's lighting and
    grading. The procedural Abrams shows the same.
- **The belt looks plain:** a smooth band, with links only in the game's
  scrolling texture. Every tracked unit has this, and it is the cost of P4.
- **The Abrams "see-through gap" over the wheels** shows only in the Blender
  closeup renderer, which draws the `recesses` slot white. The game shows a
  dark wall. It also closed a real gap: the procedural Abrams has one under
  its skirts.
- **Open, low confidence:**
  - the T-72's smoke-tube cluster may sit wrong against photos;
  - a soft "saucer" foot round its dome;
  - a shading smear on the Abrams' rear turret flank, seen in matcap only.

  Each is a `build.py` edit for the family lane.

## The recipe for every other tracked unit

### 1. Brief Argon (one Bento session per unit, `--tier google-recommended`)

Check the first turn's `usageByModel` (A5). The brief carries:

- **The references:** the unit's photos from `assets/references/<family>/`,
  and our current GLB as "our current model, for proportions only".
- **The measurements, from the frame** (`fixtures/catalog.json` and
  `specs/tracked-armour-models/assets/frames/<family>.json`):
  - the hull box (length, width, height), centred on x = 0 and y = 0 and
    standing on z = 0;
  - each mount's pivot and its world muzzle (pivot + `muzzle_m`);
  - the roof height.
  Give the current, post-slice-15 numbers (A3's lesson).
- **The structure:**
  - module-level helpers;
  - `def build():` builds the vehicle into the current scene with no reset,
    render or export, and returns its root;
  - `def main():` resets, builds, renders five views (three-quarter front,
    side, front, top, three-quarter rear), saves `<unit>.blend` and exports
    `<unit>.glb` to `os.environ["OUT_DIR"]`;
  - `if __name__ == "__main__": main()`;
  - no randomness without a fixed seed.
- **The part names:**
  - `hull`, with everything fixed to it;
  - `turret` (origin at the ring);
  - `gun` (origin at the trunnion);
  - `hmg`, the roof gun's turning base, and `hmg_gun`, the part that pitches;
  - one `wheel_<side>_<name>` object per wheel that turns: every road wheel,
    sprocket, idler and return roller, each with its origin on its axle;
  - `track_L` and `track_R`;
  - `dressing_<name>` for sights and sensors standing over the roof line.
- **The slots,** and only these seven:
  - `armor_paint`: every painted surface, bolts and hinges included;
  - `fittings`: dark painted hardware;
  - `steel`: bare metal only (track pins, tow cables, sprocket teeth, MG
    barrels, the muzzle);
  - `rubber`, `optics`, `lamps` and `recesses`.
- **What to leave out:** antenna whips (we add ours), crew, open hatches,
  markings and weathering.
- **The clearances:**
  - nothing of the hull, skirts or stowage inside the track run's box
    (between the hull's side and the skirts, from the ground to the run's
    top);
  - nothing of the hull within the turret's swept radius above the ring;
  - axle stubs end at the hull's side;
  - road wheels, return rollers, sprocket and idler don't overlap side-on;
  - the run's top stays under the sponsons.
- **The limits:**
  - under 80k triangles;
  - edges as two-segment bevels or sharp, not single-segment bevels, which
    read as full edges to the paint's wear (P8).

### 2. Two rounds (A6)

- **Round 1** builds it.
- **Measure the GLB** against the frame: the hull box, turret extents, each
  pivot and world muzzle, the roof. Run the pilot's integration once (step 4)
  and send what it flags: lint pairs, `asset validate` findings, tier counts.
- **Round 2** takes those as numbered fixes.
- **Archive both rounds** in `assets/source/skins/<family>/argon-r1/` and
  `argon-r2/`: `build.py`, `model.glb` (renamed from the exported
  `<unit>.glb`), `notes.md`, `renders/`, and the comparison sheet, if the
  session made one.

### 3. Local edits (no more Argon)

Patch `build.py` until it keeps the contract above. The pilot's edits are
the catalogue of what to expect:
- splitting fused wheels and roof guns out of the parts they were built into;
- moving the roof gun onto the frame's pivot;
- running gear and stowage clear of each other;
- painted hardware off `steel`;
- sights into dressing.

Number each edit in the folder README. Rebuild it standalone into
`throwaway/` and copy back `model.glb` and the renders.

### 4. Integrate

- **The family script** (`roster/<family>.py`):
  - add `BODY` (or `BODIES` by variant id) and the track's `LINK_PITCH`;
  - `build` makes the rig (`rig(v.frame, v.root, trunnion=...)`), calls
    `SB.wear(v, BODY, LINK_PITCH)` and places the whip antennas
    (`RF.antenna` under `dressing_antenna_*`);
  - `wreck` calls `SB.wreck(v, side, wheels)`.
- **Delete the procedural code** only that unit used. Keep what other
  families import (the T-72's hull for the T-80, T-90 and Type 99, the
  Abrams' for the M1E3) until they wear their own.
- **Export:**
  `bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/<family>.py`,
  then the same with `-- --wreck`. Fix every lint issue in `build.py`. To
  find where a pair meets, build the variant in Blender and test each part's
  vertices against the other with `vehicle_lint._is_point_inside_mesh`.
- **Check:**
  - `asset validate` on the GLB: fit and dressing findings are `build.py`
    edits; frames never move;
  - `asset sheet <glb> --references` for game-renderer shots beside the
    photos;
  - an unprimed critique, up to three rounds.
- **Finish:** `asset bake`, `asset icons`, `asset check`, then
  `web/tests/sceneAssets`.

### What a unit lane touches

- `assets/source/skins/<family>/argon-r{1,2}/`
- `packages/scene-assets/blender/roster/<family>.py`, deleting its now-dead
  procedural code
- that family's `assets/source/roster/<family>/` GLBs and receipt
- its icon
- the spec's `assets/after/<family>/` and its slice notes

The coordinator owns `scripted_body.py`, `vehicle_export.py`, the lint
baseline, the bake and the runtime (`parallel.md`).

### Cost and time per unit

- **Argon:** the pilot's two rounds cost $5.50 (T-72B3) and $5.31 (Abrams).
  The second round is about a third of the first. Expect $5–6 a unit, or
  $6–8 where a round must also build a sibling's variant (P12). For the 35
  remaining units, that is roughly $200–250.
- **Wall-clock:** one to two hours of Bento session per unit for two rounds.
- **Local edits and integration:** two to four hours of agent time per unit.
  The pilot took longer, because it built the integration once.
  - Lint clean-up is the bulk: the T-72 needed 12 geometry edits, the
    Abrams 8.
  - A brief that carries the clearances and slot rules above should halve
    that.
