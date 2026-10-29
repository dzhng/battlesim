/** How ground paint looks (`presentation.overlay`: `glow.ground` is
 *  `emissive`, `paint` the rest). */
export interface PaintStyle {
  /** The paint's reflectance, as a share of its colour (linear): paint is
   *  ground-dark, not a lamp, so shade shows on it. */
  albedo: number;
  /** The glow: the paint's colour, linear, times this, added to the lit
   *  surface it lies on. */
  emissive: number;
  /** How much of the fog painted ground takes, 0 none, 1 the ground's. */
  fog_keep: number;
  /** The paint's light up the grass over it: its strength at a blade's
   *  root, and the height it falls off over. */
  grass_glow: number;
  grass_falloff_m: number;
  /** How far the paint's colour is pushed from its grey (1 as given): the
   *  tone mapper and grade pull a bright hue toward white. */
  saturation: number;
}

export function validatePaintStyle(style: PaintStyle): PaintStyle {
  if (
    !(style.albedo > 0 && style.albedo <= 1) ||
    !(style.emissive >= 0 && style.emissive <= 8) ||
    !(style.fog_keep >= 0 && style.fog_keep <= 1) ||
    !(style.grass_glow >= 0 && style.grass_glow <= 8) ||
    !(style.grass_falloff_m > 0 && style.grass_falloff_m <= 4) ||
    !(style.saturation >= 0 && style.saturation <= 3)
  )
    throw new Error(
      `ground paint: albedo in (0, 1], emissive in [0, 8], fog_keep in [0, 1], grass_glow in [0, 8], grass_falloff_m in (0, 4], saturation in [0, 3], got ${JSON.stringify(style)}`,
    );
  return style;
}
