# 02 Crews wear their vehicle's army

Byte-identical refactor now. The armies change later: the LAV in 07, the VBCI
and VBL in 06.

## Contract

A crewed vehicle's crew is chosen by the army its family names, not by its
faction. Today `vehicle_crew.CREW_SOLDIER` is keyed by faction
(`vehicle_crew.py:31-35`), `soldier_source` reads `assets/catalog.json`
(`:41`), and `vehicle_export.py:216-217` passes `variant["faction"]`.

## Seam

- `vehicle_export.run(..., crew_army="<army id>")`. A variant with crew and no
  `crew_army` is refused with a message naming the family.
- `infantry_roster.crew_source(army) -> str` gives the source path of that
  army's rifleman, look `a`: `rifle_squad` for the four existing armies. The
  new armies name their crew kit in 06.
- `CREW_SOLDIER` and `soldier_source` are deleted.
- Each crewed family names its army. Today's values, which keep the bytes:
  - `humvee.py`, `stryker.py`, `lav.py`: `us_army_ocp`;
  - `tigr.py`: `eastern_emr`;
  - `vbci.py`, `vbl.py`: `german_flecktarn`.

## What you can run

Re-export the 6 crewed families.

## Verification

- All 6 families re-export byte-identical to fresh exports from the
  pre-change scripts: Humvee, LAV ×2, Stryker ×4, Tigr, VBCI, VBL.
- A family without `crew_army` is refused: one small Python test beside
  `vehicle_export_test.py`.

There are no pictures, so no visual gate.

## Decisions delegated

The refusal message and where the army list is read from.

## Must stay green

Vehicle bytes, `vehicle_export_test.py`, `vehicle_lint_test.py`, `asset check`.

## Feedback that would change it

"Crews should stay faction-based" reverts the army values; the seam stays.
