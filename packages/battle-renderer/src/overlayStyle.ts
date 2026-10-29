/** The halo every overlay mark carries (the lab's `presentation.overlay.glow`:
 *  its `radius_px`, and `ground` as the strength). */
export interface OverlayGlowStyle {
  /** The halo's reach on screen (about 2.5 Gaussian sigmas), in device pixels. */
  radius_px: number;
  /** The halo's gain over the blurred overlay; 0 draws none. */
  strength: number;
}

/** The longest halo the blur loops reach, in device pixels. */
export const GLOW_MAX_RADIUS_PX = 24;

export function validateOverlayGlow(glow: OverlayGlowStyle): OverlayGlowStyle {
  if (
    !(glow.radius_px > 0 && glow.radius_px <= GLOW_MAX_RADIUS_PX) ||
    !(glow.strength >= 0 && glow.strength <= 8)
  )
    throw new Error(
      `overlay glow: radius_px in (0, ${GLOW_MAX_RADIUS_PX}], strength in [0, 8], got ${JSON.stringify(glow)}`,
    );
  return glow;
}
