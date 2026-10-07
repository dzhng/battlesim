# 11 Every vehicle its own wreck

**Unlocks:** you can tell which unit died by looking at its wreck, and the
wreck is that unit's size. Every rebuilt family (slices 14–18) exports its
wreck under this contract.

## Per-unit wrecks (user, 2026-10-06)

Every vehicle has its own wreck model: you can tell which unit died by looking
at it, and it is the unit's own size.

Today a dead vehicle becomes the wreck prop its hull names (`Hull.wreck`, a
`light_`/`medium_`/`heavy_wreck` physics row: cover tier, integrity; sized to
the hull, `crates/sim/src/battle.rs:1962-1990`), and the renderer draws
whichever of three wreck appearances has the nearest footprint
(`packages/battle-renderer/src/models/propAppearance.ts:175-187`): 31 roster
vehicles, wheeled ones included, die into the generic tank's wreck.

**Contract.**
- **Physics stays where it is:** `Hull.wreck` keeps naming the physics prop
  row. The published prop gains `wreck_of: UnitKind` (on `KnownProp` and its
  publication; a contract change, so the engine id and digests move, named in
  choices.md). A wreck placed by a map rather than left by a death (the
  endurance scene, `endurance.rs:137`; test props) names its unit too: no
  wreck without a unit, no default.
- **Look:** a vehicle appearance names its wreck appearance in
  `assets/catalog.json` (`wreck: "<appearance>"`), a scenery appearance of
  kind `wreck` whose states are `default` and, for a turreted vehicle, `hull`
  and `turret` pieces (`scenery.ts` `WRECK_PIECES`, already validated to sit
  inside the whole), exported by the family's own script from its own hull
  and turret. The renderer draws `wreck_of`'s wreck at that unit's size;
  nearest-footprint matching for wrecks (`propAppearance.ts:175-187`) is
  deleted. (Rejected: wreck as a state of the vehicle appearance itself; it
  would change unit bundle roles, while scenery wrecks with pieces already
  exist and validate.)
- A cook-off throws the turret only if the wreck has a turret piece, so a
  wheeled APC without a turret never throws one.
- The validator refuses a vehicle appearance without a wreck, and a wreck
  whose footprint isn't its unit's hull. **Transitional seam:** until a
  family is rebuilt, its wreck is a re-export of its current hull in burnt
  materials (slices 14–18 replace each with the real one); the seam ends when
  slice 19 finds no family on it.

## Work

1. **Tests first,** each red today: a killed vehicle publishes `wreck_of`
   (Rust); an Abrams wreck resolves to the Abrams wreck appearance (TypeScript);
   a vehicle appearance without a wreck is a finding; a cook-off never throws a
   turret from a wreck without a turret piece; a map-placed wreck without a
   unit is refused.
2. `wreck_of` on the published prop, carried through observation to the
   renderer; nearest-footprint matching for wrecks deleted.
3. Interim wrecks for every runtime vehicle (the seam above) and the test units
   (whose current wrecks become theirs).

## Verify

The tests; `sim` wreck and cook-off tests; `web/tests/cookOffs.test.ts` and
`propAppearance.test.ts`; one cook-off lab scene with a tracked and a wheeled
vehicle: screenshot, compare-screenshots against its before shot,
screenshot-critique unprimed, last. Digests move (named in choices.md).

## Delegated

Interim wreck materials.
