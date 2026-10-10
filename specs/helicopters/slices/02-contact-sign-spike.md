# 02 — Design spike: the airborne lost-contact sign

**Status:** planned. **Depends on:** nothing (runs alongside 01). **Owns:** D11 (look).

## Contract

The user picks the look of the red, perfectly circular sign that floats in the air where an enemy helicopter was last seen. The spike decides the look only; slice 13 builds it.

## API seam

None in code. The output is a written choice in this slice: shape, colour, halo or outline, size at map zoom and close zoom, and how it relates to the existing ground contact glyph (`packages/battle-renderer/src/contactGlyph.ts`, `presentation.contacts` in `fixtures/game.json`).

## What you can run or see

A sheet of 3–5 variants made with [design-with-images](../../../.agents/skills/design-with-images/SKILL.md), judged with [game-ui](../../../.agents/skills/game-ui/SKILL.md). Each variant is shown over ground fog, over a village and beside a metro tower, at map zoom and close zoom. Show the D18 drop line faintly as context, so the sign and the line read as one family. Evidence goes in `specs/helicopters/assets/contact-sign/`.

## Verification

The sheet must show every variant on the same three backgrounds and at both zooms.

**Visual variable:** legibility of the floating sign
**Crop or mask:** the sign plus 3× its size around it, at map and close zoom
**Out of scope (later slices):** the drop line's final design (12), the ground glyph, fog styling

1. Human checkpoint (**non-blocking**): open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and leave about 5 minutes for a reply, carrying on with independent work meanwhile. If the user is silent, decide on the evidence, record the decision and why in this slice and in [choices](../choices.md), close the opened shots and go on.
2. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

How many variants and how the sheet is laid out.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

The user's pick. Any 'it should look like X in WARNO' reference becomes a reference image here.
