// The models' presentation, from the one fixture owner's presentation block:
// the detail tiers by projected height and the impostor size, how glass takes
// light, and the buildings' tiers, chunks and pool. Every view that draws models (the
// battle, the labs, the workbench) draws with them.
import game from "@fixtures/game.json";
import {
  validateModelDetail,
  type ModelDetailPresentation,
} from "@packages/battle-renderer/src/models/modelDetail";
import {
  validateGlass,
  type GlassStyle,
} from "@packages/battle-renderer/src/models/surfaceFragments";
import {
  validateBuildingStyle,
  type BuildingStyle,
} from "@packages/battle-renderer/src/models/buildingReferences";

export const gameModelDetail: ModelDetailPresentation = validateModelDetail(
  game.presentation.models as ModelDetailPresentation,
);

export const gameGlass: GlassStyle = validateGlass(game.presentation.glass);

export const gameBuildingStyle: BuildingStyle = validateBuildingStyle(
  game.presentation.buildings as unknown as BuildingStyle,
);
