# New York and Paris building families

**Status:** in progress. Picks up the city-maps deferral ([closed spec](../done/city-maps/README.md#scope-and-release-decision), [choice](../done/city-maps/choices.md)): "New York and Paris remain later families through the same source/export contract; another shipping family needs real category coverage and preset selection rather than an art fallback."

## Goal

A generated map is built in one regional family (M08, [the parcel pass](../../crates/mapgen/src/parcels/mod.rs)). Today the only family is `china`. This adds two more, `new_york` and `paris`, each with its own art in **every** building category a district or the countryside asks for, so any seed can draw any of the three and still fill every type and size.

## Contracts

Nothing new crosses a boundary; the families travel through contracts that already exist.

- **Template descriptor** ([contract](../../crates/contract/src/templates.rs)): `regional_family` is `new_york` or `paris`; ids are prefixed `nyc-` or `paris-`. Physical rules (bay lattice, floor bands, garrison bays, collapse ≤ 6 floors, gutting above) are unchanged.
- **Source sets** ([city readme](../../packages/scene-assets/blender/city/README.md)): new sets under `assets/source/city/<set>/`, listed in `assets/catalog.json` `city_sets` with `catalogue: "generated"`. A set holds one family's templates.
- **Physical catalogue** (`fixtures/prototype-building-templates.json`, written by `asset catalogue`): grows by the new templates; its hash moves, and everything that pins it moves in the same commit (saved maps, parity records).
- **Presets** (`fixtures/map-presets.json`): `parcels.regional_families` becomes `["china", "new_york", "paris"]`. A map's family stays a draw of its own seeded stream, so the share address `/battle?type=&size=&seed=` still names one battlefield. No menu choice is added.
- **China is unchanged.** Every existing China set rebuilds byte-identical from its script; a shared-helper refactor that moves a China byte is a regression.

## Coverage required of each family

Each family covers the six categories at roughly China's footprint and floor range, including its **smallest** footprints (a lot is cut to a template, so a family without small templates fails narrow frontage):

| Category | China's range (footprint m, floors) | New York character | Paris character |
| --- | --- | --- | --- |
| `detached_home` | 8×11 to 12×9, 1–2f, one L | clapboard/shingle frame houses, porches, gables | stone/render pavillons, slate or tile hips, shutters |
| `attached_home` | 7×10 single to 5-unit rows, 2–3f, shopfronts | brownstone/brick rowhouses, stoops, cornices; shops under flats | maisons de ville, render with shutters; shops under flats |
| `urban_apartment` | 35–59 m slabs, 20×20 point, U and court, 4–8f (≤ 6f needed for Open) | pre-war brick walk-ups and corner buildings: fire escapes, cornices, water tanks (vendored NYC graph) | Haussmann blocks: ashlar, iron balconies, zinc mansards (vendored French graph) |
| `farmstead` | 3 compounds, 1–2f | American farm: frame farmhouse, gambrel barn | French corps de ferme / longère in stone and tile |
| `highrise` | 10–20f, slab and towers | brick and limestone pre-war towers, setbacks | post-war tours and barres |
| `industry` | 15×24 shed to 90×39 depot, 1–2f | brick loft factory, steel sheds | render/brick ateliers, steel sheds |

The gate is the generator, not the table: for each family alone, every map type × size generates for a sample of seeds, and the release sweep generates for all three.

## Slices

| | Slice | Owner | Depends on |
| --- | --- | --- | --- |
| A | **Graph apartments.** One graph exporter generalized out of `china.py` (China bytes identical), then `apartments_new_york` from `NYC_CornerBuilding.blend` and `apartments_paris` from `FrenchBuilding.blend`: `urban_apartment` for both families | lane A | — |
| B | **Homes and farms**: `homes_<family>` (detached and attached homes) and `farmsteads_<family>` for both families, out of `homes.py`/`farmsteads.py` made family-parameterized | lane B | — |
| C | **Towers and industry**: `towers_<family>` and `industry_<family>` for both families, out of `towers.py`/`industry.py` made family-parameterized | lane C | — |
| D | **Integration**: catalogue, bake, presets, re-pinned identities, per-family generation gate, line-up and battle screenshots, docs | integrator | A, B, C |
| E | Closeout: whole-spec review, choices ledger, close-spec | integrator | D |

Lanes are split by script, not by family, so no two lanes edit the same script; shared texture recipes and `assets/catalog.json` rows are additive. Lanes A–C run in parallel worktrees and commit **sources only** (scripts, `assets/source/city/<set>/`, `assets/catalog.json` set/appearance rows, shared helper changes). They may bake locally to judge their art in the line-up lab, but never commit `assets/runtime/` or the physical catalogue; the integrator rebakes once.

## Next Agent Prompt

Lanes A, B and C are running. On their return: merge each, rerun every China script and `cmp` its outputs, then slice D. Blender 5.2.1 is at `~/Applications/Blender-5.2.1.app` (`BLENDER=` that binary); `asset` needs Node 24 (`~/.nvm/versions/node/v24.21.0/bin`) ahead of Bun on `PATH`. Script arguments follow the script path directly (no `--`).
