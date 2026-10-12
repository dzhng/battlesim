# Slice 02 — Physical roster admission and runtime assets

## Contract unlocked

All 12 planned artillery cards become ordinary physical catalog entries. Each has complete body/hull, mobility, sensors, mounts, weapons, capabilities, sound, appearance and wreck data; each has a declared artillery role; each can be placed by the skirmish/catalog path. No row remains a `planned` unit with a disabled reason.

## API and data ownership

- Authored unit rows: `fixtures/units/roster/us.json`, `europe.json`, `eastern.json`.
- Shared role metadata: `fixtures/units/roles.json`.
- Shared/variant weapon rows and flight profiles: `fixtures/game.json`.
- Physical wreck rows: `fixtures/props/generic/wrecks.json` and any family-owned rows required by the existing wreck contract.
- Runtime appearance registration and model fit: `assets/catalog.json`, scene-assets loaders/exporters and the normal `web/asset.mjs` workflow.
- Resolution and admission: `crates/contract/src/catalog.rs`; derived browser view: `fixtures/catalog.json`.

Use the existing variant/extends conventions. Do not duplicate faction tuning where a shared physical variant can carry multiple memberships. Keep `category: "sup"`; add `roles: ["artillery"]` and any other role only when the physical unit actually owns that capability.

## Data contract

- Every indirect artillery weapon sets `trajectory: "indirect"`, targets ground as appropriate, has a large per-weapon `min_range_m`, and remains within the accepted long-range/flight limits from slice 01.
- `scatter_mrad` remains the sole angular accuracy input; no distance-specific scatter constant is introduced.
- Existing `stationary` and deployment capability fields describe each profile. There is no universal artillery setup rule.
- Mount pivots, muzzles, wreck chains and asymmetric model orientation must pass the same fit checks as other physical units.
- Remove artillery IDs from intentionally disabled model metadata only where the existing manifest contract requires it; reference/source receipts remain truthful.

## Tests and artifact

Write/extend catalog tests before the data conversion:

- all 12 IDs resolve as physical units, not planned cards;
- every unit has a valid role, mount and weapon reference;
- each artillery weapon is indirect, ground-capable and admitted by flight constraints;
- hull/mobility/appearance/wreck and sound checks pass;
- faction card membership and costs remain the authored roster identity.

Run the catalog/model/appearance checks and regenerate `fixtures/catalog.json` through the repository-owned blessing flow. Produce an ignored roster admission report listing each ID, physical type index, weapons, range/min-range, appearance and wreck identity.

## Verification

```bash
CARGO_TARGET_DIR=throwaway/target cargo test -p sim --test sim catalog::
BLESS_CATALOG=1 CARGO_TARGET_DIR=throwaway/target cargo test -p sim --test sim catalog::the_browsers_catalog_view_is_current
bun run --cwd web asset -- --list
```

Use the exact subcommand printed by `web/asset.mjs` if the bare `asset` command exposes a different name. Run the affected scene-assets/model checks after runtime assets are materialized.

## Firewalls and delegated choices

Do not change AI procurement, command UI, map generation, or ballistic algorithms here. Internal physical dimensions, costs, cadence and family inheritance are delegated only within the constraints above; record any balance choice in the feature choices ledger. Missing art may use the repository's accepted placeholder/bake contract temporarily, but the slice cannot claim complete admission until the replacement asset passes the same runtime fit checks.

## Acceptance and feedback

The slice is accepted when the normal skirmish setup can field each of the 12 IDs and the derived catalog/assets agree. If a source model cannot be registered or fitted, stop at that unit and record the exact missing input rather than borrowing another unit's appearance silently.

## Pass evidence

All 12 IDs resolve as physical catalog units and the focused catalog suite is
green. Each newly fetched source and wreck validates individually. The full
`asset check` remains blocked by unrelated repository-wide LFS pointers in
existing runtime bundles and source libraries; no baseline asset was replaced
to hide that environmental failure.
