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

Implemented the startup lane's reach-local fog owner. Horizon rays read nearby buildings from the existing spatial grid and conservative angular sectors, retaining the original analytic intersections and source-order ties. Changes invalidate only eyes that can reach the changed old or new geometry; equivalent row reordering preserves maps. Whole-building surface sampling likewise follows reachable building/eye pairs, including inward roof probes, and clears flags when an eye leaves. No simulation or publication contract changed.

Focused CPU proof: 13 combined fog tests, web typecheck and touched-file lint/format checks pass. The frame-owner regression tests pin immediate local invalidation, movement-queue progress and probe buffer lifetime; dense independent slab rays pin angular coverage around turned boxes and the west wrap. Independent code review found no remaining issue.

On the frozen 16,000-box/48-eye synthetic separated-region workload, actual compute-pass spans fell **37.831 →1.182 ms**, passing the **≤2 ms** local budget. The grid-only intermediate was 2.611 ms and failed; adding angular candidates reduced deterministic ray/box visits to 2,310,144 versus 3,145,728,000 globally. Frozen village cost showed no regression (1.346 →0.999 ms). GPU buffers grew by 303,828 bytes; texture bytes stayed fixed. Protocol, allocations, raw evidence pointers and limits are in [frame-cost.md](../frame-cost.md). This is a count/region stress case, not full dense-Metro or G0 admission; S4's selected full-extent verdict remains absent.

Final GPU proof: all fog scene checks pass. Shape oracle maximum error remains 1.669e−7; synthetic lookup has zero mismatches across 3,917 compared vectors. Street/garrison agreement remains 99.401%/99.354%, and both measured building-corner edges retain 0.5 px steps with identical position errors. The paired village/city whole-building flag hashes and 72/384 probe vectors match exactly; removing every eye clears all flags.

Visual comparison against the frozen pre-change village: the near frame, mask and building-corner crop are byte-identical. All corner boundary extents, wall/gate/post silhouettes, illumination and fog fade are preserved. Small full-frame deltas occur at distant forest boundaries and unit/effect pixels, so this is a corner-fidelity claim rather than whole-frame identity. The final unprimed critique distinguishes hatched fog from cast shadows and finds no broad halo, wall spill, missing structure or layering break. It flags existing bright seams/steps, tiny dark infantry and thin roof-trim ambiguity; the unchanged trim remains a C22+ art concern, explicitly outside this shot. Complete capture metrics and critique are retained in scratch `throwaway/fog-comparison/`. The matched corner/near set was open in Preview from 05:56:57 to 06:02 UTC on 2026-10-02. With no feedback in that non-blocking checkpoint, the unchanged corner look was accepted within the stated limits and Preview was closed.
