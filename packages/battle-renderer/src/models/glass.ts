/** `presentation.glass`: how a blended surface (a pane of glass)
 *  takes light. */
export interface GlassStyle {
  /** How far a pane's shading normal turns toward the eye: 0 shades it as
   *  modelled, 1 as if seen half as obliquely. It bounds what a pane mirrors
   *  at a grazing angle, so a window seen along a street stays a dark
   *  opening and never a pale plate of horizon. */
  turn: number;
  /** The brightest a pane's own light gets, as a multiple of a white matte
   *  surface in sun shadow (the environment's `unlit`): the sun's glint off
   *  it never outshines a wall. */
  glint: number;
}

export function validateGlass(glass: GlassStyle): GlassStyle {
  if (!(glass?.turn >= 0 && glass.glint > 0))
    throw new Error(
      `presentation.glass: turn must be ≥ 0 and glint > 0, got ${JSON.stringify(glass)}`,
    );
  return glass;
}
