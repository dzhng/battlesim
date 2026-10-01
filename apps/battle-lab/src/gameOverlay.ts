// The overlays' presentation, from the one fixture owner's presentation block
// (`presentation.overlay`): the halo every overlay carries, the orders'
// colours and widths, supply's colour and the objective zone's edge. Every
// lab route draws with them.
import game from "@fixtures/game.json";
import { metresPerPxAt } from "@packages/renderer-core/src/camera3d";
import {
  validateOverlayGlow,
  type OverlayGlowStyle,
} from "@packages/battle-renderer/src/frame/overlayPass";
import { validateOrderStyle, type OrderStyle } from "@packages/battle-renderer/src/orderOverlay";
import {
  validatePaintStyle,
  type PaintStyle,
} from "@packages/battle-renderer/src/frame/paintedMarks";
import { validateSupplyStyle, type SupplyStyle } from "@packages/battle-renderer/src/supplyOverlay";
import type { Rgba } from "@packages/battle-renderer/src/mesh";
import {
  validateRulerStyle,
  type RulerStyle,
} from "@packages/battle-renderer/src/rangeRulerOverlay";
import { validateZoneColor } from "@packages/battle-renderer/src/playAreaOverlay";
import {
  strokeWidth,
  validateStrokeRule,
  type StrokeRule,
  type StrokeWidth,
} from "@packages/battle-renderer/src/strokeWidth";
import { validateXray, type XrayStyle } from "@packages/battle-renderer/src/models/modelInstances";
import { validateOrderFlash, type OrderFlash } from "@web/battle/present/orderReveal";
import { gameHud } from "@web/battle/present/hudTheme";
import { gameCamera } from "./gameCamera";

/** The halo the overlay pass lays under what stays in the overlay (contacts
 *  and the x-ray): `presentation.overlay.glow.overlay`.
 *  The painted ground marks glow by their own emissive (`gamePaint`), the
 *  callouts by the HUD's CSS (`glow.callouts`, `hudTheme.ts`). */
export const gameOverlayGlow: OverlayGlowStyle = validateOverlayGlow({
  radius_px: game.presentation.overlay.glow.radius_px,
  strength: game.presentation.overlay.glow.overlay,
});

/** The ground paint (`frame/paintedMarks.ts`): its emissive glow
 *  (`glow.ground`), `paint`'s reflectance and fog. */
export const gamePaint: PaintStyle = validatePaintStyle({
  emissive: game.presentation.overlay.glow.ground,
  ...game.presentation.overlay.paint,
});

const { flash: orderFlash, ...orders } = game.presentation.overlay.orders;

/** The HUD's one "can't" colour, as the ground marks draw it: a route the
 *  unit can't take, a range ruler past its last reach. */
const CANNOT: Rgba = [...gameHud.bad, 1];

export const gameOrderStyle: OrderStyle = validateOrderStyle({
  ...(orders as unknown as Omit<OrderStyle, "blocked">),
  blocked: CANNOT,
});

/** How long an order's marks show once it is given (`orders.flash`). */
export const gameOrderFlash: OrderFlash = validateOrderFlash(orderFlash);

export const gameSupplyStyle: SupplyStyle = validateSupplyStyle(
  game.presentation.overlay.supply as unknown as SupplyStyle,
);

/** The range ruler's ground paint (Space held with a selection). */
export const gameRulerStyle: RulerStyle = validateRulerStyle({
  ...(game.presentation.overlay.ruler as unknown as Omit<RulerStyle, "beyond">),
  beyond: CANNOT,
});

/** The objective zone's edge. */
export const gameZone: Rgba = validateZoneColor(game.presentation.overlay.zone);

/** The colour a unit's hidden parts are drawn in through the world in front
 *  of it (`ModelInstance.xray`). */
export const gameXray: XrayStyle = (() => {
  // A selected unit's hidden parts take the selection's colour, so the x-ray
  // and the selection marker can never disagree; only the alpha is set here.
  const x = game.presentation.overlay.xray as unknown as { own: Rgba; selected_alpha: number };
  const [r, g, b] = gameOrderStyle.selected;
  return validateXray({ own: x.own, selected: [r, g, b, x.selected_alpha] });
})();

/** How every painted mark's stroke thins as the camera pulls out
 *  (`presentation.overlay.stroke`). */
const gameStrokeRule: StrokeRule = validateStrokeRule(game.presentation.overlay.stroke);

/** The marks' stroke widths where one pixel spans `metresPerPx`. */
export function gameStroke(metresPerPx: number): StrokeWidth {
  return strokeWidth(gameStrokeRule, metresPerPx);
}

/** The line scale for routes that don't follow the camera (the labs): the
 *  opening camera's, on a 1080-pixel viewport. */
export const OPENING_METRES_PER_PX = (() => {
  const c = gameCamera.opening();
  return metresPerPxAt(c.distance, c.fovY, 1080);
})();

/** Ignore incidental occluded fragments; coverage is measured per drawn model. */
export const gameXrayMinHiddenFragmentFraction =
  game.presentation.overlay.xray.min_hidden_fragment_fraction;
