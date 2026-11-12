# Family slice template: one Argon body per vehicle

Every family slice (17–29) follows this template. The slice file adds only
its models, the user's notes (quoted exactly) and the calls specific to its
family. The recipe itself is in [`argon-pilot.md`](argon-pilot.md), "The
recipe for every other tracked unit": brief, two Argon rounds, local edits,
integrate. This page holds what every family lane adds to it.

## Contract

- **Every model wears a scripted body:** `build.py` under
  `assets/source/skins/<family>/argon-r2/` (one folder per vehicle; a
  folder's `build(variant)` builds the variants of one vehicle), worn
  through `scripted_body.wear`. The family script keeps its frame, rig,
  whip antennas and wreck call, and nothing else of the body.
- **Unchanged:** frames, fixtures and the simulation (no battle digest
  moves); the GLB node contract; the shared modules (`scripted_body`,
  `vehicle_export`, `running_gear` and the rest, see `../parallel.md`).
- **Done for a model** is the README's bar: the critique (no score under 6,
  an average of at least 7) and every user note visibly fixed in a
  before/after beside the reference. At most three critique rounds, after
  which what is left is named in the slice file.

## Passes

1. **Brief round 1 for every vehicle in the lane at once.** One Bento
   session per vehicle (or per variant set, below), all started together,
   since they run remotely. Check each session's first turn
   `usageByModel` is `google/gemini-4-argon` (A5); if not, stop that
   session and report it, never let another model build.
2. **The brief** is the recipe's step 1, plus:
   - the user's notes for the vehicle, quoted, as must-fix items;
   - the variants it must build, if any, as `build(variant)` with one id
     per appearance;
   - **the skirt's front panel is flush with the front fender** (outer
     faces in one plane, top edges continuous): nothing of the skirt stands
     outboard of, above, or ahead of the fender, and appliqué stops where
     the glacis takes over. This was the user's "front side panels stick
     out" note on 11 vehicles, and the T-72B3's edit 14;
   - the turret's size and place measured from the frame survey
     (`assets/frames/<family>.json`), where a note is about the turret.
3. **Measure round 1** (hull box, turret extents, pivots, world muzzles,
   roof) and run the integration once (export, lint, `asset validate`).
   Round 2 takes the numbered fixes. No third round (A6).
4. **Local edits and integration** as in the recipe, one vehicle at a time.
   Number each edit in the folder README.

## Verification, per vehicle

1. Export live and `--wreck` through the asset CLI: zero lint issues for
   the vehicle's appearances (not just zero new ones). Re-export each script
   that imports this one and changed.
2. `asset validate <glb>` clean; tiers inside the caps (`tracked_heavy`
   120k/45k/13k/4k, `tracked_medium` 90k/34k/10k/3k).
3. `asset sheet <appearance> --references` (game renderer) for before,
   after and reference, saved as `assets/after/<family>/<vehicle>-compare.png`;
   one look at the wreck.
4. **Last check: an unprimed `screenshot-critique`** of the after shots,
   scoring silhouette, surface, fittings, materials and cohesion. Scores
   per round go in the slice file.
5. **One commit per vehicle**, on the lane branch. Never push.

## Out of scope in every family slice

- paint colours, schemes, camouflage, weathering, markings (P16);
- crew and open hatches;
- other families' scripts and the shared modules (file a shared-part request,
  `../parallel.md`);
- frames (fit is fixed in `build.py`, P9);
- deleting procedural code another family still imports (slice 30 sweeps the
  dead code once every family wears its body);
- raised launchers; the colour grade.

## Delegated to you

- the brief's wording, and which reference photos it carries;
- which round-2 fixes to ask for, and the local edits after it;
- which drawing to trust when sources disagree (record why).

A call you invent goes into `decisions.md` before you act on it.

## What stays green

- `cargo test -p sim --test sim catalog::`: frames are untouched.
- The file-size ratchet.
- Lint for every appearance this lane owns.
- Every re-exported dependant.
