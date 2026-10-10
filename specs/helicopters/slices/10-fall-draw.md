# 10 — Drawing the fall

**Status:** planned. **Depends on:** 04, 06. **Owns:** L2 (renderer).

## Contract

The browser draws a downed helicopter's spinning, falling arc from the published `crashes` group, then hands over to the wreck. It no longer vanishes in mid-air, snaps to the ground or teleports.

## API seam

- The feed (`packages/battle-renderer/src/scene.ts`) carries falling airframes.
- `apps/battle-lab/src/cookOffs.ts` skips airborne deaths; its 6 m reach and 2 s memory are shorter than a fall.
- The impact fireball goes at the impact point.

## What you can run or see

The `air` scene, extended with a shoot-down.

## Verification

The scene runs without console or GPU errors. `compare-screenshots` against the cook-off's before shot of the same death.

**Visual variable:** the fall and impact
**Crop or mask:** a contact sheet of the arc, from hit to wreck at rest
**Out of scope (later slices):** smoke trail (11), wreck art (15)

1. Judge the candidate against its target with [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md): telemetry plus a less-wrong verdict. With no target, use its single-image diagnostics.
2. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

Fireball timing and size at impact.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

If the fall reads as too long or too short, the arc's drag in slice 04 changes.
