// WGSL owner: models_material.

// @block models_material_ModelSurface
struct ModelSurface {
  albedo: vec3f,
  normal: vec3f,
  roughness: f32,
  metallic: f32,
  occlusion: f32,
  tint: f32,
}

// @block models_material_coverageValue
fn coverageValue(material: u32, uv: vec2f) -> f32 {
  let row = (material * 5u);
  let layer = materials[(row + 4u)].y;
  let image = textureSample(surface, tiled, uv, i32(max(layer, 0f)));
  var value = materials[row].w;
  if ((layer >= 0f)) {
    value = (value * image.w);
  }
  return value;
}

// @block models_material_cutoutCoverage
fn cutoutCoverage(material: u32, uv: vec2f) -> f32 {
  let cutoff = materials[((material * 5u) + 4u)].x;
  return saturate(((coverageValue(material, uv) * 0.5f) / max(cutoff, 1e-3f)));
}

// @block models_material_materials
@group(1) @binding(1) var<storage, read> materials: array<vec4f>;

// @block models_material_modelSurface
fn modelSurface(color: vec4f, material: u32, track: f32, uv: vec2f, normal: vec3f, tangent: vec4f) -> ModelSurface {
  let row = (material * 5u);
  let base = (&materials[row]);
  let factors = (&materials[(row + 1u)]);
  let layers = (&materials[(row + 2u)]);
  let worn = (&materials[(row + 3u)]);
  let a = textureSample(albedo, tiled, uv, i32(max((*layers).x, 0f)));
  let nm = textureSample(surface, tiled, uv, i32(max((*layers).y, 0f)));
  let orm = textureSample(surface, tiled, uv, i32(max((*layers).z, 0f)));
  var albedo_1 = ((color.xyz * (*factors).w) * (*base).xyz);
  var threshold = 0.5f;
  if (((*layers).x >= 0f)) {
    albedo_1 = (albedo_1 * a.xyz);
    threshold = a.w;
  }
  else {
    if (((*layers).x <= -2f)) {
      let mean = textureSampleLevel(albedo, tiled, vec2f(0.5), i32((-2f - (*layers).x)), 16);
      albedo_1 = (albedo_1 * mean.xyz);
    }
    else {
      if (((track >= 1f) && ((track - 1f) < 0.5f))) {
        albedo_1 = (albedo_1 * 0.55f);
      }
    }
  }
  var roughness = (*factors).y;
  var metallic = (*factors).x;
  var occlusion = 1f;
  var tint = (*factors).z;
  if (((*layers).z >= 0f)) {
    occlusion = orm.x;
    roughness = (roughness * orm.y);
    metallic = (metallic * orm.z);
    tint = (tint * orm.w);
  }
  var n = normal;
  let tangentLength = length(tangent.xyz);
  if ((((*layers).y >= 0f) && (tangentLength > 0.5f))) {
    let t = normalize((tangent.xyz - (n * dot(n, tangent.xyz))));
    let b = (cross(n, t) * tangent.w);
    let m = ((nm.xyz * 2f) - vec3f(1));
    n = normalize((((t * m.x) + (b * m.y)) + (n * m.z)));
  }
  if (((*layers).w > 0f)) {
    let w = smoothstep((threshold - 0.04f), (threshold + 0.04f), color.w);
    albedo_1 = mix(albedo_1, (*worn).xyz, w);
    roughness = mix(roughness, (*worn).w, w);
    metallic = mix(metallic, 0f, w);
  }
  return ModelSurface(albedo_1, n, roughness, metallic, occlusion, tint);
}

// @block models_material_roomLookup
fn roomLookup(uv: vec2f, pick: u32, cellTexels: f32) -> vec3f {
    let over = max(max(-uv, uv - vec2f(1.0)), vec2f(0.0));
    let depth = 1.0 - max(over.x, over.y);
    let across = clamp(uv, vec2f(0.0), vec2f(1.0)) - vec2f(0.5);
    let shrink = 16.0 / (16.0 + 4.5 * depth);
    var at = vec2f(0.5) + across * shrink;
    if (((pick >> 16u) & 1u) == 1u) { at.x = 1.0 - at.x; }
    let footprint = max(length(dpdx(at)), length(dpdy(at))) * cellTexels;
    let level = clamp(log2(max(footprint, 0.000001)), 0.0, log2(cellTexels));
    let inset = 0.5 * exp2(ceil(level)) / cellTexels;
    at = clamp(at, vec2f(inset), vec2f(1.0 - inset));
    let cell = pick % 10u;
    let corner = vec2f(f32(cell % 4u), f32(cell / 4u));
    return vec3f((corner + at) / 4.0, level);
  }

// @block models_material_roomPick
fn roomPick(anchor: vec3f) -> u32 {
    let bits = bitcast<vec3u>(anchor);
    var h = 0u;
    for (var i = 0; i < 3; i++) {
      // PCG's output permutation, chained over the three coordinates.
      let state = (h ^ bits[i]) * 747796405u + 2891336453u;
      let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
      h = (word >> 22u) ^ word;
    }
    return h;
  }

// @block models_material_surface
@group(1) @binding(3) var surface: texture_2d_array<f32>;

// @block models_material_albedo_modelLayout
@group(1) @binding(2) var albedo: texture_2d_array<f32>;

// @block models_material_tiled
@group(1) @binding(4) var tiled: sampler;
