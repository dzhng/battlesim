// WGSL owner: grass.

// @block grass_GrassParams
struct GrassParams {
  planes: array<vec4f, 4>,
  eye: vec4f,
  window: vec4f,
  grid: vec4u,
  density: vec4f,
  tiers: vec4f,
  shading: vec4f,
  clear: vec4f,
  counts: vec4u,
  ground: vec4f,
  sway: vec4f,
  growth: array<vec4f, GRASS_GROWTH_ROWS>,
  patches: array<vec4f, GRASS_GROWTH_ROWS>,
  mixes: array<vec4f, GRASS_GROWTH_ROWS * GRASS_MIX_MAX>,
  patchScales: vec4f,
  grain: vec4f,
  kinds: array<vec4f, GRASS_MAX_KINDS>,
  bases: array<vec4u, GRASS_MAX_KINDS>,
}

// @block grass_grassVertexOf
const GRASS_BLADE_GUST_FROM = 0.2;
const GRASS_BLADE_FLUTTER_PHASE = 0.35;
const GRASS_BLADE_FLUTTER_ACROSS = 0.5;
const GRASS_BLADE_LAID_LEAN = 1.1;
const GRASS_BLADE_LAID_SINK = 0.6;
fn grassVertexOf(vid: u32, iid: u32) -> GrassVertex {
  let P = &grassParams;
  let tier = tier;
  let c = clumps[tier.y + iid];
  let row = (*P).kinds[c.kind];
  var out: GrassVertex;
  if (vid / tier.z >= u32(row.y)) {
    // A blade this kind does not have: outside the depth range, clipped.
    out.clip = vec4f(0.0, 0.0, 2.0, 1.0);
    return out;
  }
  let base = select((*P).bases[c.kind].y, (*P).bases[c.kind].x, tier.x == 0u);
  let s = shapes[base + vid];
  let scale = c.height / max(row.x, 1e-4);
  let sn = c.yawSin;
  let cs = c.yawCos;
  let sp = s.spine.xyz * scale;
  let sd = s.side.xyz * scale * c.width;
  let spine = vec3f(sp.x * cs - sp.y * sn, sp.x * sn + sp.y * cs, sp.z);
  let side = vec3f(sd.x * cs - sd.y * sn, sd.x * sn + sd.y * cs, sd.z);
  // The one wind: a steady lean, gust fronts rolling downwind, a flutter.
  let t = cam.time;
  let wind = cameraWind();
  let dir = wind.toward.xy;
  let along = dot(c.root.xy, dir);
  let front = sin(6.2831853 * (along - t * wind.gustMps) * wind.gustPerM);
  let gust = smoothstep(GRASS_BLADE_GUST_FROM, 1.0, front);
  let flutter = (*P).sway.z * sin((*P).sway.w * t + s.side.w * 6.2831853 + along * GRASS_BLADE_FLUTTER_PHASE);
  let bend = s.spine.w * s.spine.w;
  let packed = unpack4x8unorm(c.colour);
  // Laid over by tracks or trampling: the clump sinks and its blades lean
  // out its own way, still in the wind but less. A stiff kind answers the
  // wind less (its shape's own response).
  let flat = 1.0 - packed.w;
  let answer = s.tint.w * (1.0 - flat);
  let lean = ((*P).sway.x + (*P).sway.y * gust + flutter) * answer;
  let push = dir * lean + vec2f(-dir.y, dir.x) * flutter * GRASS_BLADE_FLUTTER_ACROSS * answer;
  let laid = vec2f(cs, sn) * flat * GRASS_BLADE_LAID_LEAN;
  var world = c.root + vec3f(spine.xy, spine.z * (1.0 - flat * GRASS_BLADE_LAID_SINK)) + side + vec3f((push + laid) * c.height * bend, 0.0);
  world.z -= 0.5 * dot(push, push) * c.height * bend;
  let n = s.normal.xyz;
  let colour = packed.xyz;
  out.clip = cam.viewProj * vec4f(world, 1.0);
  // A small, stable per-blade depth nudge reduces nearly coplanar crossings.
  // Equal stored depths still rely on the clumps' deterministic draw order.
  let tie = ((bitcast<u32>(c.root.x) * 2654435761u) ^ (bitcast<u32>(c.root.y) * 2246822519u)) + (vid / tier.z) * 374761393u;
  out.clip.z *= 1.0 + 1e-3 * (f32(tie >> 8u) / 16777216.0 - 0.5);
  out.world = world;
  out.root = c.root;
  out.normal = vec3f(n.x * cs - n.y * sn, n.x * sn + n.y * cs, n.z);
  // A clump a few pixels tall shows its tops, not its roots: drawn with them,
  // its dark roots and gaps read as speckle, so its tint and its lighting
  // give way to the ground's.
  let footprint = (*P).eye.w * distance(cam.eye, c.root);
  let plain = (*P).shading.x * smoothstep((*P).tiers.z, (*P).tiers.w, footprint);
  out.albedo = colour * colour * mix(s.tint.xyz, vec3f(1.0), plain);
  out.ground = mix(1.0 - (*P).shading.y, 1.0, plain);
  return out;
}

// @block grass_grassDensity
const GRASS_DENSITY_LEAST_FACING = 0.15;
fn grassDensity(dist: f32, drop: f32) -> f32 {
  let P = &grassParams;
  let facing = clamp(drop / dist, GRASS_DENSITY_LEAST_FACING, 1.0);
  let footprint = (*P).eye.w * dist;
  return min((*P).density.y, facing / ((*P).density.x * footprint * footprint));
}

// @block grass_GrassClump
struct GrassClump {
  root: vec3f,
  height: f32,
  colour: u32,
  kind: u32,
  yawSin: f32,
  yawCos: f32,
  width: f32,
}

// @block grass_GrassShapeVertex
struct GrassShapeVertex {
  spine: vec4f,
  side: vec4f,
  normal: vec4f,
  tint: vec4f,
}

// @block grass_GrassVertex
struct GrassVertex {
  clip: vec4f,
  world: vec3f,
  root: vec3f,
  normal: vec3f,
  albedo: vec3f,
  ground: f32,
}

// @block grass_clumps_grassDrawLayout
@group(1) @binding(2) var<storage, read> clumps: array<GrassClump>;

// @block grass_clumps_grassBuildLayout
@group(0) @binding(3) var<storage, read_write> clumps: array<GrassClump>;

// @block grass_grassHash
fn grassHash(a: u32, b: u32) -> vec3f {
  var s = a * 747796405u + b * 2891336453u + 12345u;
  var out = vec3f(0.0);
  for (var k = 0u; k < 3u; k++) {
    out[k] = f32(pcgHash(s) >> 8u) * (1.0 / 16777216.0);
    s = s * 747796405u + 2891336453u;
  }
  return out;
}

// @block grass_grassUnderProp
fn grassUnderProp(p: vec2f) -> bool {
  let grid = props[0];
  let size = bitcast<vec4u>(props[1]);
  let g = (p - grid.xy) / grid.z;
  if (g.x < 0.0 || g.y < 0.0) { return false; }
  let c = vec2u(g);
  if (c.x >= size.x || c.y >= size.y) { return false; }
  let cell = c.y * size.x + c.x;
  let pair = bitcast<vec4u>(props[2u + cell / 2u]);
  let run = select(pair.xy, pair.zw, (cell & 1u) == 1u);
  for (var i = 0u; i < run.y; i++) {
    let a = props[run.x + 2u * i];
    let b = props[run.x + 2u * i + 1u];
    let o = p - a.xy;
    if (abs(dot(o, a.zw)) < b.x && abs(dot(o, vec2f(-a.w, a.z))) < b.y) { return true; }
  }
  return false;
}

// @block grass_params
@group(GROUP) @binding(0) var<uniform> grassParams: GrassParams;

// @block grass_shapes
@group(1) @binding(3) var<storage, read> shapes: array<GrassShapeVertex>;

// @block grass_ranges
@group(0) @binding(5) var<storage, read_write> ranges: array<vec2u>;
// @block grass_args
@group(0) @binding(4) var<storage, read_write> args: array<atomic<u32>, 12>;

// @block grass_props
@group(0) @binding(2) var<storage, read> props: array<vec4f>;
