// Fog's presentation, from the one fixture owner's presentation block: the
// sight lights' resolution and budgets, the unseen look (named styles, one
// selected), the contact glyphs drawn over fog, and the border that marks
// the playable area (drawn sight runs on past it). Every lab route draws
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
import type { MapBorderStyle } from "@packages/battle-renderer/src/playAreaOverlay";
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

export const villageMapBorder: MapBorderStyle = (() => {
  const b = village.presentation.map_border as unknown as MapBorderStyle;
  if (b.color.length !== 4 || !(b.width_px > 0) || !(b.min_width_m > 0) || !(b.lift_m >= 0))
    throw new Error(
      "presentation.map_border: color rgba, width_px > 0, min_width_m > 0, lift_m ≥ 0",
    );
  return b;
})();
