# 20 US prototype armour

**Unlocks:** the US roster has the prototype armour the Eastern roster has
(T-14, T-15, Type 15) and Europe has (KF51 Panther): the M1E3 Abrams and the
M10 Booker, as recognisable models, unbound and undeployable like the
Eastern prototypes.

User, 2026-10-07: "We did include T14/T15, so we need prototype US." Europe's
equivalent already exists (the KF51 Panther prototype, playable; Leopard 2A8;
Challenger 3 disabled), so nothing is added for Europe.

## Contract

- Two roster cards in `fixtures/units/roster/us.json`, shaped like the T-14's:
  `us_m1e3_abrams` (category `veh`) and `us_m10_booker` (category `veh`),
  factions `us`, each with a `disabled_reason` matching the Eastern
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
   Booker from its own references (tracked, 105 mm). US desert tan; real
   tiers; own wrecks.

## Verify

The disabled gate (`crates/sim/tests/catalog.rs`), `icons.test.ts`,
`purchasePicker.test.tsx`; `asset validate` on each GLB; a sheet beside its
references; compare-screenshots against the references; screenshot-critique
last; preview-shots non-blocking.

## Delegated

Card cost and stats within the roster's conventions (recorded); which
generated views, if any.
