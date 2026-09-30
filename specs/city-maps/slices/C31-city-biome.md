# C31: urban and plain ground composition

**Depends on:** C28, C52, C83. **Kind:** slice.

## Question
Does one generated map read as town and open country without a city-wide biome suppressing its plains?

## Contract it unlocks
One spatial composition from `MapDefinition.surfaces` and `MapDefinition.land_regions` (C04): pavement and urban yards suppress farm plots; the reserved plain retains fields, meadow, grass and optional forest from the ground lane. Grass grows only on unpaved surfaces. The grass build's per-clump loop over props is indexed (`grassPass.ts:243, 685-693`). The urban/plain boundary is visible in the full frame but is never a second sim rule or a renderer-only guess about roads.

## API seam
`packages/battle-renderer/src/terrain/{biome.ts,plots.ts}`, `grassPass.ts`, `fixtures/biomes/`.

## What the human can run or see
A paired town-edge and open-plain crop from the same generated seed.

## Verification
- Grass build ms at generated town prop counts; open fields remain in the same map.
- Village grass unchanged.
- Frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the paired crops (**urban suppression and retained plain fields only**) against **C28's pavement shot and the accepted ground-lane field shots**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Pavement, curbs, markings.


## Delegated to the implementer
Urban yard and park appearance; grass density. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village biome.

## Feedback that would change this slice
Urban/plain transitions that confuse movement or scale reopen composition, with accepted ground variables held fixed.
