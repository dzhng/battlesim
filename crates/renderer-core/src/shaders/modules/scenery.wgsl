// WGSL owner: scenery.

// @block scenery_scenerySurface
fn scenerySurface(world: vec3f, normal: vec3f, color: vec4f, local: vec3f, sunlight: f32) -> vec3f {
  let eye = (&cam.eye);
  var n = normalize(normal);
  var leafShade = 1f;
  var roughness = 0.8999999761581421f;
  let footprint = length(fwidth(world));
  if (((color.w > 0.5f) && (footprint < LEAF_FADE_M.y))) {
    let leaves = leafCells(local, n, footprint);
    n = leaves.xyz;
    leafShade = leaves.w;
    roughness = 0.85f;
  }
  let lit = shade((color.xyz * leafShade), vec3f(), roughness, 0f, 0f, 1f, n, world, sunlight, (*eye));
  return lit.xyz;
}

// @block scenery_leafCells
// Leaves finer than a crown's triangles, painted on them: the cells of a
// jittered lattice in the tree's own space, each a round leaf about its
// centre, domed, with its own tilt and tone, shaded where it meets the next
// and in the gap past its edge (LEAF_ROUND). A cell is a few pixels across:
// LEAF_CELL_M near, doubling with distance into the clumps a far crown reads
// as (blended into the next size over the last LEAF_BLEND of each, so none
// pops), and gone where even those shrink to a pixel's footprint
// (LEAF_FADE_M, of `fwidth`'s length).
const LEAF_CELL_M = 0.08;
const LEAF_CELL_PX = 3.0;
const LEAF_FADE_M = vec2f(0.3, 0.45);
const LEAF_TILT = 0.5;
const LEAF_TONE = 0.2;
const LEAF_RIM = 0.25;
const LEAF_RIM_SHADE = 0.7;
const LEAF_STRAY = 0.8;
const LEAF_BLEND = 0.3;
const LEAF_ROUND = 0.6;
const LEAF_DOME = 0.6;
const LEAF_GAP_SHADE = 0.7;

// A cell's leaf: its tilt and its shade at `q`, in cells, `soft` the cells a
// pixel spans. Only the eight cells round `q` are searched, so a leaf's
// centre strays at most LEAF_STRAY from its cell's.
fn leafCell(q: vec3f, soft: f32) -> vec4f {
  let base = floor(q - vec3f(0.5));
  var near = 8.0;
  var next = 8.0;
  var id = 0u;
  var at = vec3f();
  for (var k = 0u; k < 8u; k++) {
    let c = base + vec3f(f32(k & 1u), f32((k >> 1u) & 1u), f32(k >> 2u));
    let i = bitcast<vec3u>(vec3i(c));
    let h = pcgHash((i.x * 1597334677u) ^ (i.y * 3812015801u) ^ (i.z * 2798796415u));
    let stray = vec3f(f32(h & 1023u), f32((h >> 10u) & 1023u), f32((h >> 20u) & 1023u)) / 1023.0 - vec3f(0.5);
    let centre = c + vec3f(0.5) + stray * LEAF_STRAY;
    let d = length(centre - q);
    if (d < near) {
      next = near;
      near = d;
      id = h;
      at = centre;
    } else if (d < next) {
      next = d;
    }
  }
  let g = pcgHash(id);
  let tilt = vec3f(f32(g & 1023u), f32((g >> 10u) & 1023u), f32((g >> 20u) & 1023u)) / 511.5 - vec3f(1.0);
  let tone = 1.0 + LEAF_TONE * (f32((g * 747796405u) >> 22u) / 511.5 - 1.0);
  let rim = mix(LEAF_RIM_SHADE, 1.0, smoothstep(0.0, LEAF_RIM + soft, next - near));
  let edge = mix(1.0, LEAF_GAP_SHADE, smoothstep(LEAF_ROUND - soft, LEAF_ROUND + soft, near));
  let dome = (q - at) * (LEAF_DOME / LEAF_ROUND);
  return vec4f(tilt + dome, tone * rim * edge);
}

fn leafCells(p: vec3f, n: vec3f, footprint: f32) -> vec4f {
  let size = max(footprint * LEAF_CELL_PX, LEAF_CELL_M);
  let octave = log2(size / LEAF_CELL_M);
  let fine = LEAF_CELL_M * exp2(floor(octave));
  var cell = leafCell(p / fine, footprint / fine);
  let blend = smoothstep(1.0 - LEAF_BLEND, 1.0, fract(octave));
  if (blend > 0.0) {
    cell = mix(cell, leafCell(p / (fine * 2.0), footprint / (fine * 2.0)), blend);
  }
  let fade = 1.0 - smoothstep(LEAF_FADE_M.x, LEAF_FADE_M.y, footprint);
  return vec4f(normalize(n + cell.xyz * (LEAF_TILT * fade)), mix(1.0, cell.w, fade));
}

// @block scenery_sway
fn sway(foot: vec3f, point: vec3f, up: f32, height: f32, leaf: f32) -> vec3f {
  let wind = cameraWind();
  let t = cam.time;
  let dir = wind.toward.xy;
  let strength = clamp(wind.gustMps / SWAY_FULL_GUST_MPS, 0.0, 1.0);
  let along = dot(foot.xy, dir);
  let front = sin(6.2831853 * (along - t * wind.gustMps) * wind.gustPerM);
  let gust = smoothstep(0.2, 1.0, front);
  let seed = pcgHash(bitcast<u32>(floor(foot.x * 4.0)) ^ (bitcast<u32>(floor(foot.y * 4.0)) * 2654435761u));
  let phase = f32(seed >> 8u) * (6.2831853 / 16777216.0);
  let swing = sin(t * SWAY_RATE + phase) + 0.25 * sin(1.5 * t * SWAY_RATE + 1.8 * phase);
  let h = clamp(up, 0.0, 1.0);
  let reach = h * h * h;
  let carried = SWAY_LEAN + SWAY_GUST * gust + SWAY_SWING * swing * 0.5 * (1.0 + gust);
  let bend = height * SWAY_BEND * strength * carried * reach;
  // Bent, the point keeps its distance from the foot: it drops a little.
  var offset = vec3f(dir * bend, -0.5 * bend * bend / max(height * h, 1.0));
  let q = point - foot;
  let shiver = vec3f(
    sin(t * SWAY_SHIVER_RATE + q.x * 2.1 + q.y * 0.7 + phase),
    sin(t * SWAY_SHIVER_RATE * 1.3 + q.y * 1.9 + q.z * 0.8 + phase),
    sin(t * SWAY_SHIVER_RATE * 0.9 + q.z * 2.3 + q.x * 0.6 + phase),
  );
  offset += shiver * (SWAY_SHIVER_M * strength * 0.5 * (1.0 + gust) * leaf * h);
  return offset;
}

// @block scenery_standing
fn standing(local: vec3f, pose: vec4f, shape: vec4f, leaf: f32) -> vec3f {
  let at = placedPoint(local, pose);
  let height = max((shape.w / SWAY_HEART), 1f);
  return (at + sway(pose.xyz, at, (local.z / height), height, leaf));
}

// @block scenery_placedNormal
fn placedNormal(normal: vec3f, shape: vec4f, yaw: f32) -> vec3f {
  let c = cos(yaw);
  let s = sin(yaw);
  let nx = (normal.x / shape.x);
  let ny = (normal.y / shape.y);
  return normalize(vec3f(((nx * c) - (ny * s)), ((nx * s) + (ny * c)), (normal.z / shape.z)));
}

// @block scenery_pastTrunk
fn pastTrunk(d: f32, radius: f32, k: f32) -> f32 {
  let m = abs(d);
  return sign(d) * (min(m, radius) + max(m - radius, 0.0) * k);
}
