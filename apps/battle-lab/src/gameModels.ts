// The models' presentation, from `fixtures/presentation.json`:
// the detail tiers by projected height and the impostor size,
// and the buildings' tiers, chunks and pool. Every view that draws models (the
// battle, the labs, the workbench) draws with them.
import { presentationSettings } from "@web/presentationSettings";
import type { BuildingStyle } from "@packages/battle-renderer/src/models/buildingReferences";

/** Handed to the canonical Rust frame's tier selection. */
export const gameModelDetail = JSON.stringify(presentationSettings.models);

export const gameBuildingStyle: BuildingStyle =
  presentationSettings.buildings as unknown as BuildingStyle;
