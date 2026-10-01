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


## Outcome — physical systems (2026-10-01)

Physical seats now come from exposed bays on the first three source floors,
with at most 32 seats and balanced directional allocation. Whole squads are
admitted against the remembered building definition and checked again when
entry completes, because replenishment can change squad size during entry.
Weapon assessment uses occupied carriers' window muzzles, matching launch.
Focused regressions cover physical placement, capacity, replenishment refusal
and actual rifle fire over an intervening shop. The old hypothetical closest
window firing origin is removed. Seat positions enter the battle digest.

Existing authored boxes lack floor/bay facts and retain a documented ground-floor
bridge. Source window fit, final tall-source authoring and the framed visual
moment remain specialist/map-lane work. The physical lane closeout and exact report scopes are recorded in
[the integration evidence](../assets/sim-lane-closeout/README.md).

Once source floors are present, an unresolved facade bay list supplies no seats:
only resolved physical bays can create fighting positions. The legacy lattice
is limited to descriptors without floor facts. A mixed resolved/unresolved
facade regression rejects invented windows while retaining real seats elsewhere.

Holdable replacement shells reuse source bay positions only below their observed
remaining height. Command admission reads the remembered shell; physical entry
checks the live shell. Original IDs retain the full remembered source plan, so
an unseen collapse cannot leak through a changed capacity. A two-metre damaged
shell regression first failed on inherited upper-floor seats, then passed with
only surviving ground bays. Holdable `into` states emit uniform part heights;
per-part full heights belong only to ungarrisonable terminal gutted states.
