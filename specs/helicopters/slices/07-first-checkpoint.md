# 07 — First browser checkpoint: air-aware weapons and the `air` scene

**Status:** planned. **Depends on:** 03, 06. **Owns:** D1, D2, D22, D29, L3, L12 (record).

## Contract

**First browser checkpoint:**
- `test_heli` flies a route over the village at 20 m, pops over roofs and goes around the tower;
- a rifle squad fires at it: sparks, no damage;
- `test_gun_jeep` (the IFV stand-in, D40) damages it;
- a `test_tank` beside them never engages it.

## API seam

- The `targets: [ground | low_air]` column (D43) on every root weapon row in `fixtures/game.json`, inherited through `extends` (D29).
- One predicate, `weapons::reaches(def, layer)`, read inside `can_damage`, `effective`, `compatible` and `preferred_kind`. `fires_regardless` still applies, so plinking survives (D2).
- Selection measures range in 3D.
- A unit's layer comes from its type's mobility.
- A stray round still hits physically: no change to flight.

## What you can run or see

`/lab/air` with the scene `web/scenes/air.mjs`. Fixture id `air`, on the slice 03 map.

## Verification

Tests:
- `a_tank_gun_never_selects_a_helicopter`
- `an_atgm_never_selects_a_helicopter`
- `a_rifle_squad_keeps_plinking_a_helicopter_it_cannot_hurt` (pins D2)
- `an_autocannon_damages_a_helicopter`
- `a_stray_tank_round_still_hits_a_helicopter`

The scene also asserts heights from the observation, not only from shots. Record L12 (the camera passing through the helicopter) if it shows; don't fix it here.

**Visual variable:** the flight profile over the village
**Crop or mask:** the helicopter's path from the village to the tower, plus 2× crops at a roof pop and at the tower detour
**Out of scope (later slices):** contact sign (13), drop line (12), smoke (11), the fall (10)

1. Human checkpoint (**non-blocking**): open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and leave about 5 minutes for a reply, carrying on with independent work meanwhile. If the user is silent, decide on the evidence, record the decision and why in this slice and in [choices](../choices.md), close the opened shots and go on.
2. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

The scene's camera framing and timeline.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

Anything about how the flight reads: too high, too low, too floaty. That changes the D25 numbers, and needs the user.
