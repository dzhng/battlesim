# 07 Regional appearances, loaded for the map's region only

**Unlocks:** a shared piece that looks like its region, and a download that grows with one region's art, not three.

**Seam:** asset appearance entries (`assets/catalog.json`, `packages/scene-assets/src/schema.ts`) gain optional `regional_family`, validated against the presets' families. `PropAppearances` (`models/propAppearance.ts`) filters a scenery kind's candidates to the map's family and family-less ones, preferring the family's, then its existing footprint fit. The loader fetches family-tagged appearances only for the map's `regional_family` (slice 04), alongside the kits it already selects per map. `downloadBudget.test.ts` counts a family's kits plus shared scenery plus that family's scenery against 50 MiB.

**Verify (tests first):** selection unit tests with fake variants; the budget test red first with an oversized fake variant; a map of one family requests no other family's bundles.

**Delegated:** where the filter lives in the loader.
