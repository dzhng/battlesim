# 12 — Drop line, ground ring and ghosts

**Status:** planned. **Depends on:** 03, 06. **Owns:** D18, L14.

## Contract

You can read where the helicopter is over the ground: a thin drop line from the airframe to a ring on the ground. Placement and destination ghosts show it at cruise altitude, not landed.

## API seam

- Designed first through [game-ui](../../../.agents/skills/game-ui/SKILL.md), keeping the D11 sign's family from slice 02.
- The drop line is an overlay segment (`MeshBuilder.segment`), not a painted mark.
- The ring stays ground paint (`orderOverlay.ts`).
- `unitGhosts.ts` lifts air ghosts to cruise height.

## What you can run or see

The `air` scene.

## Verification

The scene runs without console or GPU errors.

**Visual variable:** reading where the helicopter is over the ground
**Crop or mask:** 2× crop of airframe, line and ring, over flat ground and over a roof
**Out of scope (later slices):** the contact sign (13)

1. Human checkpoint (**non-blocking**): open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and leave about 5 minutes for a reply, carrying on with independent work meanwhile. If the user is silent, decide on the evidence, record the decision and why in this slice and in [choices](../choices.md), close the opened shots and go on.
2. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

Line width and opacity within the game-ui rules.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

Any 'too busy' feedback here changes line opacity, not the rule.
