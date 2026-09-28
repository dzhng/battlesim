# C84: field palette

**Depends on:** C83, SG4. **Kind:** slice.

## Question
Do fields read desaturated (olive, tan, brown) while seen ground never reads as fog (Q-G13)?

## Contract it unlocks
The plot palettes in `summer.json` move to GG's palette, with rapeseed yellow, plus the biome L* floor test from SG4.

## API seam
`fixtures/biomes/summer.json` (`palettes`, `plots`).

## What the human can run or see
`field-250` and `patchwork-1100`.

## Verification
- **The `fog-look` darkest-seen check passes under every style** (L-G4).
- The L* floor test.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**plot albedo across the patchwork, excluding roads and forests**) against **battle-look's Broken Arrow farm frames, `../assets/reference/ground/ours-vs-refs-board.jpg` row 4, and C62's `patchwork-1100` baseline**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** In-field texture.


## Delegated to the implementer
Palette entries within the floor. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
`fog-look`.
