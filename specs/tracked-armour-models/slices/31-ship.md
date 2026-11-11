# 31: ship (coordinator)

## Contract

The merged pass is baked and validated. Every picture and number that depends
on it is brought current, the full check ladder passes once, and the spec is
closed.

## Steps

1. **Bake.** On the merged tree: `bun run --cwd web asset -- bake`, then
   `asset icons`.
2. **Catalog-load tripwire.** Raise `CATALOG_LOAD_MAX_BYTES`
   (`packages/scene-assets/src/schema.ts:772-773`) to the measured load plus
   room (decision Q3). Then run `asset check` until it is green.
3. **Pictures.** Inspect each one for regressions, then bless (re-approving
   after checking is allowed):
   - `asset sheet`, under `lockf -k <main>/throwaway/gpu.lock`;
   - the unit-roster baselines: `UPDATE_BASELINES=unit-roster`
     (`web/scenes/unit-roster.mjs:111,182`);
   - the tutorial shots: `UPDATE_BASELINES=ui` (`web/tutorial-shots.mjs:43,53`);
   - the menu reel footage, re-recorded. Its tanks are the Abrams SEP v2 and
     T-72B3 (`fixtures/units/capture/units.json`). Never narrow a mechanic to
     keep the reel unchanged.
4. **Balance sample.** Run once, comparing against the named changes (the
   frame fixes and the Ajax's machine gun):
   - `cargo run -p mapgen --release --example battle_sweep -- --seeds 10 --seconds 30`;
   - `skirmish_tournament`.

   Record the result in `decisions.md`. A shift beyond what the named changes
   explain is reported to the user.
5. **Full ladder, once:** `bun run check`, then `bun run verify`.
6. **Skill lessons.** Add to `make-3d-models` only lasting lessons, as smells.
   Candidates:
   - "a barrel built from sections floats";
   - "skirts follow the hull outline";
   - "the lint compares parts only below the root".

   No per-task notes.
7. **Close the spec:** run `close-spec`.
8. **Clean up.** Remove every lane worktree and its build output.
9. **Push** to `staging`, then run `bun run promote:main`.

## Verification

As in the steps. The visual gates for this slice are step 3's inspection and
slice 30's cohesion critique.
