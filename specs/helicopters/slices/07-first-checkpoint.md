# 07 — First browser checkpoint: air-aware weapons and the `air` scene

**Status:** done: the rules and the scene ([evidence](#evidence)); the human checkpoint is open. **Depends on:** 03, 06. **Owns:** D1, D2, D22, D29, L3, L12 (record).

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

## Evidence

- **Contract.** `/lab/air` (fixture `air`) plays the `air` test map's new `flight` encounter; scene `web/scenes/air.mjs`. The map grew to 640 × 520 m: a village of six building templates (two of them, a 3-floor shop and a 7-floor point block, on the route and tall enough to lift it), one `china-tower-20f` (61 m) across the straight line, and a 70 m wood. `air-hover` still passes on it.
- **The scene passes** (no page errors, no GPU warnings), asserting from blue's observation, sampled every tick:
  - 20 m over open ground before its order and hovering at its goal; never above ground + 40 m (highest 32.3 m);
  - over each roof whose top + 10 m passes cruise, at least 10 m over that roof and above cruise (16 samples, lowest 10.0 m over the 7-floor roof);
  - its track never inside the tower's footprint, passing beside it, and never turning back (it fell back 0 m);
  - 150 rifle rounds strike its hull in the rifle phase while its health stays 200;
  - the gun jeep, set to fire at tick 540 and held at 590, hits it once (200 → 100), and it still hovers 10 s later;
  - the tank, seen throughout (870 samples), never fires its main gun (`weaponPoses[0].shots` stays 0).
- **Found and fixed by the scene** ([choices](../choices.md)): at cruise the helicopter skimmed the tower-corner waypoint a metre or two wide, flew 130 m on and turned back for it. A waypoint now counts as passed once the aircraft is beyond it along the leg that led there, and a corner is taken no faster than the aircraft can stop in the rest of the route. Test: `air_flight::flies_on_past_a_tower_without_turning_back`; all 39 `air*` tests pass. This is aircraft only, so no ground digest can move; `menu_reel` fields no aircraft and was not rerun.
- **Shots** (`throwaway/evidence/air/`): the route side on and from above, each with a strobe of the helicopter every 20 ticks; the roof pop and tower detour with 2× crops; a rifle strike, the jeep firing and the jeep's hit, with crops; `sheet-flight-profile.png`. Single-image metrics flag none as empty, flat or transparent.
- **L12** (camera through the helicopter) did not show in any shot.
- **Unprimed screenshot critique** (last check, run three times as the framing changed). The final pass's actionable findings:
  - **The tower detour reads as a near miss** (high). The helicopter's centre passes about 12 m from the tower's north wall, a little more than its 7 m half length, so from above its rotor seems to graze the roof outline. The clearance is slice 03's (walls widened by the hull's half length). If it reads wrong in play, it is flight feedback for the user, and this slice leaves it as it is.
  - **Height is hard to read in the wide views** (high). The side strobe shows the climb over the block only as a 25 px rise at 420 m, and no wide shot has a height cue under the airframe. That is slice 12's drop line and ring. The roof-pop shot, taken from under the roof edge, shows the gap.
  - **A pale see-through copy on the tower** (medium). This is the game's x-ray of an own unit hidden behind a building, by design.
  - **Sparks hang in the air behind the airframe** (medium). Seen in both the earlier pass and the final one. Rifle sparks stay where the round struck while the helicopter flies on at 61 m/s. A renderer effects question (whether sparks should inherit the target's velocity); no change here.
  - **Weak or odd effects** (medium). The rifle tracer reads as a hairline laser from fog, the spark is small, the jeep's muzzle bloom is large and lights the wheat, and the autocannon hit shows a flash but no tracer. These belong to the effects owner, unchanged here.
  - **Strobe artifacts** (medium). Tracers pasted in with the copies, and a smear where the copies overlap at the slow start. Scratch composite only.
  - Earlier passes flagged the roof-pop height, the missing helicopter in the end frame and the fog discs in the plan strobe. These were fixed by reframing: the eye under the roof edge, the frame widened to the goal, and each copy taken against the next frame.
- **Human checkpoint:** open. The shots are ready for [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This pass did not open Preview.
