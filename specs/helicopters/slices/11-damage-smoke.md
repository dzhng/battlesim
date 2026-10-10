# 11 — Damage smoke trail

**Status:** done. **Depends on:** 04, 06. **Owns:** D19, D34.

## Result

- **Contract:** `IdentifiedUnit.smoking` and `OwnUnit.smoking` (published as a last `smoking` field in the `identified` and `own` rows; web `IdentifiedView.smoking`, `OwnUnitView.smoking`). The rule is `Unit::smoking`: an aircraft below half its hull HP. The enemy's row carries the bit, never HP. A falling airframe always smokes. New data row `presentation.effects.damage_smoke` (a puff style plus `rate_hz`), and `EffectShooter.smoking`.
- **Tests:** `air::an_identified_enemy_helicopter_below_half_hp_publishes_smoking` (falsified at a 0.52 share: red), web `observation.test.ts` (the codec vector's enemy decodes `smoking`, no `hp`), `effectFrame.test.ts` (a smoking aircraft trails smoke behind it, a sound one none; a hovering one still smokes).
- **Parity:** six publication records re-recorded again; only their publication hashes moved, no digest.
- **Scene:** `/lab/air-crash` checks that the helicopter publishes that it smokes and draws smoke; all checks pass. Evidence: `throwaway/evidence/air-crash/frame-smoke-1280x800.png`, `crop-smoke-trail-2x.png` (airframe and 30 m behind), and the fall sheet, now with the trail.
- **compare-screenshots:** no before shot of a smoking airframe exists (D19 is new); single-image judgement. The trail starts thin at the airframe and widens behind it; in the visible area it is dark grey-brown.
- **screenshot-critique (unprimed, run twice, the second last):**
  - Fixed after the first pass: the trail read as a string of tan beads (8 Hz, pale albedo). It is now 20 Hz, darker and wider at its end.
  - Recorded, not fixed: once blue has no unit left to see with, the frame takes the fog look (grey, hatched) while effects and the airframe keep their colour, so the trail reads ochre against it. That is the fog style, the same for the wreck's smoke. The trail is a smooth ribbon that rises and spreads little (minimal by the slice's brief); the hit point's autocannon puff hangs in the air (an existing effect); the wreck reads undamaged (slice 15's art).

## Contract

A helicopter below 50% HP trails smoke. You see it on your own helicopters and on identified enemy ones. Rotor wash is out of scope.

## API seam

- A coarse `smoking` bit in the identified-unit publication, and smoking from HP for your own units.
- A smoke emitter that follows a moving source. Today's smoke comes only from wreck props (`effectFeed.ts`).

## What you can run or see

The `air` scene with a damaged helicopter.

## Verification

Tests: `an_identified_enemy_helicopter_below_half_hp_publishes_smoking`, and a web decode test.

**Visual variable:** the smoke trail
**Crop or mask:** a 2× crop around the airframe and 30 m behind it
**Out of scope (later slices):** the fall (10)

1. Judge the candidate against its target with [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md): telemetry plus a less-wrong verdict. With no target, use its single-image diagnostics.
2. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

Smoke density and colour, kept minimal.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

If the user wants smoke on all damaged vehicles, this generalises to every hull.
