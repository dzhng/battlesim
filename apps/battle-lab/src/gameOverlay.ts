// The overlays' presentation, from `fixtures/presentation.json`
// (`presentation.overlay`): the halo every overlay carries, the orders'
// colours and widths, supply's colour and the objective zone's edge. Every
// lab route draws with them.
import { overlaySettings } from "@web/presentationSettings";
import { metresPerPxAt } from "@packages/renderer-core/src/camera3d";
import { type OverlayGlowStyle } from "@packages/battle-renderer/src/overlayStyle";
import { type OrderStyle } from "@packages/battle-renderer/src/orderOverlay";
import { type PaintStyle } from "@packages/battle-renderer/src/paintStyle";
import { type SupplyStyle } from "@packages/battle-renderer/src/supplyOverlay";
import type { Rgba } from "@packages/battle-renderer/src/mesh";
import { type RulerStyle } from "@packages/battle-renderer/src/rangeRulerOverlay";
import {
  strokeWidth,
  type StrokeRule,
  type StrokeWidth,
} from "@packages/battle-renderer/src/strokeWidth";
import { type XrayStyle } from "@packages/battle-renderer/src/models/modelInstances";
import { type OrderFlash } from "@web/battle/present/orderReveal";
import { gameHud } from "@web/battle/present/hudTheme";
import { gameCamera } from "./gameCamera";

/** The halo the overlay pass lays under what stays in the overlay (contacts
 *  and the x-ray): `presentation.overlay.glow.overlay`.
 *  The painted ground marks glow by their own emissive (`gamePaint`), the
 *  callouts by the HUD's CSS (`glow.callouts`, `hudTheme.ts`). */
export const gameOverlayGlow: OverlayGlowStyle = {
  radius_px: overlaySettings.glow.radius_px,
  strength: overlaySettings.glow.overlay,
};

/** The ground paint: its emissive glow
 *  (`glow.ground`), `paint`'s reflectance and fog. */
export const gamePaint: PaintStyle = {
  emissive: overlaySettings.glow.ground,
  ...overlaySettings.paint,
};

const { flash: orderFlash, ...orders } = overlaySettings.orders;

/** The HUD's one "can't" colour, as the ground marks draw it: a route the
 *  unit can't take, a range ruler past its last reach. */
const CANNOT: Rgba = [...gameHud.bad, 1];

/** The pending ground-attack footprint: a target-area warning, not a unit
 * selection colour. It shares the HUD's red warning role so it reads as the
 * consequence of firing into this patch of ground. */
export const gameAttackGroundSpread: Rgba = [...gameHud.bad, 0.82];

export const gameOrderStyle: OrderStyle = {
  ...(orders as unknown as Omit<OrderStyle, "blocked" | "ally">),
  blocked: CANNOT,
  ally: [...gameHud.ally, 1],
};

/** How long an order's marks show once it is given (`orders.flash`). */
export const gameOrderFlash: OrderFlash = orderFlash;

export const gameSupplyStyle: SupplyStyle = overlaySettings.supply as unknown as SupplyStyle;

/** The range ruler's ground paint (Space held with a selection). */
export const gameRulerStyle: RulerStyle = {
  ...(overlaySettings.ruler as unknown as Omit<RulerStyle, "beyond">),
  beyond: CANNOT,
};

/** The objective zone's edge. */
export const gameZone = overlaySettings.zone as unknown as Rgba;

/** The colour a unit's hidden parts are drawn in through the world in front
 *  of it (`ModelInstance.xray`). */
export const gameXray: XrayStyle = (() => {
  // A selected unit's hidden parts take the selection's colour, so the x-ray
  // and the selection marker can never disagree; only the alpha is set here.
  const x = overlaySettings.xray as unknown as {
    own: Rgba;
    ally: Rgba;
    selected_alpha: number;
  };
  const [r, g, b] = gameOrderStyle.selected;
  return { own: x.own, ally: x.ally, selected: [r, g, b, x.selected_alpha] };
})();

/** How every painted mark's stroke thins as the camera pulls out
 *  (`presentation.overlay.stroke`). */
const gameStrokeRule: StrokeRule = overlaySettings.stroke;

/** The marks' stroke widths where one pixel spans `metresPerPx`. */
export function gameStroke(metresPerPx: number, renderScale = 1): StrokeWidth {
  return strokeWidth(gameStrokeRule, metresPerPx, renderScale);
}

/** The line scale for routes that don't follow the camera (the labs): the
 *  opening camera's, on a 1080-pixel viewport. */
export const OPENING_METRES_PER_PX = (() => {
  const c = gameCamera.opening();
  return metresPerPxAt(c.distance, c.fovY, 1080);
})();

/** Ignore incidental occluded fragments; coverage is measured per drawn model. */
export const gameXrayMinHiddenFragmentFraction = overlaySettings.xray.min_hidden_fragment_fraction;
