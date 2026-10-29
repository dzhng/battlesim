// WGSL owner: passes_models.

// @block passes_models_buildFn
@compute @workgroup_size(64) fn buildFn(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) li: u32) {
  let P = &params;
  if (wg.x >= (*P).grid.x || wg.y >= (*P).grid.y) { return; }
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
  let lo2 = lo - vec2f(0.4 + (*P).clear.y);
  let hi2 = hi + vec2f(0.4 + (*P).clear.y);
  for (var i = 0u; i < 4u; i++) {
    let pl = (*P).planes[i];
    let pv = vec3f(select(lo2.x, hi2.x, pl.x >= 0.0), select(lo2.y, hi2.y, pl.y >= 0.0), select(zlo, zhi, pl.z >= 0.0));
    if (dot(pl.xyz, pv) + pl.w < 0.0) { return; }
  }
  let eye = (*P).eye.xyz;
  let nearest = clamp(eye, vec3f(lo2, zlo), vec3f(hi2, zhi));
  let dNear = max(distance(eye, nearest), 0.5);
  if ((*P).eye.w * dNear >= (*P).density.w) { return; }
  let n = u32(ceil(T * T * grassDensity(dNear, max(eye.z - zlo, 0.0))));
  // The tile's own offset of the sequence, from its place on the world grid.
  let cell = vec2i(floor(lo / T + 0.5));
  let offset = grassHash(bitcast<u32>(cell.x), bitcast<u32>(cell.y)).xy;
  for (var j = li; j < n; j += 64u) {
    // The sequence's point, jittered by a fixed amount so a sparse field
    // shows no lattice; both depend on the tile and j alone.
    let h = grassHash(bitcast<u32>(cell.x) ^ (j * 2654435761u), bitcast<u32>(cell.y) + j);
    let jitter = (vec2f(fract(h.z * 7.13), fract(h.x * 5.31)) - 0.5) * 0.8;
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
    if (wood > -((*P).clear.x + 1.2)) {
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
    if (worn.x > 0.9) { continue; }
    let edge = smoothstep(0.0, 1.2, min(margin, abs(site.z))) * (1.0 - worn.x);
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
      let room = max(0.0, min(site.y - params_1.verge.w, margin) - 0.3);
      p = fell + plot.rows.xy * clamp(toRow, -room, room);
    }
    // A wheeling stays bare.
    if (groundWheeling(p, 0.0, i32(site.x)) > 0.5) { continue; }
    let root = vec3f(p, grassGround(p));
    // The side's learned scars: craters, scorch and tracks leave clumps out.
    let scar = groundScars(p, footprint);
    let S = scarParams.grass;
    let bare = max(max(scar.weights.x, scar.weights.y), scar.weights.z * S.y) * S.x;
    // Sparse patches: the stand thins there and nowhere thickens.
    let sparse = 1.0 - smoothstep(0.3, 0.55, valueNoise(p * (*P).patchScales.y + vec2f(41.7, 13.1)));
    let keep = rho * g.x * (1.0 - stand.z * sparse) * mix(0.5, 1.0, edge) * (1.0 - bare) * (1.0 - shore) * groundShoulderGrass(worn);
    if (rank >= keep) { continue; }
    if (grassUnderProp(p)) { continue; }
    // A clump nearing its rank's threshold is small: it grows as the
    // density passes it, and shrinks away as the fade takes it.
    let grow = 1.0 - smoothstep(0.7 * keep, keep, rank);
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
        let drift = smoothstep(0.4, 0.65, valueNoise(p * (*P).patchScales.w + vec2f(f32(i) * 17.3 + 3.1, f32(grows) * 7.9)));
        share *= mix(1.0, 3 * drift, entry.z);
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
    let height = row.x * g.y * grow * fade * mix(0.8, 1.2, h.x) * mix(stand.x, stand.y, tall) * mix(0.35, 1.0, edge);
    if (height < 0.02) { continue; }
    let centre = root + vec3f(0.0, 0.0, height * 0.5);
    let radius = height * 0.8 + 0.4;
    var shown = true;
    for (var i = 0u; i < 4u; i++) {
      let pl = (*P).planes[i];
      if (dot(pl.xyz, centre) + pl.w < -radius) { shown = false; }
    }
    if (!shown) { continue; }
    var tier = 1u;
    if (height / footprint > (*P).tiers.x * mix(0.85, 1.15, h.y)) { tier = 0u; }
    let width = max(1.0, (*P).tiers.y * footprint / max(row.z, 1e-4));
    // Dry patches, and a grass drier than its neighbours: toward straw at
    // the ground's own luminance, so no patch is darker than its field.
    let parched = smoothstep(0.55, 0.8, valueNoise(p * (*P).patchScales.z + vec2f(9.3, 77.1)));
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
    let slot = atomicAdd(&args[tier * 5u + 1u], 1u);
    let cap = select((*P).grid.w, (*P).grid.z, tier == 0u);
    if (slot >= cap) { continue; }
    let base = select((*P).grid.z, 0u, tier == 0u);
    clumps[base + slot] = GrassClump(
      root, height, pack4x8unorm(vec4f(sqrt(max(colour, vec3f(0.0))), 1.0 - flat)), kind,
      sin(h.z * 6.2831853), cos(h.z * 6.2831853), width);
  }
}

// @block passes_models_cardVertex
@vertex fn cardVertex(@builtin(vertex_index) vertex: u32, @location(0) placement: vec4f, @location(2) tint: vec4f, @location(3) scale: vec4f) -> cardVertex_Output {
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
  return cardVertex_Output((cam.viewProj * vec4f(world, 1f)), world, vec2f((((column + 0.5f) + (0.5f * u)) / 8f), (((row + 0.5f) - (0.5f * w)) / 2f)), layer, yaw, scale.xyz, tint.xyz, placement.xyz);
}

// @block passes_models_impostor
@fragment fn impostor(_arg_0: impostor_Input) -> impostor_Output {
  let surface_1 = modelSurface(_arg_0.color, _arg_0.material, _arg_0.track, _arg_0.uv, normalize(_arg_0.normal), _arg_0.tangent);
  let albedo_1 = (surface_1.albedo * surface_1.occlusion);
  return impostor_Output(vec4f(pow(max(albedo_1, vec3f()), vec3f(0.4545454680919647)), 1f), vec4f(((surface_1.normal * 0.5f) + vec3f(0.5)), 1f), vec4f(surface_1.tint, 0f, 0f, 1f));
}

// @block passes_models_item_alternate
@fragment fn item(_arg_0: _Input, @builtin(position) clip: vec4f) -> item_1 {
  let eye = (&cam.eye);
  let view = normalize(((*eye) - _arg_0.world));
  var n = normalize(_arg_0.normal);
  if ((dot(n, view) < 0f)) {
    n = -(n);
  }
  let surface_1 = modelSurface(_arg_0.color, _arg_0.material, _arg_0.track, _arg_0.uv, n, _arg_0.tangent);
  let albedo_1 = (surface_1.albedo * mix(vec3f(1), _arg_0.tint, surface_1.tint));
  let sun_1 = sampleSunShadow(_arg_0.world, n, clip.xy);
  let shaded = shade(albedo_1, vec3f(), surface_1.roughness, 0f, surface_1.metallic, surface_1.occlusion, normalize((surface_1.normal + (view * 0.5f))), _arg_0.world, sun_1, (*eye));
  let ceiling = (unlit(vec3f(1), _arg_0.world, (*eye)).xyz * 1f);
  return item_1(vec4f((min(shaded.xyz, ceiling) + highlightGlow(_arg_0.highlight)), coverageValue(_arg_0.material, _arg_0.uv)), vec4f());
}

// @block passes_models_item_primary
@fragment fn item(_arg_0: _Input, @builtin(position) clip: vec4f) -> @location(0) vec4f {
  let coverage = coverageValue(_arg_0.material, _arg_0.uv);
  let cutoff = materials[((_arg_0.material * 5u) + 4u)].x;
  if (((cutoff > 0f) && (coverage < cutoff))) {
    discard;;
  }
  let eye = (&cam.eye);
  var n = normalize(_arg_0.normal);
  if ((dot(n, ((*eye) - _arg_0.world)) < 0f)) {
    n = -(n);
  }
  let surface_1 = modelSurface(_arg_0.color, _arg_0.material, _arg_0.track, _arg_0.uv, n, _arg_0.tangent);
  let sun_1 = sampleSunShadow(_arg_0.world, n, clip.xy);
  let shaded = shade(_arg_0.xray.xyz, vec3f(), surface_1.roughness, 0f, surface_1.metallic, surface_1.occlusion, surface_1.normal, _arg_0.world, sun_1, (*eye));
  return vec4f(shaded.xyz, (_arg_0.xray.w * coverage));
}

// @block passes_models_modelCutoutCaster
@fragment fn modelCutoutCaster(_arg_0: modelCutoutCaster_Input, @builtin(position) clip: vec4f) -> @builtin(sample_mask) u32 {
  return u32((cutoutCoverage(_arg_0.material, _arg_0.uv) > pixelNoise(clip.xy)));
}

// @block passes_models_modelCutoutDepth
@fragment fn modelCutoutDepth(_arg_0: modelCutoutDepth_Input, @builtin(position) clip: vec4f) -> @builtin(sample_mask) u32 {
  let coverage = cutoutCoverage(_arg_0.material, _arg_0.uv);
  return coveredSamples(coverage, fwidth(coverage), pixelNoise(clip.xy));
}

// @block passes_models_modelVertex
@vertex fn modelVertex(@location(0) position: vec3f, @location(1) normal: vec4f, @location(2) uv: vec2f, @location(3) color: vec4f, @location(4) joints: vec4u, @location(5) weights: vec4f, @location(6) tangent: vec4f, @location(7) material: vec2u, @builtin(instance_index) instance: u32, @location(8) placement: vec4f, @location(9) data: vec4f, @location(10) tint: vec4f, @location(11) scale: vec4f) -> modelVertex_Output {
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
  let normal_1 = vec3f(((localNormal.x * c) - (localNormal.y * s)), ((localNormal.x * s) + (localNormal.y * c)), localNormal.z);
  let tangent_1 = vec4f(((localTangent.x * c) - (localTangent.y * s)), ((localTangent.x * s) + (localTangent.y * c)), localTangent.z, tangent.w);
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
  return modelVertex_Output((cam.viewProj * vec4f(world, 1f)), world, normal_1, tangent_1, color, vec2f((uv.x - scroll), uv.y), material.x, track, tint.xyz, max(0f, (-1f - tint.w)), placement.xyz, vec4f((red / 255f), (green / 255f), (blue / 255f), scale.w), instance);
}

// @block passes_models_modelXrayCountFragment
@fragment fn modelXrayCountFragment(_arg_0: modelXrayCountFragment_Input, @builtin(position) clip: vec4f) -> @location(0) vec4f {
  return countCoverage(clip, _arg_0.instance, _arg_0.xray);
}

// @block passes_models_modelXrayFragment
@fragment fn modelXrayFragment(_arg_0: modelXrayFragment_Input) -> @location(0) vec4f {
  return drawCoverage(_arg_0.instance, _arg_0.xray);
}
