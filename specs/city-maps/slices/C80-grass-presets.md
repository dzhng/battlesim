# C80: grass presets

**Depends on:** GG. **Kind:** slice.

## Question
Can every grass and crop preset guarantee its final drawn height ≤0.9 m, and carry shape, colour and wind response (Q-G10)?

## Contract it unlocks
- `GrassSpec` gains blade shape, width, colour ramp, clumping and a wind-response factor (it scales the shared wind; no new clock).
- **A validator for effective field height ≤0.9 m that includes every biome scale and variation multiplier.** Today's wheat (0.95 m, `assets/catalog.json:664-667`) makes it fail first, then it's fixed.
- Blades stay ≤`GRASS_MAX_BLADES`. Measure the padding cost (every clump is padded to 8 blades).

## API seam
`packages/scene-assets/src/grass.ts`, the grass schema, the validator.

## What the human can run or see
A workbench grass sheet with a 0.9 m ruler.

## Verification
- Validator red, then green.
- Adversarial multiplier test.
- Generator determinism.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**blade and clump shape at uniform tint**) against **`../assets/reference/ground/grass-a.jpg`, `grass-b.jpg` and today's sheet**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Colour, fields.


## Delegated to the implementer
Packing; neutral defaults. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
One indirect draw per tier; capacity bounds.

## Feedback that would change this slice
Grass that implies cover above its physical cap changes effective-height presets before any downstream species tuning.


## Outcome — physical systems (2026-10-01)

The systems-only effective-height guard composes actual vertex height across
all LODs, biome scale, clump variation and patch variation with f32 arithmetic.
The renderer and tile bound use the same shared variation constants. Width must
lie horizontally (apart from bake roundoff), so widening cannot evade the cap.
Three falsified regressions cover combined multipliers, tilted strips and a
higher far LOD. All nine grass checks pass.

Existing wheat and verge field scales were reduced to meet the 0.9 m contract;
source meshes, wind and provenance are unchanged. This does not certify grass
look or the other C80 presets. Matched production village frames (1280×800, DPR 1, tick 90, fixed camera)
show 59,492 changed pixels after lowering the verge scale; mean RGB channel
difference is 0.384. Full shots and 3× grass crops are under
`throwaway/sim-lane/grass-{before,after}*.png`. No actor or height ruler is in
that framing, so it proves the field change reaches production rather than
measuring world height from pixels. The visual fallback disposition is recorded below; integrated gate scopes are recorded in [the closeout evidence](../assets/sim-lane-closeout/README.md).
The mathematical bound is pinned by the adversarial validator checks.

Visual checkpoint: before/after full frames and 3× detail crops were opened in
one Preview window for over five minutes, then closed without a user response.
All three delegated agents were occupied, so the screenshot-critique fallback
was used: the strongest visible objection is that the full frame reads as a
fine crosshatched ground texture and neither the frame nor the crop supplies a
world-height reference; those images cannot prove a 0.9 m bound or acceptable
blade silhouette. The left-hand dark gradient is also ambiguous as a lighting
or fog cue without an actor/shadow reference. Those appearance judgements remain
open with the specialist. The scoped validator decision stands on actual vertex
bounds, the shared renderer multipliers, adversarial failures and production
pixel-difference evidence. A later fresh critique may add appearance findings;
this pass does not accept the broader grass look.

Fresh critique follow-up: once a slot became available, an unprimed agent
inspected all four full/detail images. It saw no perceptible improvement between
the matched frames despite the measured pixel difference, described pixelated
flecks/tangled fine strands and patterned diagonal repetition, and found the
left dark region ambiguous between shadow and fog. Neither view provides a
world-height reference. These baseline appearance findings remain in the visual
pass; no grass-look acceptance is claimed. The effective-height guard and lower
scales remain justified by the physical bound and actual production execution.
