// The models' presentation, from the one fixture owner's presentation block:
// the detail tiers by projected height and the impostor size, how glass takes
// light, and the buildings' tiers, chunks and pool. Every view that draws models (the
// battle, the labs, the workbench) draws with them.
import game from "@fixtures/game.json";
import {
  validateGlass,
  type GlassStyle,
} from "@packages/battle-renderer/src/models/surfaceFragments";
import {
  validateBuildingStyle,
  type BuildingStyle,
} from "@packages/battle-renderer/src/models/buildingReferences";

/** Handed to Rust's tier rule unread (`model_tiers`). */
export const gameModelDetail = JSON.stringify(game.presentation.models);

export const gameGlass: GlassStyle = validateGlass(game.presentation.glass);

export const gameBuildingStyle: BuildingStyle = validateBuildingStyle(
  game.presentation.buildings as unknown as BuildingStyle,
);
