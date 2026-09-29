// The overlays' presentation, from the one fixture owner's presentation block
// (`presentation.overlay`): the halo every overlay carries, the orders'
// colours and widths, supply's and the consequences' colours and the
// objective zone's edge. Every lab route draws with them.
import village from "@fixtures/village.json";
import { metresPerPxAt } from "@packages/renderer-core/src/camera3d";
import {
  validateOverlayGlow,
  type OverlayGlowStyle,
} from "@packages/battle-renderer/src/frame/overlayPass";
import {
  resolveOrderScheme,
  validateOrderStyle,
  type AuthoredOrderStyle,
  type OrderStyle,
} from "@packages/battle-renderer/src/orderOverlay";
import {
  validatePaintStyle,
  type PaintStyle,
} from "@packages/battle-renderer/src/frame/paintedMarks";
import { validateSupplyStyle, type SupplyStyle } from "@packages/battle-renderer/src/supplyOverlay";
import {
  validateConsequenceStyle,
  type ConsequenceStyle,
} from "@packages/battle-renderer/src/consequenceOverlay";
import type { Rgba } from "@packages/battle-renderer/src/mesh";
import {
  validateRulerStyle,
  type RulerStyle,
} from "@packages/battle-renderer/src/rangeRulerOverlay";
import { validateZoneColor } from "@packages/battle-renderer/src/playAreaOverlay";
import { validateXray, type XrayStyle } from "@packages/battle-renderer/src/models/modelInstances";
import { validateOrderFlash, type OrderFlash } from "@web/battle/present/orderReveal";
import { villageCamera } from "./villageCamera";

/** The halo the overlay pass lays under what stays in the overlay (contacts,
 *  the x-ray, garrison and guidance marks): `presentation.overlay.glow.overlay`.
 *  The painted ground marks glow by their own emissive (`villagePaint`), the
 *  callouts by the HUD's CSS (`glow.callouts`, `hudTheme.ts`). */
export const villageOverlayGlow: OverlayGlowStyle = validateOverlayGlow({
  radius_px: village.presentation.overlay.glow.radius_px,
  strength: village.presentation.overlay.glow.overlay,
});

/** The ground paint (`frame/paintedMarks.ts`): its emissive glow
 *  (`glow.ground`), `paint`'s reflectance and fog. */
export const villagePaint: PaintStyle = validatePaintStyle({
  emissive: village.presentation.overlay.glow.ground,
  ...village.presentation.overlay.paint,
});

const { flash: orderFlash, ...orders } = village.presentation.overlay.orders;

export const villageOrderStyle: OrderStyle = validateOrderStyle(
  resolveOrderScheme(orders as unknown as AuthoredOrderStyle),
);

/** How long an order's marks show once it is given (`orders.flash`). */
export const villageOrderFlash: OrderFlash = validateOrderFlash(orderFlash);

export const villageSupplyStyle: SupplyStyle = validateSupplyStyle(
  village.presentation.overlay.supply as unknown as SupplyStyle,
);

export const villageConsequenceStyle: ConsequenceStyle = validateConsequenceStyle(
  village.presentation.overlay.consequences as unknown as ConsequenceStyle,
);

/** The range ruler's ground paint (Space held with a selection). */
export const villageRulerStyle: RulerStyle = validateRulerStyle(
  village.presentation.overlay.ruler as unknown as RulerStyle,
);

/** The objective zone's edge. */
export const villageZone: Rgba = validateZoneColor(village.presentation.overlay.zone);

/** The colour a unit's hidden parts are drawn in through the world in front
 *  of it (`ModelInstance.xray`). */
export const villageXray: XrayStyle = (() => {
  // A selected unit's hidden parts take the scheme's selection colour, so the
  // x-ray and the selection marker can never disagree; only the alpha is set here.
  const x = village.presentation.overlay.xray as unknown as { own: Rgba; selected_alpha: number };
  const [r, g, b] = villageOrderStyle.selected;
  return validateXray({ own: x.own, selected: [r, g, b, x.selected_alpha] });
})();

/** The line scale for routes that don't follow the camera (the labs): the
 *  opening camera's, on a 1080-pixel viewport. */
export const OPENING_METRES_PER_PX = (() => {
  const c = villageCamera.opening();
  return metresPerPxAt(c.distance, c.fovY, 1080);
})();
