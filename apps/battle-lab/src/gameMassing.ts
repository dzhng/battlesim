// Massing's presentation, from the one fixture owner's presentation block:
// which regional families have no art and draw as boxes, and each building
// category's tint. Every route that draws a map draws its artless buildings
// with them.
import game from "@fixtures/game.json";
import {
  validateMassingStyle,
  type MassingStyle,
} from "@packages/battle-renderer/src/scenery/massing";

export const gameMassing: MassingStyle = validateMassingStyle(
  game.presentation.massing as unknown as MassingStyle,
);
