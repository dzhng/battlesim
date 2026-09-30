# C40: floor band seats

**Depends on:** C04, C53 (C13 for the window overlay). **Kind:** slice.

## Question
Where can soldiers occupy a building (Q3)?

## Contract it unlocks
- `garrison::building_seats(building, rules) → SeatPlan`:
  - bands = min(floors, 3), at the building's floor heights;
  - one seat per C00 physical facade bay per band, on exposed edges only (~3 m policy retained);
  - capacity capped at **32** (gameplay cap);
  - whole-squad admission.
- Shelter stays today's flat `buildings.cover_strength`.
- Muzzles are at seats.
- **Named village digest change:** capacity 16 → min(bays × bands, 32).

## API seam
`sim::garrison` (`:83-100`), `world::props::facade_slots` (`props.rs:112`), `BuildingRules`.

## What the human can run or see
A seat overlay on C13's reusable template facade (seats at windows), and a GIF of an MG on floor 3 firing over a 2-storey shop.

## Verification
- Run tweak-mechanics first.
- Native tests: floor 3 sees over a 2-storey building; no seat above band 3; no seat on a non-exposed edge; cap holds.
- `village_report -- --quick --compare main`.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**seat markers versus window positions only**) against **C13's reusable template facade**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Eyes (C41), art quality.


## Delegated to the implementer
Deterministic rounding and cap allocation (record in `choices.md`). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Flat shelter; muzzles at seats.

## Feedback that would change this slice
Seats that miss accepted facade bays reopen descriptor/seat fit; new bands or capacity rules need a named rule decision.
