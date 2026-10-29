// Fog's presentation, from the one fixture owner's presentation block: the
// unseen look (named styles, one selected), the contact glyphs drawn over fog,
// and the border that marks
// the playable area. The rendered margin outside that border stays clear.
// Every lab route draws with them; the presentation crate reads and checks
// the rest of the sight lights' geometry itself.
import game from "@fixtures/game.json";
import {
  validateContactGlyphStyle,
  type ContactGlyphStyle,
} from "@packages/battle-renderer/src/contactGlyph";
import {
  validateMapBorder,
  type MapBorderStyle,
} from "@packages/battle-renderer/src/playAreaOverlay";
import type { FogPresentation, FogStyle } from "@packages/battle-renderer/src/fog";
import { gameHud } from "@web/battle/present/hudTheme";

/** Its named unseen looks, checked by the presentation crate as each is
 *  resolved (`resolveFogLook`). */
export const gameFogPresentation = game.presentation.fog as unknown as FogPresentation;

/** The unseen look the fixture selects. */
export const gameFogStyle: FogStyle = gameFogPresentation.styles[gameFogPresentation.style];

/** Contact glyphs, in the HUD's enemy colour. */
export const gameContactStyle: ContactGlyphStyle = validateContactGlyphStyle({
  ...(game.presentation.contacts as unknown as Omit<ContactGlyphStyle, "color">),
  color: gameHud.enemy,
});

export const gameMapBorder: MapBorderStyle = validateMapBorder(
  game.presentation.map_border as unknown as MapBorderStyle,
);
