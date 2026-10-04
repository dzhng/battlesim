# New York and Paris building families

Generated maps are built in one of three regional families: China, New York or Paris. City-maps shipped China alone and left New York and Paris as later families through the same source/export contract ([closed spec](../city-maps/README.md#scope-and-release-decision)); its [choices ledger](../city-maps/choices.md) added that another shipping family needs real category coverage and preset selection rather than an art fallback. The [choices ledger](choices.md) records every decision the plan left open.

## What shipped

A map's family is a draw of its own seeded stream ([`parcels::family`](../../../crates/mapgen/src/parcels/mod.rs)) over the presets' `parcels.regional_families` ([map-presets.json](../../../fixtures/map-presets.json)). Every building of a map, town and countryside alike, is of that family. The share address `/battle?type=&size=&seed=` therefore still names one battlefield, and the menu offers no region choice.

Each family has its own art in all six categories (detached home, attached home, urban apartment, farmstead, highrise, industry). Footprints and floor counts sit at China's range, so the same lots fill:

- **New York:**
  - pre-war brick walk-ups and corner buildings from the vendored NYC graph, with fire escapes, cornices, water tanks and shops;
  - Cape Cod, Foursquare, Dutch Colonial and bungalow houses with porches;
  - brownstones with stoops, and shop rows under flats;
  - a red-barn farm with a gambrel barn and a silo;
  - brick and limestone towers with water tanks;
  - a brick loft and stepped-front warehouses.
- **Paris:**
  - Haussmann blocks from the vendored French graph, with ashlar, iron balconies and zinc mansards with dormers;
  - meulière and rendered pavillons with shutters;
  - mansard rows and shops;
  - a longère and a farmyard with a dovecote and a half-timbered barn;
  - post-war tours and barres;
  - a sawtooth atelier and a barrel-vaulted depot.

Nothing new crosses a runtime boundary. Families travel through contracts that already existed: the template descriptor's `regional_family` ([contract](../../../crates/contract/src/templates.rs)), the city source sets ([city readme](../../../packages/scene-assets/blender/city/README.md)), the derived physical catalogue (`fixtures/prototype-building-templates.json`) and the runtime bake. The simulation and renderer are family-agnostic.

## Why it is shaped this way

- **One builder, a design per family.** A family is data over one owner, never a fork of a script:
  - each scripted kind (`homes.py`, `farmsteads.py`, `towers.py`, `industry.py`) takes `<script> [family] [out]` through `kit.family_set`/`kit.design`, with each family a design in the script's table and the new families' shared parts in `regional.py`;
  - graph-derived apartments share [`graphset.py`](../../../packages/scene-assets/blender/city/graphset.py); `china.py`, `nyc.py` and `paris.py` hold their graph's tables and the few hooks that read it.

  A fourth family is a new design or a new graph table.
- **China is frozen.** The refactors had to rebuild every China set byte-identical. They do: all fifteen sets, China's and the new ones, rebuild to their committed bytes. China's released art and saved maps are unchanged.
- **Physics first, art inside it.** Regional features that would claim cover the simulation does not give are drawn inside the box or kept to declared art fits:
  - porches are inset into the box;
  - stoops stay within the side fit;
  - water tanks are decorative fit above the roof;
  - New York towers have no setbacks, because the contract has no stacked-part join.
- **Coverage is a generator property, not an art inventory.** Lots are cut to templates. A family without small footprints or low apartments would fail narrow frontage or Open maps, whatever its art looked like. So coverage is proved by generation, not by a table.

## Invariants and where they are held

- **Every listed family covers every category, at the accepted scale.** Each family needs at least three variants per category, apartments on both sides of six floors, industry at one or two floors, and storeys of 12 m or less. This is held by `every_family_has_several_variants_of_every_category_at_its_accepted_scale` in [`prototype_catalogue.rs`](../../../crates/mapgen/tests/prototype_catalogue.rs).
- **Each family alone builds every map type, towns and countryside.** This is held by `each_regional_family_alone_builds_every_map_type` in [`parcels.rs`](../../../crates/mapgen/tests/parcels.rs). One map, one family, is `every_building_of_a_map_is_of_one_regional_family`.
- **One map fetches one family's art within 50 MiB.** This is held by `downloadBudget.test.ts` in [`web/tests/sceneAssets`](../../../web/tests/sceneAssets).
- **Every declared fighting bay has a visible opening**, refused at export for every generated set. China's graph set is the one named, printed exception ([city readme](../../../packages/scene-assets/blender/city/README.md), "Rules a set keeps").
- **Released saved maps keep their battlefield.** Market Town and the camera lab select their China template ids from the growing library, so their pinned catalogue hash still resolves ([saved maps](../../../fixtures/README.md#saved-maps)).
- **Map identity moved once, deliberately.** The presets moved to `layout-presets-13` and the catalogue moved, and the native halves of the paired native/Wasm records were re-recorded against the new requests. No record changed its exit code or status.

## Known limits

- Some industrial lots share massing across families, in regional dress: the shed and works in all three, the twin-span warehouse in China and Paris (ledger).
- Some far-tier pops come from machinery China shares, and are left to keep China's bytes: shutters as colour bands, ruin debris, burnt-roof holes.
- Paris corner balconies wrap onto the side wall.
- A gutted Paris block reads as a flat slab.
- Ruins of every family keep the shared knee-high ring.

## Rejected directions

- **Re-tagging China's generic houses, sheds and towers as other families.** That would be the "art fallback" city-maps ruled out. Every category got regional designs, even where massing is shared.
- **Widening the industrial scale band for a three-storey loft.** The loft became two storeys instead. A requirement is not loosened to fit art.
- **An `optional` module flag so families could share fitting tables.** It silently disabled the unplaced-module check for China. Fittings are now offered, and built only where placed.
- **Regenerating the saved maps.** That would have re-reviewed released battlefields for no physical change. Selecting their templates by id keeps them exact.

## Evidence

At close:
- the per-family release sweep generated every type × size for four seeds in each family (108 of 108);
- the `generated` browser scene passed on a Paris-drawn map (Mixed Small seed 1), and a camera flight passed through a New York town (Metro Small seed 1);
- unprimed critiques of line-up sheets and in-game shots drove the art fixes.

The vendored graphs' provenance is in [assets/README.md](../../../assets/README.md) and the [vendored folder](../../../packages/scene-assets/blender/vendor/procedural-buildings/README.md).

Retained limits outside this feature, found on `main` at close:
- `asset check` reports an orphan `assets/runtime/audio`, because the asset bake does not know the sound library's clips;
- clippy from Rust 1.99 flags untouched `sim`/`mapgen` code;
- `labLoading.test.tsx`'s `/benchmark` wait (1 s) is marginal under load.
