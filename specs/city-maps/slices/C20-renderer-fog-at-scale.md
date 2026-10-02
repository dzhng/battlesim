# C20: renderer fog at scale

**Depends on:** C01. **Kind:** slice.

## Question
Does renderer fog cost scale with the occluders in an eye's reach, not with every known occluder?

## Contract it unlocks
The technique S4 picked, inside `frame/fogVisibility.ts` (merge at `:223`). Occluder changes invalidate only the affected eyes (`setOccluders`, `:620`).

## API seam
`packages/battle-renderer/src/frame/fogVisibility.ts`.

## What the human can run or see
`/lab/city-scale` with fog on.

## Verification
- Oracle agreement vectors unchanged.
- Fog scene ≥95%.
- Paired `FOG_COST=1` at G0's occluder count: fog build ≤2 ms.
- The village cost unchanged within noise.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**fog edges at building corners only**) against **the pre-change village fog frame**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Building art (C22+).


## Delegated to the implementer
Bucket or index representation. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village fog look; spike 02's edge sharpness.

## Feedback that would change this slice
Shadow mistaken for unexplored space reopens fog rendering with physical visibility and light fixed.

## Outcome

Implementation pass: the horizon merge reads each rebuilding eye's nearby occluder indices from the existing whole-structure grid, then conservatively narrows each ray to an angular sector. The analytical intersection and packed horizon arithmetic stay unchanged, and candidates retain source order, including equal-slope ties. An occluder change invalidates only maps whose built eye can reach the changed old or new geometry; those maps rebuild immediately, preserving the previous timing. Geometry-equivalent row reordering preserves every map. The whole-surface pass likewise dispatches only reachable structures and tests only their nearby eyes, preserving the original bounding-circle prefilter (including inward roof probes). Clearing its prior flags before writing the reachable rows removes stale seen flags when an eye leaves a neighbourhood.

Narrow CPU proof: the fog seam and new spatial/scheduling tests pass (13 tests combined), including garrison/source-knowledge contracts. The frame-owner test falsified blanket invalidation and both independent-review findings (a stale queued move and a destroyed pre-frame probe buffer), then passed after fixes. Web typecheck and touched-file lint pass.

A deterministic synthetic census with 16,000 boxes across approximately 10 km, 48 separated eyes and 600 m reach gives 8,988 eye–occluder pairs instead of 768,000 global pairs (185–188 candidates per eye). At 4,096 rays, the grid-only merge visits 36,814,848 candidate rows; the angular-sector refinement visits 2,310,144, instead of 3,145,728,000 global rows. This proves bounded work, **not** the GPU time budget. Scratch evidence and reproducer are in `throwaway/fog-candidates.json` and `throwaway/fogCandidateProbe.test.ts`.

**Measured intermediate result:** the grid-only arm matched village and synthetic-city whole flags and probe vectors, and passed the fog scene with unchanged edge/agreement metrics. Correct compute-pass timestamp spans measured 37.830839 → 2.610918 ms on the synthetic city, missing the 2 ms gate; the angular-sector refinement is queued for cost and scene checks.

**Still required before accepting C20:** final-candidate GPU checks are queued under the shared GPU lock. GPU oracle agreement, the matched building-corner captures/critique/Preview checkpoint, the paired city build ≤2 ms and village cost comparison remain unverified. The S4 verdict named by this slice does not exist in this checkout; the chosen exact per-eye grid cull and the gap are recorded in the C20 choices. No simulation rules or observation vectors changed.
