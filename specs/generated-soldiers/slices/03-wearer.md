# 03 The wearer handles every soldier; a lying-fit gate for every corpse

No paid calls. Everything here is proven on the pilot's body.

## Contract

A pinned body can be worn by **every kit and mode**: rifle, LMG, marksman,
heavy sniper, a shoulder launcher active and carried, a tripod ATGM active and
carried, and a MANPADS active and carried. It should also survive the next
bodies' variety: a tall ruck or whip, any skin tone, darker prints.

And **every soldier's corpse lies low**, not just the one the native test
checks.

## Seam

Each item names its change.

1. **`fit.lies`, a new validator finding.** In `packages/scene-assets/src/validate.ts`,
   using `lyingBounds` (`pose.ts:173`, present but unused): every soldier's
   finest tier at each of its `corpse_poses` lies below
   `soldier_lying_height_m` and above −0.2 m. That is a new 0.6 m entry in the
   catalog's validator tolerances, beside `soldier_height_m`: the type is at
   `schema.ts:531`, and `validate.ts:334` merges it. Write the test first, beside the validator's existing tests, from a
   lying and a standing fixture.
2. **One back measure.** `Kit.back_y()` gives the rearmost point of vertices
   whose dominant weight is `spine_03`, across every part. It replaces all
   three copies, each of which finds no parts on a skinned body and crashes:
   `infantry_equipment.py:491` (tripod pack), `:599` (slung launcher) and
   `infantry_kit.py:867` (the test soldier's `carried_launcher`). The roster's
   slung launcher and the test soldier's `carried_launcher` collapse into one
   placement if the code allows; that call is delegated.
3. **Height fit from a head band.** `generated_body._fit` scales height to the
   top of the head and helmet, measured in a central band (|x| within the
   shoulders), not the mesh's highest vertex (`generated_body.py:148`). A whip
   antenna or a ruck above the helmet must not shrink the soldier.
4. **A skin split for any skin tone.** `_skin_like` (`generated_body.py:323`)
   needs a bright red channel, so dark skin fails it and takes the side tint.
   The split becomes a rule on hue and saturation within the head-bone faces,
   independent of brightness. It is a texel and bone rule, never a per-army
   case.
5. **The regrade grades cloth only.** Skin keeps its own colour; today the
   gain hits the whole atlas (`generated_body.py:353-360`). Add an outlier
   clamp that pulls near-white texels on cloth faces toward the cloth mean,
   which fixes the pilot's flecks. Check the overall value against the
   procedural soldiers': the pilot read a little bright.
6. **The crew's hose.** Inspect it at the crew tier (the soldier's tier 1,
   decimated). If the hydration hose still stands off the vest, add a rule in
   `_decimate` or the tiers that keeps thin loops attached; that method is
   delegated.

## What you can run

- `node web/asset.mjs sheet` for the pilot and for the launcher kits worn on
  the pilot body:
  - `fgm_148_javelin_team` active and carried `a`;
  - `us_atgm_team_bgm_71_tow_2a` active and carried `a`;
  - `fim_92_stinger_team` active and carried `a`.
- These exports are working evidence. Slice 09 makes the shipped ones.

## Verification

- `asset validate` has no findings, `fit.lies` included, on:
  - the pilot's three kits;
  - the six launcher exports;
  - every procedural soldier GLB currently in the catalog.

  If a procedural soldier fails `fit.lies`, fix it in its own commit, or name
  it in the slice notes if it dies with slice 08.
- Every export repeats byte for byte.
- The pinned tests pass: `cargo test -p presentation --test pose_behaviour` and
  `cargo test -p native-client --test models`.
- The pilot's clip strips: no new tearing, and the launchers seated on the
  back with straps reaching it, kneeling controls in reach, tripod feet on the
  ground.
- **compare-screenshots:** the pilot's contact sheet before this slice
  against after it. Its value and colour should move toward the procedural
  soldiers', with no white flecks, and its shape should stay the same.
- **screenshot-critique** as the last check, on the full sheet set: contacts,
  strips, red side, launcher close-ups, the skin crop.
- **Non-blocking checkpoint:** open the shots with preview-shots, wait about
  5 minutes, decide, record and close.

## Decisions delegated

How the head band is measured, the hue and saturation thresholds, the
clamp's strength, the strap standoffs on the slung tube, whether the two
carried-launcher placements merge, and the hose fix.

## Must stay green

The `generated_body` contract, the stand-aim fit (`fit.soldier_height` and the
rest), the pilot's look apart from the intended grade change, and `asset check`.

## Feedback that would change it

"Too dark", "too bright" or "skin is off" retunes the grade, never the
geometry.
