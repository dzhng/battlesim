# 06 — Test airframe art and rotor pipeline

**Status:** done ([evidence](#evidence)). **Depends on:** 01. **Owns:** L10 (pipeline), L13.

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

## Evidence

- `asset check` passes (662 runtime files, `test_heli` and `test_heli_wreck` baked, the `test_heli` icon written).
- The `air-hover` scene passes with no page or GPU errors: the helicopter observed at cruise height (20 m over the ground), its rotor crop changing from one tick to the next while the jeep's does not move, and the ground darker where its shadow falls than beside it.
- Tests: rotor turn, rotor disc in culling bounds and hull fit without the rotors (`sceneAssets/articulation`, `sceneAssets/validate`); `air_light` and `topSpeedKmh` (`vehicleClass`); rotor spin and no rolling (`poseDriver`); no dust (`effectFrame`); an off-screen airframe kept by its shadow, and its lift (`modelDetail`). Sim: `air::` now runs on the fixture unit; `catalog::every_unit_type_sets_up_fires_each_mount_and_moves` fires `test_heli`'s HMG and skips its move until slice 03.
- "A watched death doesn't crash the browser" is proven at its cause only: the cook-off speed read is `topSpeedKmh`, exhaustive over mobility. No scene kills a helicopter yet; the fall is slice 10's.
- Unprimed screenshot critique (last check, `throwaway/evidence/air-hover/`). It confirmed that the model is complete, the rotors turn between frames, the shadow follows the rotor angle and the sun, and depth ordering is right. Its actionable findings:
  - **Height reads ambiguous** (high): at a glance the airframe overlaps the road beside the jeep and could be parked. The shadow, about 24 m off along a 40° sun, is the only cue to its height. This is slice 12's drop line and ring; nothing to do here.
  - **Rotors are hard-edged blades with no blur** (high): they turn, but may read as a slow propeller. Blade blur was delegated and is not done. If it reads wrong in play, that's a renderer-only fix (a blur disc or ghost blades); revisit it with real art in slice 15.
  - **Shadow edges are stair-stepped on the thin blades** (medium): this is cascade resolution, shared by every caster. No change.
  - **A bright green dot on the fuselage** (low): it is the right navigation light, as authored.
  - **The fuselage looks flat against the field** (low): test art, no change.
