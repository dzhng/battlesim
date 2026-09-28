# C87: ground composition gate

**Depends on:** every retained ground slice. **Kind:** slice.

## Question
Do the accepted ground contracts compose on real routes at ≥30 FPS?

## Contract it unlocks
An evidence gate, not a module. A tour of the village, `/lab/river`, the farmland lab and a C31 city park edge.

## API seam
The frame and map loader.

## What the human can run or see
The tour's shots and benchmark.

## Verification
- Whole-frame comparison is allowed here, because every component variable has already passed.
- The full `village_report` once.
- Serial frame benchmarks; native/wasm replay; memory and reset.
- **The lane budget: at most +3 ms GPU p50 on the village benchmark** (trees ≤1.5, dressing ≤1, grass ≤0.5).
- A failure returns to the owning slice; don't fix composition by changing several variables at once.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**whole-frame composition**) against **the accepted per-slice shots and `../assets/reference/ground/ours-vs-refs-board.jpg`**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** None.


## Delegated to the implementer
None. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Every accepted slice.
