# 09 Per-unit wrecks, real detail tiers, and dressing

**Unlocks:** the art contracts every rebuilt family (slices 12–15, 17) is
held to.

## Per-unit wrecks (user, 2026-10-06)

Every vehicle has its own wreck model: you can tell which unit died by looking
at it, and it is the unit's own size.

Today a dead vehicle becomes a `light_`/`medium_`/`heavy_wreck` prop sized to
its hull (`crates/sim/src/battle.rs:1962-1990`), and the renderer draws
whichever of three wreck appearances has the nearest footprint
(`packages/battle-renderer/src/models/propAppearance.ts:175-187`): 31 roster
vehicles, wheeled ones included, die into the generic tank's wreck.

**Contract.**
- A vehicle appearance declares its wreck in `assets/catalog.json`: a wreck
  appearance whose states are `default` and, for a vehicle with a turret,
  `hull` and `turret` pieces (`scenery.ts` `WRECK_PIECES`), exported by the
  family's own script from its own hull and turret.
- The wreck prop the simulation leaves records the unit type it was
  (a field on the wreck body; a contract change, so the engine id and
  digests move, named in choices.md). The renderer draws that type's wreck,
  at the type's size; footprint matching for wrecks is deleted.
- A cook-off throws the turret only if the wreck has a turret piece, so a
  wheeled APC without a turret never throws one.
- The validator refuses a vehicle appearance without a wreck, and a wreck
  whose footprint isn't its unit's hull. **Transitional seam:** until a
  family is rebuilt, its wreck is a re-export of its current hull in burnt
  materials (slices 13–14 replace each with the real one); the seam ends when
  slice 16 finds no family on it.

## Detail tiers that really switch (user, 2026-10-06)

Every unit switches level of detail with zoom. The renderer already picks a
tier by projected height (`models/modelDetail.ts`, `presentation.models.lod_px`);
what fails silently is the art: a mesh without a `_LOD<n>` suffix is copied
into every tier (`build.ts:203`), and `structure.tier_order` only requires each
tier to be no larger than the one before (`build.ts:224`).

**Contract.** For every unit and wreck appearance: each tier draws strictly
fewer triangles than the one before, by at least a recorded ratio per step
(set in slice 12 from the pilot), and a tier-0-only mesh copied to every tier
is refused. Vehicles have no impostor (`modelLayer.ts:1591`), so tier 3 is what
the far camera draws: it keeps silhouette and colour. Soldiers keep their
impostor cards below `impostor_px`.

## Dressing outside the hull (decision 4)

`fit.hull_extents` holds tier 0 to the physical hull within ±0.1 m on every
face (`validate.ts:816-836`), so antennas, stowage, crew and mudflaps fail.

**Contract.** A node named `dressing_*` (or with `extras.dressing`) is
excluded from hull fit and held instead to a dressing allowance per vehicle
class, recorded in choices.md: thin parts (antennas, whips) may rise above
the hull; bulky dressing (stowage, crew, tarps) may exceed the hull by at
most a small margin, so it can never read as cover the simulation lacks. Hull
and turret bodies stay at ±0.1 m. No tolerance is widened.

## Work

1. Tests first, each red today: a vehicle appearance without a wreck; a tier
   copied to every tier; dressing over its allowance; a cook-off throwing a
   turret from a wreck with no turret piece.
2. Wreck field on the simulation's wreck prop, carried through observation
   to the renderer; footprint matching removed.
3. Interim wrecks for every runtime vehicle and the test units (whose current
   wrecks become theirs).
4. Validator rules for tiers and dressing.

## Verify

The tests; `sim` wreck and cook-off tests; `web/tests/cookOffs.test.ts`;
one cook-off lab scene with a tracked and a wheeled vehicle (screenshot,
screenshot-critique unprimed, last).
