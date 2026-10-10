# 06 — Test airframe art and rotor pipeline

**Status:** planned. **Depends on:** 01. **Owns:** L10 (pipeline), L13.

## Contract

A hovering test helicopter draws correctly at altitude:
- its rotors spin;
- its rotor disc doesn't fail hull fit or culling;
- it raises no track dust and its wheels don't spin;
- its shadow doesn't pop at the screen edge;
- a watched death doesn't crash the browser.

## API seam

- New `packages/scene-assets/blender/test_heli.py`, reusing `aircraft_parts.rotor`, writes `assets/source/test/heli.glb` and its wreck.
- `validate.ts`: hull fit leaves out `rotor_*` and `blade_*`.
- `pose.ts`: culling bounds sweep the rotor disc.
- `articulation.ts`: rotor spin as an accumulated angle, using the `turned()` pattern.
- `units.ts`: `vehicleClass` returns `air_<weight>`.
- `useBattleSession.ts`: the speed read is exhaustive over mobility, which fixes the crash.
- Gate track dust in `effectFrame.ts` and wheel spin in `poseDriver.ts` on air.
- `modelDetail.ts`: the shadow margin covers an airframe's offset shadow.

## What you can run or see

`/lab/air-hover`: the test helicopter hovering on the `air` map. Registered in the one lab and scene registry.

## Verification

Gates:
- `asset check`
- the scene `air-hover` without console or GPU errors

**Visual variable:** the airframe at height with spinning rotors
**Crop or mask:** the helicopter at 2× and its ground shadow
**Out of scope (later slices):** the drop line and ring (12), smoke (11), the fall (10)

1. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

Rotor rpm for the drawing; blade blur, if any.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

If the rotor reads wrong (too fast or strobing), that's a renderer-only fix here.
