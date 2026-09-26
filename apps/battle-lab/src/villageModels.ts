// The models' presentation, from the one fixture owner's presentation block:
// the detail tiers by projected height and the impostor size. Every view that
// draws models (the battle, the labs, the workbench) draws with them.
import village from "@fixtures/village.json";
import {
  validateModelDetail,
  type ModelDetailPresentation,
} from "@packages/battle-renderer/src/models/modelDetail";

export const villageModelDetail: ModelDetailPresentation = validateModelDetail(
  village.presentation.models as ModelDetailPresentation,
);
