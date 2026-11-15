# 09 Ship: every export, crews, budget, closeout, one merge

## Contract

Every shipped soldier and crew wears a generated body. The catalogs, the
runtime, the icons, the review baselines and the load budget all agree with
that. The branch merges to `staging` once.

## Steps

1. **Export everything:**
   - every `KITS` export (about 126 GLBs plus the new armies' looks);
   - the 6 crewed families, which by now name their armies;
   - the 12 test soldiers and the test jeep.

   Run parallel single-threaded Blender processes, with a progress line per
   export. Measure one export's time first and size the parallelism from it.
2. **Receipts:** every kit's receipt (`infantry_equipment.py <kit> --receipt`),
   naming its skins.
3. **Budget and runtime.** Run `asset bake`. If the load refuses, set
   `CATALOG_LOAD_MAX_BYTES` (`packages/scene-assets/src/schema.ts`) to the
   measured load rounded up to the next 32 MiB, with a comment naming why.
   Retired budget is whatever slice 08's deleted recipes freed. Then run
   `asset icons` and `asset check`.
4. **Review baselines:** re-accept `assets/review/{rifle-reference,
   infantry-equipment, at_carried*}` and the scene baselines that moved, only
   after inspecting each for regressions. The user's rule: bless baselines
   only after checking them.
5. **The full repository closeout run, once** (development.md, check ladder
   step 3).
6. **Cost report:** `skin-generate costs infantry` into this spec's README:
   per body, per army, the total (rejected attempts included), and per
   shipped soldier GLB.
7. **Merge to `staging` once:** merge `origin/staging` in first and re-bake if
   needed. Then [close-spec](../../../.agents/skills/close-spec/SKILL.md)
   moves this spec to `specs/done/` as a rationale record.

## What you can see

- **Every army's units:** each infantry kind at
  `/labs/showcase?unit=<kind>&faction=<f>`, and the Armory yard (`&yard=1`).
- **Crews:** every crewed vehicle with its army's crew.
- **The roster:** the menu backdrop live, and refreshed unit icons.

## Verification

- `asset validate` has no findings on every soldier and crewed vehicle, and
  every export repeats byte for byte (sample a dozen across armies and modes).
- `asset check` passes. The pinned tests pass: `pose_behaviour` and native
  `models`.
- The full closeout run passes.
- **compare-screenshots:** the whole roster before against after, a lineup
  per army plus the showcase shots.
- **screenshot-critique** as the last check, on the complete capture set:
  every army lineup, the team showcases, the crews and the test soldiers.
- **Non-blocking checkpoint:** open the shots with preview-shots, wait about
  5 minutes, decide, record and close.

## Decisions delegated

The parallelism, the budget figure (within the rule above), and which
baselines need re-accepting.

## Must stay green

Everything: this is the closeout.

## Feedback that would change it

A late rejection of one body re-exports only that body's kits, their
receipts and their crews.
