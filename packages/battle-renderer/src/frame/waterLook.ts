// The water's and the shore's records in the terrain's params
// (`TerrainParams.shoreTuning`, `waterLooks`): packed in Rust
// (`static_world::ground::tables`), uniforms so a look retunes without
// rebuilding a shader.
import { d } from "typegpu";

/** The bank's bands and ground (`groundShore`, `groundColour`):
 *  `presentation.terrain.shore`. */
export const ShoreTuning = d.struct({
  /** Where the wet bank and the earth are whole (shares of their reach),
   *  how much finer the fine wander is, its share of the wander. */
  bands: d.vec4f,
  /** The broad and the fine wanders' lattice turns, each as its cosine and
   *  sine (`turned`). */
  turns: d.vec4f,
  /** The earth's mottle, the metres the field eases to the verge's over,
   *  the earth's and the wet bank's roughness. */
  ground: d.vec4f,
  /** The pixel footprints the fine wander fades over (in its own cells),
   *  the metres the bed eases in over; unused. */
  fade: d.vec4f,
});

/** The most water looks a biome holds (`biome::WATER_LOOKS_MAX`). */
export const WATER_LOOKS_MAX = 8;

/** One look of the water's surface: the biome's look
 *  (`biome.water.rivers`) over the shared surface
 *  (`presentation.terrain.water`). */
export const WaterLook = d.struct({
  /** Its body's colour (linear rgb) where little light survives the depth,
   *  and its streaks' light. */
  channel: d.vec4f,
  /** Its body's colour (linear rgb) where most of the light survives, and
   *  the surface's roughness. */
  edge: d.vec4f,
  /** The share of red, green and blue it takes per metre of water the light
   *  crosses (`exp(-e·d)`), and the share of its own light it keeps in
   *  shadow. */
  surface: d.vec4f,
  /** 1 / the primary, macro and detail normal layers' tile in metres, and
   *  how far the baked slope leans the normal. */
  normals: d.vec4f,
  /** Each layer's share of the wind's drift it moves at; unused. */
  drifts: d.vec4f,
  /** The light's lanes: their width, their drift (in lanes), 1 / the
   *  drift's scale, 1 / a streak's length. */
  lanes: d.vec4f,
  /** The lane's and the streak's weights, where the light starts and is
   *  whole. */
  lanesLit: d.vec4f,
  /** The lanes' mean light at a distance, the depth (metres) the water
   *  fades in over from its edge, the pixel footprints (metres) its ripples
   *  and streaks fade over. */
  lanesMean: d.vec4f,
  /** The bed's shift through the surface's slope, as a share of the depth
   *  it is seen through, the most depth it shifts by (metres), the most
   *  shift as a share of the screen's height, and the metres of water per
   *  mip the bed blurs by (`waterUnder`). */
  refraction: d.vec4f,
});
