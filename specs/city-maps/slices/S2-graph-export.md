# S2: graph export

**Depends on:** none (start here). **Kind:** slice.

## Question
Under our pinned Blender 5.2.1, can all three vendored graphs yield reusable (module, transform) placement rows, and at what size, time and module count?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write the verdict to `specs/city-maps/spikes/S2.md` (numbers table + one verdict row per question + kill check).

## API seam
A disposable `evaluate(PartInput) → ModuleInventory + PlacementRows` driver over the three `.blend` files. They go in the scratch worktree only, fetched with `git lfs`-style pulls of just those files. Inputs are set through `m.properties.inputs.<socket>.value` (L6).

## What the human can run or see
A contact sheet per archetype × part size, rendered in Blender from our 30, 80 and 250 m camera (`fixtures/village.json` `presentation.camera`), with street outputs off.

## Verification
- Sweep NYC, China and Paris over parts of 6×15, 8×25, 20×30 and 40×60 m, floors 2/4/6/8, 3 seeds, and the graph's LOD input.
- Measure:
  - eval seconds per part, cold and warm;
  - instances per part;
  - unique modules by mesh-content hash, and triangles per module (closes O-2);
  - placement bytes per part raw, gzip and run-encoded (module, origin, step, count over bays and floors);
  - two-run byte identity.
- Confirm:
  - rows can be read **before realize** (NYC's instance output; China before `CN_Finalize`);
  - which inputs switch street and sidewalk outputs off (L5);
  - whether a floor-height input exists or must be patched in (Q3);
  - the MIT attribution line to keep.
- Re-run China's embedded scripts in a clean Blender and diff the result against the shipped file (Q-B′). This is informational, not a kill.
- **Kill → fallback:**
  - no pre-realize tap → dedupe realized per-part meshes by a quantised part signature (width and depth snapped to bay multiples, floors);
  - >10 s per part → bake unique signatures only;
  - a graph refuses arbitrary sizes → snap parts to its legal range;
  - no floor-height input → a patch requirement for C13.

## Delegated to the implementer
The exporter access points; dedupe implementation. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges. Their two photo atlases never leave scratch.

## Feedback that would change this slice
If a graph can't be patched for floor heights, the user may accept snapped floor heights.
