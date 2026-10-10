# 13 — The airborne contact sign, built

**Status:** planned. **Depends on:** 02, 05, 12. **Owns:** D11 (built), D33.

## Contract

A lost enemy helicopter leaves the sign picked in slice 02, floating at its last height.

## API seam

`contactGlyph.ts` draws air contacts (layer `low_air`, more than the low-hover height above ground) as the chosen sign at their z. Ground contacts are unchanged.

## What you can run or see

The `air` scene: the helicopter goes behind the tower and out of sight.

## Verification

The scene runs without console or GPU errors.

**Visual variable:** the airborne contact sign
**Crop or mask:** 3× crop of the sign over fog and over a roof
**Out of scope (later slices):** none

1. Judge the candidate against its target with [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md): telemetry plus a less-wrong verdict. With no target, use its single-image diagnostics.
2. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

Nothing beyond the slice 02 pick.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

A different pick from the user restarts this slice from slice 02's sheet.
