// Fog's presentation, from the one fixture owner's presentation block: the
// sight lights' resolution and budgets, the unseen look (named styles, one
// selected), the contact glyphs drawn over fog, and the border that marks
// the playable area (drawn sight runs on past it). Every lab route draws
// with them.
import game from "@fixtures/game.json";
import {
  validateFogGeometry,
  type FogGeometryPresentation,
} from "@packages/battle-renderer/src/frame/fogInputs";
import {
  validateContactGlyphStyle,
  type ContactGlyphStyle,
} from "@packages/battle-renderer/src/contactGlyph";
import {
  validateMapBorder,
  type MapBorderStyle,
} from "@packages/battle-renderer/src/playAreaOverlay";
import {
  selectedFogStyle,
  validateFogPresentation,
  type FogPresentation,
  type FogStyle,
} from "@packages/battle-renderer/src/frame/fogStyle";
import { gameHud } from "@web/battle/present/hudTheme";

export const gameFogGeometry: FogGeometryPresentation = validateFogGeometry(
  game.presentation.fog_geometry as unknown as FogGeometryPresentation,
);

export const gameFogPresentation: FogPresentation = validateFogPresentation(
  game.presentation.fog as unknown as FogPresentation,
);

/** The unseen look the fixture selects. */
export const gameFogStyle: FogStyle = selectedFogStyle(gameFogPresentation);

/** Contact glyphs, in the HUD's enemy colour. */
export const gameContactStyle: ContactGlyphStyle = validateContactGlyphStyle({
  ...(game.presentation.contacts as unknown as Omit<ContactGlyphStyle, "color">),
  color: gameHud.enemy,
});

export const gameMapBorder: MapBorderStyle = validateMapBorder(
  game.presentation.map_border as unknown as MapBorderStyle,
);
