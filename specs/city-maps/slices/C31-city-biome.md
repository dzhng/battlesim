# C31: city biome

**Depends on:** C28. **Kind:** slice.

## Question
Does open ground in a city read as city (no farm plots, grass only where it grows), at city prop counts?

## Contract it unlocks
A `city` biome row (`fixtures/biomes/`): no farm plots (`plots.ts:165-169`); grass only on non-paved surfaces and parks; the grass build's per-clump loop over every prop footprint indexed (`grassPass.ts:243, 685-693`).

## API seam
`packages/battle-renderer/src/terrain/{biome.ts,plots.ts}`, `grassPass.ts`, `fixtures/biomes/`.

## What the human can run or see
A top-down block crop and a park corner.

## Verification
- Grass build ms at city prop counts.
- Village grass unchanged.
- Frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**unpaved-ground read only**) against **C28's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Pavement, curbs, markings.


## Delegated to the implementer
Park detection rule; grass density. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village biome.
