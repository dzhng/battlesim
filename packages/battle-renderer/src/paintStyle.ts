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
