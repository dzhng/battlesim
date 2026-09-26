// Fog's presentation, from the one fixture owner's presentation block: the
// sight lights' resolution and budgets, the unseen look (named styles, one
// selected), and the contact glyphs drawn over fog. Every lab route draws
// with them.
import village from "@fixtures/village.json";
import {
  validateFogGeometry,
  type FogGeometryPresentation,
} from "@packages/battle-renderer/src/frame/fogInputs";
import {
  validateContactGlyphStyle,
  type ContactGlyphStyle,
} from "@packages/battle-renderer/src/contactGlyph";
import {
  selectedFogStyle,
  validateFogPresentation,
  type FogPresentation,
  type FogStyle,
} from "@packages/battle-renderer/src/frame/fogStyle";

export const villageFogGeometry: FogGeometryPresentation = validateFogGeometry(
  village.presentation.fog_geometry as unknown as FogGeometryPresentation,
);

export const villageFogPresentation: FogPresentation = validateFogPresentation(
  village.presentation.fog as unknown as FogPresentation,
);

/** The unseen look the fixture selects. */
export const villageFogStyle: FogStyle = selectedFogStyle(villageFogPresentation);

export const villageContactStyle: ContactGlyphStyle = validateContactGlyphStyle(
  village.presentation.contacts as unknown as ContactGlyphStyle,
);
