# Family slice template (slices 17–29)

Every family slice follows this procedure. A family slice file holds only its
models, the user's notes for them, and the decisions specific to that family.
Read this template, `../parallel.md` and the
[`make-3d-models`](../../../.agents/skills/make-3d-models/SKILL.md) skill
before you start.

## Contract

Each model in the family reads as its real vehicle at the zoomed battle camera
and in cinematics:
- it meets the critique bar;
- every lineup note for it visibly reads as fixed beside its reference;
- it adopts every shared helper from slices 08–13;
- its frame comes from slice 15, unchanged by the art.

## Inputs (already decided; don't re-decide)

- The frame survey for the family: `assets/frames/<family>.json`. It classifies
  each of the user's notes as one of:
  - **frame**: already fixed in slice 15;
  - **art**: yours to fix;
  - **dressing**: inside the allowance.
- The shared helpers:
  - `vehicle_guns.gun_tube`;
  - `armour_parts.side_skirt` and `track_fender`;
  - `armour_parts.PLATE_BEVEL`;
  - `armour_parts.panel_seam`, or slice 13's result;
  - `running_gear` sag (automatic);
  - closed `roof_parts.hatch`/`cupola`.
- Slice 03's verdict for cast turrets.
- `chip=1.0`.

## Pass S: silhouette and proportions

- **The variable:** the outline in side, front and top views against the
  drawing.
- **Step 1, overlay.** Run `drawing_overlay.py` in vehicle mode for side, front
  and top, against the survey's source (step 0 of the skill).
- **Step 2, measure each note.** Turn each "art" note into a measured statement:
  station, current value, target value. Record it in the slice file.
- **Step 3, fix in the family script, within the frame.** If the file would grow
  past the 32,000-byte ratchet, split it by concept first. `armata.py` is the
  pattern: a library module that the variant scripts import.
- **Step 4, check hatches.** If anything on the roof moved, re-check hatch
  clearance against the turret ring (`specs/done/off-centre-turrets.md`).
- **The crop:** the overlay per view, plus the before/after/reference closeups
  that show each note.
- **The verdict:**
  - each note visibly reads as fixed, shown with `compare-screenshots` of after
    against `assets/before/` and the reference;
  - any overlay disagreement still left is named.

## Pass F: finish

- **The variable:** fittings, plate edges and seams at the battle-zoom and
  cinematic shots.
- **Adoption checklist.** This part is mechanical; tick it in the slice file.
  - [ ] The main gun is one `gun_tube` call. No sleeve sections without a core.
  - [ ] Skirts and fenders go through `side_skirt`/`track_fender`. The family's
        own `skirt_top`, `skirt_foot` or `follow_tiles` is deleted.
  - [ ] Paint-role plates have a bevel of at least `PLATE_BEVEL`.
  - [ ] Seams are placed with the slice 13 helper, on the plates the references
        show seams on. Sparing: what reads at battle zoom.
  - [ ] No `head_out`. Every hatch is closed.
  - [ ] `run(..., chip=1.0)`.
- **Fittings.** Add or fix fittings only where the critique says they don't
  read at battle zoom.
- **The verdict:** the critique bar. Record the scores in the slice file,
  per model per round.

## Verification

1. **Full-family export,** live and `--wreck`, through the asset CLI. This
   writes the receipt. Re-export any script that imports this one.
2. **Lint (decision C11):** for this family's appearances, clear every issue of
   the visible kinds that slice 01 categorised:
   - turret fittings into hull fittings;
   - skirts, bins or lights into the belt or wheels;
   - wheels overlapping wheels;
   - unbraced or floating parts.

   Seating that never shows may stay in the baseline: running gear into the
   hull sides, the turret into the roof, and hull fittings under the turret
   overhang. Check with `vehicle_lint.py -- --variant=<id>` for each appearance.
   Never update the baseline; slice 30 does that.
3. **Fit:** `bun run --cwd web asset -- validate <glb> --type <unit type>` is
   clean. Run `bun run build:wasm` in this worktree first.
4. **Budget:** the receipt's `triangles_by_tier` sits inside the `unitArt.ts`
   caps:
   - tracked_heavy: 120k/45k/13k/4k;
   - tracked_medium: 90k/34k/10k/3k.
5. **Wreck:** one closeup of the wreck. Wrecks are never linted, so look at it.
6. **Closeups** with `assets/shots.txt`, shaded and matcap, saved to
   `assets/after/<family>/`.
7. **`compare-screenshots`** of after against before and the reference, for
   each note.
8. **Last check: an unprimed `screenshot-critique`** of the after closeups.
   - It scores silhouette, surface, fittings, materials and cohesion (with the
     rest of the lineup) out of 10, and lists at most five fixes.
   - The bar is no score below 6 and an average of at least 7.
   - Allow at most three rounds per model. After that, name what is still below
     the bar in the slice file and in `decisions.md`.
9. **Checkpoint,** which never blocks: open before/after/reference with
   `preview-shots`. Wait about 5 minutes. If there's no reply, decide, record,
   close the shots, and commit.
10. **One commit per family.** Push only at the coordinator's milestones.

## Out of scope in every family slice

- paint colours, schemes, camouflage and weathering;
- crew;
- other families' scripts;
- shared helpers (file a shared-part request instead, see `../parallel.md`);
- frames (file a frame amendment, decision S9);
- raised launchers;
- the colour grade.

## Delegated to you

- how the family script is internally structured, and its part names, except
  the node names in the GLB contract (`track_L/R`, `wheel_*`, mount nodes);
- which fittings the critique asks for;
- the exact seam placement;
- which drawing to trust when sources disagree (record why).

Every other call is in this template, your slice file or `decisions.md`. A call
you have to invent goes into `decisions.md` before you act on it.

## What stays green

- `cargo test -p sim --test sim catalog::`: frames are untouched.
- The file-size ratchet.
- Lint for every appearance this lane owns.
- Every re-exported dependant.
