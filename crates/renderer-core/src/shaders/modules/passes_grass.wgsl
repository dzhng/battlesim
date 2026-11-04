// WGSL owner: passes_grass.

// @block passes_grass_grassFragment
const GRASS_BLADE_ROUGHNESS = 0.9f;
@fragment fn grassFragment(input: GrassFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let eye = (&cam.eye);
  let up = vec3f(0, 0, 1);
  let seen = fogTerm(vec3f(input.world.x, input.world.y, input.root.z), up, clip.xy, fogIsGround());
  let n = faceEyeNormal(normalize(input.normal), ((*eye) - input.world));
  let shading = normalize(mix(n, up, input.ground));
  let sunlight = sampleSunShadow(input.world, up, clip.xy);
  let under = groundPaint(vec3f(input.world.x, input.world.y, input.root.z));
  let here = groundPaintAtPixel(clip.xy);
  let lit = shade(input.albedo, vec3f(), GRASS_BLADE_ROUGHNESS, 0f, 0f, 1f, shading, input.world, sunlight, (*eye));
  return ColorFog(vec4f((lit.xyz + max(grassPaintGlow(under, (input.world.z - input.root.z)), grassPaintGlow(here, 0f))), 1f), fogCoverage(seen, 1f));
}

// @block passes_grass_GrassFragmentInput
struct GrassFragmentInput {
  @location(0) world: vec3f,
  @location(1) root: vec3f,
  @location(2) normal: vec3f,
  @location(3) albedo: vec3f,
  @location(4) ground: f32,
}

// @block passes_grass_grassFinish
@compute @workgroup_size(1) fn grassFinish() {
  var total = vec2u(0u);
  for (var tile = 0u; tile < grassParams.grid.x * grassParams.grid.y; tile++) {
    let at = tile * GRASS_TILE_RANGES;
    let count = ranges[at];
    ranges[at] = total;
    total += count;
  }
  for (var t = 0u; t < 2u; t++) {
    args[GRASS_RAW_COUNT_OFFSET + t] = total[t];
    args[t * GRASS_DRAW_WORDS + 1u] = min(total[t], select(grassParams.grid.w, grassParams.grid.z, t == 0u));
  }
}

// @block passes_grass_grassVertex
@vertex fn grassVertex(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> GrassVertexOutput {
  return grassVertexOf(vid, iid);
}

// @block passes_grass_grassBuild
var<workgroup> laneCounts: array<vec2u, GRASS_BUILD_LANES>;

@compute @workgroup_size(GRASS_BUILD_LANES) fn grassCount(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) li: u32) {
  let found = growTile(wg, li, false, vec2u(0u));
  laneCounts[li] = found;
  workgroupBarrier();
  let tile = (wg.y * grassParams.grid.x + wg.x) * GRASS_TILE_RANGES;
  // Lane offsets and the tile total, ready for the cross-tile prefix pass.
  var first = vec2u(0u);
  for (var i = 0u; i < li; i++) { first += laneCounts[i]; }
  ranges[tile + 1u + li] = first;
  if (li + 1u == GRASS_BUILD_LANES) { ranges[tile] = first + found; }
}

@compute @workgroup_size(GRASS_BUILD_LANES) fn grassBuild(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) li: u32) {
  let tile = (wg.y * grassParams.grid.x + wg.x) * GRASS_TILE_RANGES;
  _ = growTile(wg, li, true, ranges[tile] + ranges[tile + 1u + li]);
}

// @block passes_grass_GrassVertexOutput
struct GrassVertexOutput {
  @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) root: vec3f,
  @location(2) normal: vec3f,
  @location(3) albedo: vec3f,
  @location(4) ground: f32,
}
