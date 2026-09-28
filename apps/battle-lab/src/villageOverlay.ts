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
import { isRgba, type Rgba } from "@packages/battle-renderer/src/mesh";
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
 *  (`glow.ground`), `paint`'s reflectance and fog, drawn at the orders'
 *  height. */
export const villagePaint: PaintStyle = validatePaintStyle({
  emissive: village.presentation.overlay.glow.ground,
  ...village.presentation.overlay.paint,
  // The marks are drawn at the orders' height; the ground reads them there.
  lift_m: village.presentation.overlay.orders.lift_m,
});

export const villageOrderStyle: OrderStyle = validateOrderStyle(
  resolveOrderScheme(village.presentation.overlay.orders as unknown as AuthoredOrderStyle),
);

export const villageSupplyStyle: SupplyStyle = validateSupplyStyle(
  village.presentation.overlay.supply as unknown as SupplyStyle,
);

export const villageConsequenceStyle: ConsequenceStyle = validateConsequenceStyle(
  village.presentation.overlay.consequences as unknown as ConsequenceStyle,
);

/** The objective zone's edge. */
export const villageZone: Rgba = (() => {
  const zone = village.presentation.overlay.zone as unknown;
  if (!isRgba(zone)) throw new Error("presentation.overlay.zone: rgba in [0, 1]");
  return zone;
})();

/** The colour a unit's hidden parts are drawn in through the world in front
 *  of it (`ModelInstance.xray`): the observing side's units in `own`, the
 *  selection in `selected`. */
export const villageXray: { own: Rgba; selected: Rgba } = (() => {
  const x = village.presentation.overlay.xray as unknown as { own: Rgba; selected: Rgba };
  const rgba = (c: Rgba) => isRgba(c) && c[3] > 0;
  if (!rgba(x.own) || !rgba(x.selected))
    throw new Error("presentation.overlay.xray: own and selected, rgba in [0, 1] with alpha > 0");
  return x;
})();

/** The line scale for routes that don't follow the camera (the labs): the
 *  opening camera's, on a 1080-pixel viewport. */
export const OPENING_METRES_PER_PX = (() => {
  const c = villageCamera.opening();
  return metresPerPxAt(c.distance, c.fovY, 1080);
})();

/** A vehicle kind's hull half-length, from its footprint (`physics.<kind>_half_extents_m`):
 *  what its marker circle is sized from. 0 for a kind with none. */
export function hullHalfLength(kind: string): number {
  const extents = (village.physics as Record<string, unknown>)[`${kind}_half_extents_m`];
  return Array.isArray(extents) && typeof extents[0] === "number" ? extents[0] : 0;
}
