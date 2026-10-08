# 19 Closeout

Runs once, after every lane (slices 15–18) has landed: the full runs AGENTS.md
saves for a spec's end.

1. `bun run check` and `bun run verify` once; fix what they find.
2. Balance: roster numbers are untouched except the HMMWV's frame (slice 14),
   which was sampled there; run the balance report once.
3. Icons re-derived for every rebuilt appearance and every disabled card;
   `check` green.
4. No vehicle on an interim wreck (slice 11's seam ends here); every unit's
   tiers pass slice 12's reduction check and its class budgets; catalog-load
   bytes and texture layers within slice 10's limits.
5. Frame cost: the pilot's lab scene with a column of rebuilt vehicles and
   squads, against slice 09's baseline; within what slice 14 accepted.
6. The menu reel's exact-shot test passes (slice 08). One ordinary skirmish
   per faction pairing, and the menu reel: screenshots at each zoom where a
   tier changes, compare-screenshots against the before sheets,
   screenshot-critique unprimed, last; show the user with preview-shots.
7. `git grep -i village` lists only the generator's settlement class;
   `git grep` for the bare old generic ids lists only the weapon `rifle` and
   the roles `at`, `recon`. No family script redefines a wheel or a track; the
   legacy family helpers are gone.
8. Every page that draws models gets its GPU device through `renderer-core`'s
   device admission (which requests the adapter's texture-layer limit, slice
   10), not a bare `requestDevice()`: the `unit-roster` scene's model-check
   page did, and default limits (256 layers) break once Part B's textures grow.
9. Known scene failures to fix in the full run (found 2026-10-07):
   - `ground`: "the paused street: each side holds ground the other never
     saw" reports `onlyBlue: 0`. The `street` map's encounter now plays without
     the deleted scripted blue (slice 06), so blue may never advance far enough;
     re-stage blue in the `street` encounter or re-anchor the check.
   - `street-watch` (frame tick untuned for the self-playing encounter) and the
     22 `street` tours slice 06 didn't run.
   - Scenes need `bun run build:mechanics` and `assets/third-party` from LFS.
10. **Sources match their scripts.** Shared parts changed under families
    exported earlier (the VBL re-exports with 187 more tier-0 triangles after
    the tracked lane's `vehicle_parts.py` changes). Re-export every roster and
    disabled family, live and wreck, from the final shared code; then export
    each a second time and require byte-identical files (wreck noise was fixed
    in `wreckage.heat()`; the air lane's crash wrecks predate that fix). Bake,
    icons, check, commit.
11. Close the spec with close-spec.

## Delegated

Nothing: this slice runs checks and fixes what they find.
