// The overlays' presentation, from the one fixture owner's presentation block
// (`presentation.overlay`): the halo every overlay carries and the orders'
// colours and widths. Every lab route draws with them.
import village from "@fixtures/village.json";
import { metresPerPxAt } from "@packages/renderer-core/src/camera3d";
import {
  validateOverlayGlow,
  type OverlayGlowStyle,
} from "@packages/battle-renderer/src/frame/overlayPass";
import { validateOrderStyle, type OrderStyle } from "@packages/battle-renderer/src/orderOverlay";
import { villageCamera } from "./villageCamera";

export const villageOverlayGlow: OverlayGlowStyle = validateOverlayGlow(
  village.presentation.overlay.glow,
);

export const villageOrderStyle: OrderStyle = validateOrderStyle(
  village.presentation.overlay.orders as unknown as OrderStyle,
);

type Rgba = readonly [number, number, number, number];

/** The colour a unit's hidden parts are drawn in through the world in front
 *  of it (`ModelInstance.xray`): the observing side's units in `own`, the
 *  selection in `selected`. */
export const villageXray: { own: Rgba; selected: Rgba } = (() => {
  const x = village.presentation.overlay.xray as unknown as { own: Rgba; selected: Rgba };
  const rgba = (c: Rgba) => c?.length === 4 && c.every((v) => v >= 0 && v <= 1) && c[3] > 0;
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
