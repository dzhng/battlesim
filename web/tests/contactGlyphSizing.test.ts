// @vitest-environment node
import { expect, test } from "vitest";
import {
  buildContactGlyphs,
  type ContactGlyphStyle,
} from "../../packages/battle-renderer/src/contactGlyph";
import { VERTEX_FLOATS } from "../../packages/battle-renderer/src/mesh";
import { strokeWidth, type StrokeRule } from "../../packages/battle-renderer/src/strokeWidth";

const rule: StrokeRule = {
  full_m_per_px: 0.05,
  thin_m_per_px: 0.4,
  thin_scale: 0.4,
  min_px: 1,
  min_m: 0.05,
};
const style: ContactGlyphStyle = {
  fade_s: 1,
  hatch_spacing_m: 2,
  hatch_width_m: 0.1,
  hatch_angle_deg: 0,
  outline_width_m: 0.2,
  outline_color: [0.25, 0.5, 0.75],
  color: [1, 0, 0],
  hatch_alpha: 0.2,
  glow_alpha: 0.2,
  fill_alpha: 0.5,
  lift_m: 0.01,
  air: {
    fill_alpha: 0.5,
    hatch_alpha: 0.2,
    glow_alpha: 0.2,
    hatch_px: 0.25,
    rim_px: 0.5,
    keyline_px: 0.25,
    keyline_alpha: 0.5,
    min_radius_px: 1.5,
    max_radius_px: 3,
    stem_px: 0.5,
    stem_alpha: 0.5,
  },
};

test("the contact body keeps its CSS radius when its strokes need more raster coverage", () => {
  const size = strokeWidth(rule, 2);
  for (const renderScale of [0.75, 1]) {
    const mesh = buildContactGlyphs(
      [{ center: [0, 0, 10], radius: 0.25, opacity: 1, aloft: true }],
      () => 0,
      style,
      { right: [1, 0, 0], up: [0, 0, 1], size, stroke: strokeWidth(rule, 2, renderScale) },
    ).translucent!;
    // The pale rim's outer edge bounds the body. The keyline/glow outside it
    // and the fill inside it may change with stroke width; neither is its radius.
    let radius = 0;
    for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) {
      if (mesh[i + 6] === 0.25 && mesh[i + 7] === 0.5 && mesh[i + 8] === 0.75 && mesh[i + 9] === 1)
        radius = Math.max(radius, Math.abs(mesh[i]));
    }
    expect(radius).toBe(2);
  }
});
