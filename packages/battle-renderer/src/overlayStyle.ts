/** The halo every overlay mark carries (the lab's `presentation.overlay.glow`:
 *  its `radius_px`, and `ground` as the strength). */
export interface OverlayGlowStyle {
  /** The halo's reach on screen (about 2.5 Gaussian sigmas), in device pixels. */
  radius_px: number;
  /** The halo's gain over the blurred overlay; 0 draws none. */
  strength: number;
}
