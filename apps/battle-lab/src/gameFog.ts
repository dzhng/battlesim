// Fog's presentation, from `fixtures/presentation.json`: the
// unseen look (named styles, one selected), the contact glyphs drawn over fog,
// and the border that marks
// the playable area. The rendered margin outside that border stays clear.
// Every lab route draws with them; the presentation crate reads and checks
// the rest of the sight lights' geometry itself.
import { presentationSettings, selectedFogStyle } from "@web/presentationSettings";
import {
  validateContactGlyphStyle,
  type ContactGlyphStyle,
} from "@packages/battle-renderer/src/contactGlyph";
import type { MapBorderStyle } from "@packages/battle-renderer/src/playAreaOverlay";
import type { FogPresentation, FogStyle } from "@packages/battle-renderer/src/fog";
import { gameHud } from "@web/battle/present/hudTheme";

/** Its named unseen looks, checked by the presentation crate as each is
 *  resolved (`resolveFogLook`). */
export const gameFogPresentation = presentationSettings.fog as unknown as FogPresentation;

/** The unseen look the fixture selects. */
export const gameFogStyle: FogStyle = selectedFogStyle;

/** Contact glyphs, in the HUD's enemy colour. */
export const gameContactStyle: ContactGlyphStyle = validateContactGlyphStyle({
  ...(presentationSettings.contacts as unknown as Omit<ContactGlyphStyle, "color">),
  color: gameHud.enemy,
});

export const gameMapBorder: MapBorderStyle = presentationSettings.map_border;
