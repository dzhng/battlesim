# 13 Art rules: real tiers, dressing, unit budgets

**Unlocks:** the validator holds every unit to tiers that really switch with
zoom, dressing that can't pass for cover, and class budgets, so lanes can't
drift. Validator-only; no art changes here.

## Detail tiers that really switch (user, 2026-10-06)

Every unit switches level of detail with zoom. The renderer already picks a
tier by projected height (`models/modelDetail.ts`, `presentation.models.lod_px`);
what fails silently is the art: a mesh without a `_LOD<n>` suffix is copied
into every tier (`build.ts:203`), and `structure.tier_order` only requires each
tier to be no larger than the one before (`build.ts:224`).

**Contract.** For every unit and wreck appearance: each tier draws strictly
fewer triangles than the one before, by at least a recorded ratio per step
(set in slice 14 from the pilot), and an unsuffixed mesh is a finding instead
of being copied to every tier. Vehicles have no impostor (`modelLayer.ts:1591`), so tier 3 is what
the far camera draws: it keeps silhouette and colour. Soldiers keep their
impostor cards below `impostor_px`.

## Dressing outside the hull (decision 4)

`fit.hull_extents` holds tier 0 to the physical hull within ±0.1 m on every
face (`validate.ts:816-836`), so antennas, stowage, crew and mudflaps fail.

**Contract.** A node named `dressing_*` (node extras keep numbers only,
`build.ts:697-699`, so the name is the mark) is excluded from hull fit, as
gun-pitch and HMG-yaw parts already are (`OFF_HULL`, `validate.ts:632,820`),
and held instead to a dressing allowance per vehicle class, recorded in
choices.md: thin parts (antennas, whips) may rise above the hull; bulky
dressing (stowage, crew, tarps) may exceed it by at most a small margin, so it
can never read as cover the simulation lacks. Hull and turret bodies stay at
±0.1 m. This replaces the per-appearance `hull_top_m: 1.1` overrides on the
test tank and jeep (`assets/catalog.json:442,458`; the schema calls them the
allowance "for antennas, cupolas and pintle guns", `schema.ts:381-382`) and
the fix hint that invites widening (`validate.ts:834`): one owner for
dressing, no widened tolerance.

## Unit budgets

`budgetFindings` (`validate.ts:236`) checks scenery kinds only. Generalise it
to unit appearances keyed by slice 01's vehicle class and a soldier class:
triangles per tier, tier reduction ratio, bundle bytes, texture layers. The
rules land here empty; slice 14 fills the numbers from the pilot.

## Work

1. **Tests first** with synthetic GLBs, each red today: an unsuffixed mesh;
   a tier that doesn't reduce by the ratio; dressing over its allowance; hull
   over ±0.1 m; a unit over a budget rule.
2. Tier findings in `build.ts` (`:203`, `:224`); dressing exclusion and
   allowance in `validate.ts`; the `hull_top_m` overrides and the widening fix
   hint removed; `budgetFindings` generalised to units, keyed by slice 01's
   class (numbers empty until slice 14 fills them).
3. Run `asset validate` over every current unit and record which fail the new
   rules: that list is expected (today's art) and must reach zero by slice 19;
   the bake keeps working on findings it already tolerates, never by silencing
   the new ones.

## Verify

The tests; `scene-assets` tests. No screenshots: validator only.

## Delegated

Finding codes and messages.
