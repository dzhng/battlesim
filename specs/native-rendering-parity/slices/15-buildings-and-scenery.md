# 15 — Buildings, scenery and their state

**Contract:**
- Native draws the browser's buildings module by module, facades included, and its scenery: forest trees, understorey, backdrop and dressing.
- Both use the tiers the browser chose.
- Fallen buildings, wrecks, felled trees and cleared trees come from the publication.

**Seam:**
- **Placement is static per map, so the browser exports it.** The manifest gains hashed artifacts, built by the same owners the browser draws from:
  - building module instances per template and state (intact, ruin, gutted): `templateLibrary.ts`, `buildingPlacements.ts`, `buildingReferences.ts`;
  - scenery instances: `scenery/placement.ts`, `dressing.ts`, `forestShapes.ts`;
  - the tier rules: `buildingTier.ts`, `scenery/lod.ts`.

  Native ports none of the placement logic.
- **State is per frame, so native derives it from the publication** (slice 13):
  - known destroyed buildings swap state (`destroyedBuildings.ts`, `setFallenBuildings`);
  - `knownProps` wrecks and shoved bodies;
  - `fallenBodies` fall at `(tick−1)/tickHz` (`scenery/felled.ts`);
  - `cleared` foliage.
- Draw through `model_batches.gd`. `legacy_map_layer.gd` is deleted, and the token-matched shells, the 1 km reach and the `street_tree` cone go with it.

**Runnable artifact:** plate pairs: paris-corner close cuts for balconies, railings and lamp posts, and market-town broad cuts for tree lines and forests.

**Focused gates:**
- **Counts.** Per plate, native's drawn building-module and scenery instances per kind equal the browser's, which the browser exports from its layers in the plate's frame.
- **Visual.** `compare-screenshots`, then an unprimed `screenshot-critique`, then `preview-shots`.
- **Cost.** This layer's pass cost, and slice 12's measurement re-run on both scenes, with native at or below the browser.

**Visual variable:** buildings and scenery: identity, completeness, placement and state.

**Delegated:** artifact packing.
