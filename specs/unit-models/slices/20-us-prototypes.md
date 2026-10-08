# 20 Prototype armour and a light tank per faction

**Unlocks:** the US roster has the prototype armour the Eastern roster has
(T-14, T-15, Type 15) and Europe has (KF51 Panther): the M1E3 Abrams and the
M10 Booker, as recognisable models, unbound and undeployable like the
Eastern prototypes.

User, 2026-10-07: "We did include T14/T15, so we need prototype US", then
"let's make sure all 3 factions has a light tank". Europe's prototype MBT
already exists (the KF51 Panther, playable). Light tanks: Eastern has the
Type 15 (disabled card); the US gets the M10 Booker here; Europe gets the
**CV90120** (BAE Hägglunds' tracked CV90 hull with a 120 mm gun), the closest
match to the Type 15 and M10 and a relative of the roster's CV90 family.
(Rejected: Centauro II, a fielded wheeled 120 mm tank destroyer, not a light
tank; swap if the user prefers it.)

## Contract

- Two roster cards in `fixtures/units/roster/us.json`, shaped like the T-14's:
  `us_m1e3_abrams` and `us_m10_booker` in `us.json`, factions `us`, and
  `europe_cv90120_light_tank` in `europe.json`, factions `europe` (all
  category `veh`), each with a `disabled_reason` matching the Eastern
  prototypes' ("Ground profile and model admission pending"). Card stats
  follow the roster's existing conventions for their class (`tweak-mechanics`
  for anything new); the resolved catalog is re-blessed; no playable card
  changes.
- Entries in `fixtures/units/model-manifest.json` with `model_status:
  reference_built`, own source GLBs under `assets/source/roster/disabled/`,
  frames through `disabled_variant` from dimensions stated from their
  references (no archived frame exists).
- Each its own wreck and its own icon on its unavailable card.

## Work

1. References under `assets/references/m1e3/` and `assets/references/m10/`
   (licensable only; the M1E3 is a prototype, so photos are scarce: generated
   views labelled per the schema where needed, never trusted over a photo).
2. Cards and manifest entries; the disabled gate and icon tests pass.
3. Models through `vehicle_parts.py`/`vehicle_export.py` (`run_disabled`):
   M1E3 seeded from the pilot Abrams with its references' differences; M10
   Booker from its own references (tracked, 105 mm), US desert tan; CV90120
   from the CV90 lane's hull with its 120 mm turret, Swedish scheme. Real
   tiers; own wrecks. References also under `assets/references/cv90120/`.

## Verify

The disabled gate (`crates/sim/tests/catalog.rs`), `icons.test.ts`,
`purchasePicker.test.tsx`; `asset validate` on each GLB; a sheet beside its
references; compare-screenshots against the references; screenshot-critique
last; preview-shots non-blocking.

## Delegated

Card cost and stats within the roster's conventions (recorded); which
generated views, if any.
