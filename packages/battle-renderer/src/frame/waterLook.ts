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
  /** Its body's colour (linear rgb) where little light survives the depth;
   *  unused. */
  channel: d.vec4f,
  /** Its body's colour (linear rgb) where most of the light survives;
   *  unused. */
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
  /** The depth (metres) the water fades in over from its edge, the pixel
   *  footprints (metres) its ripples fade over, and a puddle's depth at its
   *  centre (0 for a river's or a swamp's look, whose depth is the
   *  ground's). */
  fades: d.vec4f,
  /** Its reflection's three lobes' roughness, rising; unused. */
  roughness: d.vec4f,
  /** Each lobe's share of the reflection; unused. */
  lobes: d.vec4f,
  /** The bed's shift through the surface's slope, as a share of the depth
   *  it is seen through, the most depth it shifts by (metres), the most
   *  shift as a share of the screen's height, and the metres of water per
   *  mip the bed blurs by (`waterUnder`). */
  refraction: d.vec4f,
  /** The caustics on the bed (`waterCaustics`): 1 / their pattern's tile in
   *  metres, their strength, the share of their light lost a metre down,
   *  and how sharp their crests are. */
  caustics: d.vec4f,
});
