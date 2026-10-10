# 11 — Damage smoke trail

**Status:** planned. **Depends on:** 04, 06. **Owns:** D19, D34.

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
