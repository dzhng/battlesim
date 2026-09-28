# C51: playable city encounter

**Depends on:** every required city slice, and C87 (the ground composition gate); optional ground slices (C68, C86, C79 density) are outside its completion set. **Kind:** slice.

## Question
Can friends play the city encounter at ≥30 FPS?

## Contract it unlocks
`fixtures/maps/<id>/encounter.json` (spawn, capture zone, variants, defender policy), run by the existing scenario builder (renamed if it now serves two maps). A menu entry, a `/city` route, and attribution on screen and in the credits.

## API seam
`crates/sim/src/village/` (the builder), `apps/`, credits.

## What the human can run or see
`/city` with replay and benchmark access.

## Verification
- The full 300 s `city-contact` benchmark ≥30 FPS.
- Full `city_report`.
- Whole-battle critique.
- Native/wasm replay parity.
- Friends play it.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**whole-frame coherence only (composition); a failed per-variable gate can't be hidden here**) against **the accepted per-slice shots**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** None.


## Delegated to the implementer
Deployments; objectives; camera target. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Every accepted slice gate.
