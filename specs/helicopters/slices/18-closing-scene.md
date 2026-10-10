# 18 — Closing scene (D14) and closeout

**Status:** the closing scene is built and verified narrowly; the full-run closeout below is still to do. **Depends on:** every slice above. **Owns:** D14, D40, L12 (fix only if shown).

## Contract

D14: you buy an Apache in a skirmish. It flies in, pops over a village and kills a tank. An IFV shoots it down, and the wreck flattens trees.

## API seam

- A deterministic encounter `closing` on the `air` map with `test_attack_heli`. The red tank and IFV are bought through purchase commands (D40).
- A recorded skirmish replay at a pinned `/battle?...` seed, with the real Apache.

## What you can run or see

The scene `closing`, and the recorded replay.

## Verification

Run once, now (open: the orchestrator's closeout):
- [ ] the full `check`;
- [ ] `verify`;
- [ ] the balance report (record the shift; don't retune).

Fix L12 only if a shot shows it.

**Visual variable:** the whole battle beat
**Crop or mask:** full frames at fly-in, the tank kill, the shoot-down and the wreck at rest
**Out of scope (later slices):** none: this is integration

1. Judge the candidate against its target with [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md): telemetry plus a less-wrong verdict. With no target, use its single-image diagnostics.
2. Human checkpoint (**non-blocking**): open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and leave about 5 minutes for a reply, carrying on with independent work meanwhile. If the user is silent, decide on the evidence, record the decision and why in this slice and in [choices](../choices.md), close the opened shots and go on.
3. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

The seed choice.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

The user's verdict on the D14 replay.

## Record (2026-10-10)

**The scene.** `/lab/air-closing` (fixture `air-closing`, encounter `closing` on the `air` map, seed 6, the test catalog set). Choices are in [choices](../choices.md).
- **Units:** blue's AH-64E starts at the west edge (10, 470) at full health. Red's T-72B3 stands at (300, 470), behind the village's 7-floor block as seen from the west, on return fire only. Red's BMP-2M Berezhok stands at (575, 456), on the far side of the wood, where the 61 m tower hides it from the whole approach.
- **Scripts:** at tick 1 the Apache moves to (205, 472), over the block's roof. At tick 300 it is set to return fire only and moves on to (455, 465), over the wood.
- **Map:** the map is unchanged. The existing wood catches the wreck, so the other `air` scenes stand on the same ground.

**The beat, natively and in the browser (identical ticks):**
1. The Apache climbs over the roofs (20 → 32.3 m) and blue first identifies the tank at tick 152, from 32.3 m.
2. Its rockets and chin gun kill the tank at tick 208.
3. Flying on past the tower, it is seen by the BMP at tick 410. The BMP's autocannon bursts down it at tick 496, at (434, 476, 22).
4. It falls on about 60 m into the wood. Its wreck rests at (496.3, 452.4), on the ground, inside the wood polygon, with no standing trunk within 5 m. Trees stood there before, and three are felled.
5. The battle replays to the same digest.

**Rule fix found by the scene:** a side that saw an airframe go down now also learns the trees its crash felled, as it already learned its wreck (`Battle::land`). Before the fix, once the Apache died blue had no eyes on the wood, so blue's view drew its wreck under standing crowns, and D14's "the wreck flattens trees" was invisible to the buyer. The test is `air_crash::the_side_that_saw_it_go_down_sees_the_trees_it_felled`. Only crash landings that fell toppling props can move a digest. `menu_reel`, `skirmish`, `publication`, `battle_authority` and `destruction` stay green.

**Tests:**
- Native `air_closing` (4 tests): the fly-in and pop-up; the tank dies before the Apache, which the IFV downs; the wreck in the wood with its trees felled; the replay digest. A mutation (the goal moved to (480, 465)) put the wreck 5 m past the wood, and the wood test failed as it should.
- `air_crash` (7 tests) passes, and so does `maps` (7 tests).
- `every_saved_encounter_makes_a_battle_on_its_map` had been failing since slice 15 on the `apache` encounter: it ran test maps on the test units alone. It now runs them on the test set, the one their labs run.
- Clippy on `sim` is clean. Web typecheck, oxlint and oxfmt are clean.

**Browser:** `air-closing` passes all 12 checks, and `air-crash` passes again after the rule change. `air` and `air-hover` were not rerun, because neither the map nor anything they draw changed. The scene's evidence is in `throwaway/evidence/air-closing/`:
- full frames: `frame-fly-in`, `frame-tank-kill`, `frame-ifv-fires`, `frame-shoot-down`, `frame-falling` and `frame-wreck`;
- `crop-wreck-2x`;
- the contact sheet `beat-sheet.png`.

**compare-screenshots** (single-image diagnostics, no target):
- The four daylight frames are rich: entropy 4.6–5.2 bits, contrast 111–133, edges 0.25–0.39.
- The wreck frame and its crop sit at the low-contrast line (contrast 61, entropy 3.7). That is blue's fog after it has lost its only unit, not an empty frame.
- **Verdict:** each frame shows its moment.

**screenshot-critique**, two unprimed rounds:
- **Round 1, acted on:** the hit was a few pixels at the horizon, and the wreck could not be found under the crowns. The hit and the fall are now framed on the airframe, and the wreck from nearly overhead. Round 1 also showed that the Apache's starting `condition.hp: 90` made it trail smoke from the first frame. It now starts at full health, and the BMP's burst still downs it over the wood.
- **Round 2 (last), recorded, not this slice's to fix:**
  - The climax (hit, fall, wreck) is under fog hatching, and fogged crowns read as grey smoke. Blue's only unit is dead, so this is honest (fog style is not a bug). **Open for the user:** a blue ground observer in the encounter would keep the climax in colour.
  - The burning tank's hull and debris take a fog-like tint under a lit fireball (medium-high, seen twice). This is worth a renderer look.
  - The drop line ends on a roof, so the Apache looks perched on the block (slice 12's open question).
  - The hit is weak, and the falling airframe looks whole and in control (slice 10's look).
  - The wreck's fire reads as a row of separate sprites, and its smoke is olive.
  - A pale ghost box hangs in fog left of the tower (a remembered building).
  - The dark wedge in the tank-kill frame is the tower's shadow, not a defect.
- L12 (camera through a helicopter) does not show in any shot, so it was not fixed.

**Human checkpoint:** the shots were not opened. This ran as a delegated worker that cannot hear a reply, so the orchestrator should show them to the user (`beat-sheet.png` first).

**The recorded skirmish replay:** not found, and the deterministic lab is the D14 proof. A throwaway probe (`throwaway/closing/skirmish-probe.mjs`) played `/battle?type=mixed&size=small&seed=N&faction=us`: it bought the AH-64E (accepted at any destination, since it flies in from the edge), readied, and attack-moved it to the farthest objective.
- Seeds 1 and 2, 8 minutes each: the Apache killed a Tigr and was never shot down. The AI's opening is a ground screen of light vehicles and infantry, and its tanks and IFVs come later in its rotation.
- Seed 3 timed out in preparation (240 s).

A natural D14 needs four things in one battle: the AI fields a tank and an IFV, both cross the Apache's path, the IFV wins, and the wreck lands in trees. Finding that would take a long seed search for a moment the lab already pins exactly.
