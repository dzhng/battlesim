// WGSL owner: passes_models.

// @block passes_models_modelRoomsFragment
@fragment fn modelRoomsFragment(input: ModelFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let row = (input.material * 5u);
  let layer = materials[(row + 2u)].x;
  let cellTexels = (f32(textureDimensions(albedo).x) / 4f);
  let at = roomLookup(input.uv, roomPick(input.anchor), cellTexels);
  let picture = textureSampleLevel(albedo, tiled, at.xy, i32(max(layer, 0f)), at.z);
  var colour = materials[row].xyz;
  if ((layer >= 0f)) {
    colour = picture.xyz;
  }
  colour = (colour * mix(vec3f(1), input.tint, materials[(row + 1u)].z));
  let eye = (&cam.eye);
  var n = normalize(input.normal);
  if ((dot(n, ((*eye) - input.world)) < 0f)) {
    n = -(n);
  }
  let seen = modelSeen(input.world, n, input.anchor, clip.xy);
  return ColorFog(vec4f((unlit(colour, input.world, (*eye)).xyz + highlightGlow(input.highlight)), 1f), fogCoverage(seen, 1f));
}

// @block passes_models_modelCardsFragment
@fragment fn modelCardsFragment(input: CardFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let baseColor = textureSample(albedo, materialSampler, input.uv, input.layer);
  let packed = textureSample(normal, materialSampler, input.uv, input.layer);
  if ((baseColor.w < 0.5f)) {
    discard;;
  }
  let local = ((packed.xyz * 2f) - vec3f(1));
  var n = cardNormal(local, input.scale, input.yaw);
  let eye = (&cam.eye);
  if ((dot(n, ((*eye) - input.world)) < 0f)) {
    n = -(n);
  }
  let linearColor = pow(max(baseColor.xyz, vec3f()), vec3f(2.200000047683716));
  let tinted = (linearColor * mix(vec3f(1), input.tint, packed.w));
  let sunlight = sampleSunShadow(input.world, n, clip.xy);
  let shaded = shade(tinted, vec3f(), 0.85f, 0f, 0f, 1f, n, input.world, sunlight, (*eye));
  let seen = modelSeen(input.world, n, input.anchor, clip.xy);
  return ColorFog(vec4f(shaded.xyz, 1f), fogCoverage(seen, 1f));
}

// @block passes_models_modelFragment
@fragment fn modelFragment(input: ModelFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let eye = (&cam.eye);
  var n = normalize(input.normal);
  if ((dot(n, ((*eye) - input.world)) < 0f)) {
    n = -(n);
  }
  let materialSample = modelSurface(input.color, input.material, input.track, input.uv, n, input.tangent);
  let baseColor = (materialSample.albedo * mix(vec3f(1), input.tint, materialSample.tint));
  let paintSample = (groundPaint(input.world) * smoothstep(0.5f, 0.8f, n.z));
  let sunlight = sampleSunShadow(input.world, n, clip.xy);
  let shaded = shade(paintedAlbedo(baseColor, paintSample), vec3f(), materialSample.roughness, 0f, materialSample.metallic, materialSample.occlusion, materialSample.normal, input.world, sunlight, (*eye));
  let seen = paintedSeen(modelSeen(input.world, n, input.anchor, clip.xy), paintSample);
  return ColorFog(vec4f(((shaded.xyz + paintGlow(paintSample)) + highlightGlow(input.highlight)), 1f), fogCoverage(seen, 1f));
}

// @block passes_models_CardFragmentInput
struct CardFragmentInput {
  @location(0) world: vec3f,
  @location(1) uv: vec2f,
  @location(2) @interpolate(flat) layer: u32,
  @location(3) @interpolate(flat) yaw: f32,
  @location(4) @interpolate(flat) scale: vec3f,
  @location(5) tint: vec3f,
  @location(6) anchor: vec3f,
}

// @block passes_models_CardVertexOutput
struct CardVertexOutput {
  @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) uv: vec2f,
  @location(2) @interpolate(flat) layer: u32,
  @location(3) @interpolate(flat) yaw: f32,
  @location(4) @interpolate(flat) scale: vec3f,
  @location(5) tint: vec3f,
  @location(6) anchor: vec3f,
}

// @block passes_models_ImpostorBuffers
struct ImpostorBuffers {
  @location(0) albedo: vec4f,
  @location(1) normal: vec4f,
  @location(2) mask: vec4f,
}

// @block passes_models_ModelVertexOutput
struct ModelVertexOutput {
  @invariant @builtin(position) clip: vec4f,
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) tangent: vec4f,
  @location(3) color: vec4f,
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) material: u32,
  @location(6) track: f32,
  @location(7) tint: vec3f,
  @location(8) @interpolate(flat) highlight: f32,
  @location(9) @interpolate(flat) anchor: vec3f,
  @location(10) @interpolate(flat) xray: vec4f,
  @location(11) @interpolate(flat) instance: u32,
}

// @block passes_models_ModelFragmentInput
struct ModelFragmentInput {
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) tangent: vec4f,
  @location(3) color: vec4f,
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) material: u32,
  @location(6) track: f32,
  @location(7) tint: vec3f,
  @location(8) @interpolate(flat) highlight: f32,
  @location(9) @interpolate(flat) anchor: vec3f,
  @location(10) @interpolate(flat) xray: vec4f,
  @location(11) @interpolate(flat) instance: u32,
}

// @block passes_models_CutoutFragmentInput
struct CutoutFragmentInput {
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) material: u32,
}

// @block passes_models_cardVertex
@vertex fn cardVertex(@builtin(vertex_index) vertex: u32, @location(0) placement: vec4f, @location(2) tint: vec4f, @location(3) scale: vec4f) -> CardVertexOutput {
  let layer = u32(tint.w);
  let frame = (&frames[layer]);
  let yaw = placement.w;
  let center = cardPoint((*frame).xyz, scale.xyz, yaw, placement.xyz);
  let toEye = cardDirection((cam.eye - center), scale.xyz, yaw);
  let heading = atan2(toEye.y, toEye.x);
  var column = round((heading / 0.7853981633974483f));
  column = (column - (8f * floor((column / 8f))));
  let elevation = atan2(toEye.z, length(vec2f(toEye.x, toEye.y)));
  var row = 1f;
  var pitch = 0.5f;
  if ((elevation > 0.675f)) {
    row = 0f;
    pitch = 0.85f;
  }
  let cellYaw = (column * 0.7853981633974483f);
  let cp = cos(pitch);
  let toward = vec3f((cp * cos(cellYaw)), (cp * sin(cellYaw)), sin(pitch));
  let right = normalize(cross(vec3f(0, 0, 1), toward));
  let up = cross(toward, right);
  var k = vertex;
  if ((k == 3u)) {
    k = 0u;
  }
  if ((k > 3u)) {
    k = (k - 2u);
  }
  var u = -1f;
  if (((k == 1u) || (k == 2u))) {
    u = 1f;
  }
  var w = -1f;
  if ((k >= 2u)) {
    w = 1f;
  }
  let local = ((*frame).xyz + (((right * u) + (up * w)) * (*frame).w));
  let world = cardPoint(local, scale.xyz, yaw, placement.xyz);
  return CardVertexOutput((cam.viewProj * vec4f(world, 1f)), world, vec2f((((column + 0.5f) + (0.5f * u)) / 8f), (((row + 0.5f) - (0.5f * w)) / 2f)), layer, yaw, scale.xyz, tint.xyz, placement.xyz);
}

// @block passes_models_impostor
@fragment fn impostor(input: ModelFragmentInput) -> ImpostorBuffers {
  let materialSample = modelSurface(input.color, input.material, input.track, input.uv, normalize(input.normal), input.tangent);
  let baseColor = (materialSample.albedo * materialSample.occlusion);
  return ImpostorBuffers(vec4f(pow(max(baseColor, vec3f()), vec3f(0.4545454680919647)), 1f), vec4f(((materialSample.normal * 0.5f) + vec3f(0.5)), 1f), vec4f(materialSample.tint, 0f, 0f, 1f));
}

// @block passes_models_modelBlendedFragment
const GLASS_TURN = 0.5f;
const GLASS_GLINT = 1f;
@fragment fn modelBlendedFragment(input: ModelFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let eye = (&cam.eye);
  let view = normalize(((*eye) - input.world));
  var n = normalize(input.normal);
  if ((dot(n, view) < 0f)) {
    n = -(n);
  }
  let materialSample = modelSurface(input.color, input.material, input.track, input.uv, n, input.tangent);
  let baseColor = (materialSample.albedo * mix(vec3f(1), input.tint, materialSample.tint));
  let sunlight = sampleSunShadow(input.world, n, clip.xy);
  let shaded = shade(baseColor, vec3f(), materialSample.roughness, 0f, materialSample.metallic, materialSample.occlusion, normalize((materialSample.normal + (view * GLASS_TURN))), input.world, sunlight, (*eye));
  let ceiling = (unlit(vec3f(1), input.world, (*eye)).xyz * GLASS_GLINT);
  return ColorFog(vec4f((min(shaded.xyz, ceiling) + highlightGlow(input.highlight)), coverageValue(input.material, input.uv)), vec4f());
}

// @block passes_models_modelGhostFragment
@fragment fn modelGhostFragment(input: ModelFragmentInput, @builtin(position) clip: vec4f) -> @location(0) vec4f {
  let coverage = coverageValue(input.material, input.uv);
  let cutoff = materials[((input.material * 5u) + 4u)].x;
  if (((cutoff > 0f) && (coverage < cutoff))) {
    discard;;
  }
  let eye = (&cam.eye);
  var n = normalize(input.normal);
  if ((dot(n, ((*eye) - input.world)) < 0f)) {
    n = -(n);
  }
  let materialSample = modelSurface(input.color, input.material, input.track, input.uv, n, input.tangent);
  let sunlight = sampleSunShadow(input.world, n, clip.xy);
  let shaded = shade(input.xray.xyz, vec3f(), materialSample.roughness, 0f, materialSample.metallic, materialSample.occlusion, materialSample.normal, input.world, sunlight, (*eye));
  return vec4f(shaded.xyz, (input.xray.w * coverage));
}

// @block passes_models_modelCutoutCaster
@fragment fn modelCutoutCaster(input: CutoutFragmentInput, @builtin(position) clip: vec4f) -> @builtin(sample_mask) u32 {
  return u32((cutoutCoverage(input.material, input.uv) > pixelNoise(clip.xy)));
}

// @block passes_models_modelCutoutDepth
@fragment fn modelCutoutDepth(input: CutoutFragmentInput, @builtin(position) clip: vec4f) -> @builtin(sample_mask) u32 {
  let coverage = cutoutCoverage(input.material, input.uv);
  return coveredSamples(coverage, fwidth(coverage), pixelNoise(clip.xy));
}

// @block passes_models_modelCutoutCasterDiscard
@fragment fn modelCutoutCaster(input: CutoutFragmentInput, @builtin(position) clip: vec4f) {
  if (!(cutoutCoverage(input.material, input.uv) > pixelNoise(clip.xy))) { discard; }
}

// @block passes_models_modelCutoutDepthDiscard
@fragment fn modelCutoutDepth(input: CutoutFragmentInput, @builtin(position) clip: vec4f) {
  let coverage = cutoutCoverage(input.material, input.uv);
  if (coverage <= 0.5f) { discard; }
}

// @block passes_models_modelVertex
@vertex fn modelVertex(@location(0) position: vec3f, @location(1) normal: vec4f, @location(2) uv: vec2f, @location(3) color: vec4f, @location(4) joints: vec4u, @location(5) weights: vec4f, @location(6) tangent: vec4f, @location(7) material: vec2u, @builtin(instance_index) instance: u32, @location(8) placement: vec4f, @location(9) data: vec4f, @location(10) tint: vec4f, @location(11) scale: vec4f) -> ModelVertexOutput {
  let base = u32(data.x);
  let p = vec4f(position, 1f);
  let n = vec4f(normal.xyz, 0f);
  let m0 = (&palette[(base + joints.x)]);
  let m1 = (&palette[(base + joints.y)]);
  let m2 = (&palette[(base + joints.z)]);
  let m3 = (&palette[(base + joints.w)]);
  let skinned = (((moved((*m0), p) * weights.x) + (moved((*m1), p) * weights.y)) + ((moved((*m2), p) * weights.z) + (moved((*m3), p) * weights.w)));
  let skinnedNormal = (((moved((*m0), n) * weights.x) + (moved((*m1), n) * weights.y)) + ((moved((*m2), n) * weights.z) + (moved((*m3), n) * weights.w)));
  let t = vec4f(tangent.xyz, 0f);
  let skinnedTangent = (((moved((*m0), t) * weights.x) + (moved((*m1), t) * weights.y)) + ((moved((*m2), t) * weights.z) + (moved((*m3), t) * weights.w)));
  let c = cos(placement.w);
  let s = sin(placement.w);
  let local = (skinned.xyz * scale.xyz);
  let localNormal = (skinnedNormal.xyz / scale.xyz);
  let localTangent = (skinnedTangent.xyz * scale.xyz);
  let world = vec3f((((local.x * c) - (local.y * s)) + placement.x), (((local.x * s) + (local.y * c)) + placement.y), (local.z + placement.z));
  let worldNormal = vec3f(((localNormal.x * c) - (localNormal.y * s)), ((localNormal.x * s) + (localNormal.y * c)), localNormal.z);
  let worldTangent = vec4f(((localTangent.x * c) - (localTangent.y * s)), ((localTangent.x * s) + (localTangent.y * c)), localTangent.z, tangent.w);
  var scroll = 0f;
  var track = 0f;
  if ((material.y == 1u)) {
    scroll = data.y;
    track = (1f + fract((uv.x - scroll)));
  }
  if ((material.y == 2u)) {
    scroll = data.z;
    track = (1f + fract((uv.x - scroll)));
  }
  let packed = data.w;
  let red = floor((packed / 65536f));
  let green = (floor((packed / 256f)) - (red * 256f));
  let blue = (packed - (floor((packed / 256f)) * 256f));
  return ModelVertexOutput((cam.viewProj * vec4f(world, 1f)), world, worldNormal, worldTangent, color, vec2f((uv.x - scroll), uv.y), material.x, track, tint.xyz, max(0f, (-1f - tint.w)), placement.xyz, vec4f((red / 255f), (green / 255f), (blue / 255f), scale.w), instance);
}

// @block passes_models_modelXrayCountFragment
@fragment fn modelXrayCountFragment(input: ModelFragmentInput, @builtin(position) clip: vec4f) -> @location(0) vec4f {
  return countCoverage(clip, input.instance, input.xray);
}

// @block passes_models_modelXrayFragment
@fragment fn modelXrayFragment(input: ModelFragmentInput) -> @location(0) vec4f {
  return drawCoverage(input.instance, input.xray);
}
