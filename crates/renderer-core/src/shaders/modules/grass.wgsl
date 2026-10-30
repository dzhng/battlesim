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
fn grassVertexOf(vid: u32, iid: u32) -> GrassVertexOutput {
  let P = &grassParams;
  let grassTier = grassTier;
  let c = clumps[grassTier.y + iid];
  let row = (*P).kinds[c.kind];
  var out: GrassVertexOutput;
  if (vid / grassTier.z >= u32(row.y)) {
    // A blade this kind does not have: outside the depth range, clipped.
    out.clip = vec4f(0.0, 0.0, 2.0, 1.0);
    return out;
  }
  let base = select((*P).bases[c.kind].y, (*P).bases[c.kind].x, grassTier.x == 0u);
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
  let tie = ((bitcast<u32>(c.root.x) * 2654435761u) ^ (bitcast<u32>(c.root.y) * 2246822519u)) + (vid / grassTier.z) * 374761393u;
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
@group(0) @binding(4) var<storage, read_write> args: array<u32, 12>;

// @block grass_props
@group(0) @binding(2) var<storage, read> props: array<vec4f>;

// @block grass_grassTier
@group(1) @binding(1) var<uniform> grassTier: vec4u;

// @block grass_growTile
const GRASS_PLACEMENT_JITTER_M = 0.4;
const GRASS_PLACEMENT_JITTER_SPAN_M = 0.8;
const GRASS_STAND_BARE_WEAR = 0.9;
const GRASS_PLACEMENT_HEADLAND_M = 0.3;
const GRASS_STAND_SPARSE_FROM = 0.3;
const GRASS_STAND_SPARSE_TO = 0.55;
const GRASS_STAND_EDGE_DENSITY = 0.5;
const GRASS_STAND_GROW_FROM = 0.7;
const GRASS_MIX_DRIFT_FROM = 0.4;
const GRASS_MIX_DRIFT_TO = 0.65;
const GRASS_MIX_DRIFT_GATHER = 3;
const GRASS_STAND_EDGE_HEIGHT = 0.35;
const GRASS_STAND_LEAST_HEIGHT_M = 0.02;
const GRASS_STAND_TIER_JITTER_FROM = 0.85;
const GRASS_STAND_TIER_JITTER_TO = 1.15;
const GRASS_STAND_PARCHED_FROM = 0.55;
const GRASS_STAND_PARCHED_TO = 0.8;
// Each tile owns a prefix range, each lane a range within that tile. Count
// before emitting so GPU scheduling changes neither draw order nor overflow
// selection. A lane keeps its candidates in sequence order.
fn growTile(wg: vec3u, li: u32, emit: bool, first: vec2u) -> vec2u {
  var found = vec2u(0u);
  let P = &grassParams;
  if (wg.x >= (*P).grid.x || wg.y >= (*P).grid.y) { return found; }
  let T = (*P).window.z;
  let lo = (*P).window.xy + vec2f(f32(wg.x), f32(wg.y)) * T;
  let hi = lo + vec2f(T);
  // The tile's box: its ground at the corners and centre, a metre either
  // way, and the tallest clump above.
  var zlo = 1e9;
  var zhi = -1e9;
  for (var k = 0u; k < 5u; k++) {
    var q = (lo + hi) * 0.5;
    if (k < 4u) { q = vec2f(select(lo.x, hi.x, (k & 1u) == 1u), select(lo.y, hi.y, k >= 2u)); }
    let z = grassGround(q);
    zlo = min(zlo, z);
    zhi = max(zhi, z);
  }
  zlo -= 1.0;
  zhi += 1.0 + (*P).window.w;
  let lo2 = lo - vec2f(GRASS_PLACEMENT_JITTER_M + (*P).clear.y);
  let hi2 = hi + vec2f(GRASS_PLACEMENT_JITTER_M + (*P).clear.y);
  for (var i = 0u; i < 4u; i++) {
    let pl = (*P).planes[i];
    let pv = vec3f(select(lo2.x, hi2.x, pl.x >= 0.0), select(lo2.y, hi2.y, pl.y >= 0.0), select(zlo, zhi, pl.z >= 0.0));
    if (dot(pl.xyz, pv) + pl.w < 0.0) { return found; }
  }
  let eye = (*P).eye.xyz;
  let nearest = clamp(eye, vec3f(lo2, zlo), vec3f(hi2, zhi));
  let dNear = max(distance(eye, nearest), 0.5);
  if ((*P).eye.w * dNear >= (*P).density.w) { return found; }
  let n = u32(ceil(T * T * grassDensity(dNear, max(eye.z - zlo, 0.0))));
  // The tile's own offset of the sequence, from its place on the world grid.
  let cell = vec2i(floor(lo / T + 0.5));
  let offset = grassHash(bitcast<u32>(cell.x), bitcast<u32>(cell.y)).xy;
  for (var j = li; j < n; j += GRASS_BUILD_LANES) {
    // The sequence's point, jittered by a fixed amount so a sparse field
    // shows no lattice; both depend on the tile and j alone.
    let h = grassHash(bitcast<u32>(cell.x) ^ (j * 2654435761u), bitcast<u32>(cell.y) + j);
    let jitter = (vec2f(fract(h.z * 7.13), fract(h.x * 5.31)) - 0.5) * GRASS_PLACEMENT_JITTER_SPAN_M;
    let fell = lo + T * fract(offset + f32(j + 1u) * vec2f(0.7548776662466927, 0.5698402909980532)) + jitter;
    let fellZ = grassGround(fell);
    let dist = max(distance(eye, vec3f(fell, fellZ)), 0.5);
    let footprint = (*P).eye.w * dist;
    if (footprint >= (*P).density.w) { continue; }
    // GrassClump j stands once the density passes (j + 1/2) per tile.
    let rank = (f32(j) + 0.5) / (T * T);
    let rho = grassDensity(dist, max(eye.z - fellZ, 0.0));
    if (rank >= rho) { continue; }
    let cell = groundCell(fell, footprint);
    let paved = groundPaved(fell, cell);
    let site = groundSite(fell, cell, paved);
    let water = groundWater(fell, cell);
    // Bare within the margins; thinner and lower for a metre beyond them, so
    // a field meets a road or a wood without a wall of blades. Across a
    // road's shoulder it thins and lowers further as the ground is worn.
    // A wood's edge is its rect or its floor's ragged verge, whichever lies
    // farther out. A tree line is no wood: the verge's grass grows on the
    // band under it, and the field's own right up to that.
    var wood = max(site.w, forestVergeInside(fell, site.w));
    var line = vec2f(0.0, -1e9);
    if (wood > -((*P).clear.x + GROUND_GRASS_EDGE_M)) {
      line = groundLineBand(fell, cell, site.w);
      if (line.x > 0.5) { wood = -1e9; }
    }
    let margin = min(-wood, -water) - (*P).clear.x;
    if (margin < 0.0) { continue; }
    // Bare on the wet bank round water, thickening across the earth behind it.
    let shore = groundShore(fell, footprint, water).z;
    if (shore >= 1.0) { continue; }
    let worn = groundShoulder(fell, footprint, paved);
    // Bare on a road's own surface and at the foot of its shoulder, but for
    // a track's centre strip.
    if (worn.x > GRASS_STAND_BARE_WEAR) { continue; }
    let edge = smoothstep(0.0, GROUND_GRASS_EDGE_M, min(margin, abs(site.z))) * (1.0 - worn.x);
    var grows = min(u32(plots[i32(site.x)].detail.y), 14u);
    if (groundVerge(site, footprint) > 0.5 || line.y > 0.0) { grows = 15u; }
    let g = (*P).growth[grows];
    let stand = (*P).patches[grows];
    // A drilled crop keeps to its plot's rows, the bright bands between the
    // furrows the ground is painted with: the clump moves across them toward
    // the nearest, but never out of its plot or into a bare margin.
    let plot = plots[i32(site.x)];
    var p = fell;
    if (g.w > 0.0 && plot.rows.z > 0.0) {
      let across = dot(fell, plot.rows.xy) / plot.rows.z;
      let toRow = (floor(across) + 0.5 - across) * plot.rows.z * g.w;
      let room = max(0.0, min(site.y - terrainParams.verge.w, margin) - GRASS_PLACEMENT_HEADLAND_M);
      p = fell + plot.rows.xy * clamp(toRow, -room, room);
    }
    // A wheeling stays bare.
    if (groundWheeling(p, 0.0, i32(site.x)) > 0.5) { continue; }
    let root = vec3f(p, grassGround(p));
    // The side's learned scars: craters, scorch and tracks leave clumps out.
    let scar = groundScars(p, footprint);
    let S = scarParams.look.grass;
    let bare = max(max(scar.weights.x, scar.weights.y), scar.weights.z * S.y) * S.x;
    // Sparse patches: the stand thins there and nowhere thickens.
    let sparse = 1.0 - smoothstep(GRASS_STAND_SPARSE_FROM, GRASS_STAND_SPARSE_TO, valueNoise(p * (*P).patchScales.y + vec2f(41.7, 13.1)));
    let keep = rho * g.x * (1.0 - stand.z * sparse) * mix(GRASS_STAND_EDGE_DENSITY, 1.0, edge) * (1.0 - bare) * (1.0 - shore) * groundShoulderGrass(worn);
    if (rank >= keep) { continue; }
    if (grassUnderProp(p)) { continue; }
    // A clump nearing its rank's threshold is small: it grows as the
    // density passes it, and shrinks away as the fade takes it.
    let grow = 1.0 - smoothstep(GRASS_STAND_GROW_FROM * keep, keep, rank);
    let fade = 1.0 - smoothstep((*P).density.z, (*P).density.w, footprint);
    // Which of the row's grasses this clump is: its hash against their
    // shares, a drifting grass weighted up inside its drifts and down to
    // nothing outside them.
    var shares = array<f32, 4>();
    var total = 0.0;
    let mixed = u32(g.z);
    for (var i = 0u; i < mixed; i++) {
      let entry = (*P).mixes[grows * 4u + i];
      var share = entry.y;
      if (entry.z > 0.0) {
        let drift = smoothstep(GRASS_MIX_DRIFT_FROM, GRASS_MIX_DRIFT_TO, valueNoise(p * (*P).patchScales.w + vec2f(f32(i) * 17.3 + 3.1, f32(grows) * 7.9)));
        share *= mix(1.0, GRASS_MIX_DRIFT_GATHER * drift, entry.z);
      }
      shares[i] = share;
      total += share;
    }
    var chosen = 0u;
    if (total > 0.0) {
      chosen = mixed - 1u;
      var roll = fract(h.x * 91.37) * total;
      for (var i = 0u; i + 1u < mixed; i++) {
        if (roll < shares[i]) { chosen = i; break; }
        roll -= shares[i];
      }
    }
    let grass = (*P).mixes[grows * 4u + chosen];
    let kind = u32(grass.x);
    let row = (*P).kinds[kind];
    // Patches a few metres across stand taller or lower, as uneven grass does.
    let tall = valueNoise(p * (*P).patchScales.x);
    let height = row.x * g.y * grow * fade * mix(GRASS_HEIGHT_MIN, GRASS_HEIGHT_MAX, h.x) * mix(stand.x, stand.y, tall) * mix(GRASS_STAND_EDGE_HEIGHT, 1.0, edge);
    if (height < GRASS_STAND_LEAST_HEIGHT_M) { continue; }
    let centre = root + vec3f(0.0, 0.0, height * 0.5);
    let radius = height * 0.8 + 0.4;
    var shown = true;
    for (var i = 0u; i < 4u; i++) {
      let pl = (*P).planes[i];
      if (dot(pl.xyz, centre) + pl.w < -radius) { shown = false; }
    }
    if (!shown) { continue; }
    var grassTier = 1u;
    if (height / footprint > (*P).tiers.x * mix(GRASS_STAND_TIER_JITTER_FROM, GRASS_STAND_TIER_JITTER_TO, h.y)) { grassTier = 0u; }
    let slot = first[grassTier] + found[grassTier];
    found[grassTier] += 1u;
    let cap = select((*P).grid.w, (*P).grid.z, grassTier == 0u);
    if (!emit || slot >= cap) { continue; }
    let width = max(1.0, (*P).tiers.y * footprint / max(row.z, 1e-4));
    // Dry patches, and a grass drier than its neighbours: toward straw at
    // the ground's own luminance, so no patch is darker than its field.
    let parched = smoothstep(GRASS_STAND_PARCHED_FROM, GRASS_STAND_PARCHED_TO, valueNoise(p * (*P).patchScales.z + vec2f(9.3, 77.1)));
    // What sets a clump apart from the ground under it (how dry it is, its
    // grain) goes with the clump as the fade takes it, so the field's far
    // edge meets the painted ground with the ground's own colour.
    let dryness = min(1.0, stand.w * parched + grass.w) * fade;
    // Each clump a little lighter or darker than the next, half on its own
    // and half with the tussock beneath it: the grain a field keeps when
    // it is too far to show blades. Dry grass is paler, never darker.
    let tussock = valueNoise(p * (*P).grain.x + vec2f(63.9, 27.4));
    let value = 1.0 + (*P).shading.z * fade * (fract(h.y * 57.31) + tussock - 1.0);
    let colour = value * (1.0 + (*P).grain.y * dryness) * groundTint(scarredSurface(groundColour(p, footprint, site, paved, water), scar).xyz, dryness);
    // Tracks and trampling lay the clump over (carried by the colour's alpha).
    let flat = max(scar.weights.z, scar.weights.w) * S.z;
    let base = select((*P).grid.z, 0u, grassTier == 0u);
    clumps[base + slot] = GrassClump(
      root, height, pack4x8unorm(vec4f(sqrt(max(colour, vec3f(0.0))), 1.0 - flat)), kind,
      sin(h.z * 6.2831853), cos(h.z * 6.2831853), width);
  }
  return found;
}
