// WGSL owner: passes_fog.

// @block passes_fog_backdropFragment_ground
@fragment fn backdropFragment(input: MeshFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let eye = (&cam.eye);
  let cell = groundCell(input.world.xy, length(fwidth(input.world.xy)));
  if (groundClassView()) {
    return ColorFog(vec4f(0, 0, 0, 1), vec4f(0, 0, 0, 1));
  }
  let surface = groundAlbedo(input.world, input.color, cell, groundPaved(input.world.xy, cell));
  let up = normalize(input.normal);
  let paintSample = groundPaint(input.world);
  let albedo = paintedAlbedo(surface.xyz, paintSample);
  let lit = shade(albedo, vec3f(), surface.w, 0f, 0f, 1f, up, input.world, 1f, (*eye));
  let seen = fogTerm(input.world, up, clip.xy, fogIsGround());
  return ColorFog(vec4f((lit.xyz + paintGlow(paintSample)), 1f), fogCoverage(paintedSeen(seen, paintSample), 1f));
}

// @block passes_fog_backdropFragment_foliage
@fragment fn backdropFragment(input: MeshFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let seen = fogTerm(input.heart, vec3f(), clip.xy, false);
  return ColorFog(vec4f(surface(input.world, input.normal, input.color, input.local, 1f), 1f), fogCoverage(seen, 1f));
}

// @block passes_fog_forestFragment
@fragment fn forestFragment(input: FoliageFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let n = normalize(input.normal);
  let seen = fogTerm(input.heart, vec3f(), clip.xy, false);
  let sunlight = sampleSunShadow(input.world, n, clip.xy);
  let lit = surface(input.world, input.normal, input.color, input.local, sunlight);
  return ColorFog(vec4f(lit, 1f), fogCoverage(seen, 1f));
}

// @block passes_fog_fragment
const GRASS_BLADE_ROUGHNESS = 0.9f;
@fragment fn fragment(input: GrassFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let eye = (&cam.eye);
  let up = vec3f(0, 0, 1);
  let seen = fogTerm(vec3f(input.world.x, input.world.y, input.root.z), up, clip.xy, fogIsGround());
  var n = normalize(input.normal);
  if ((dot(n, ((*eye) - input.world)) < 0f)) {
    n = -(n);
  }
  let shading = normalize(mix(n, up, input.ground));
  let sunlight = sampleSunShadow(input.world, up, clip.xy);
  let under = groundPaint(vec3f(input.world.x, input.world.y, input.root.z));
  let here = groundPaintAtPixel(clip.xy);
  let lit = shade(input.albedo, vec3f(), GRASS_BLADE_ROUGHNESS, 0f, 0f, 1f, shading, input.world, sunlight, (*eye));
  return ColorFog(vec4f((lit.xyz + max(grassPaintGlow(under, (input.world.z - input.root.z)), grassPaintGlow(here, 0f))), 1f), fogCoverage(seen, 1f));
}

// @block passes_fog_fs
@fragment fn fs(f: SpriteVertexOutput) -> ColorFog {
  // Derivatives first, in uniform control flow.
  let tuv = clamp(vec2f(f.uv.x, -f.uv.y) * 0.5 + 0.5, vec2f(0.004), vec2f(0.996));
  let gx = dpdx(tuv);
  let gy = dpdy(tuv);
  let offsetWidth = fwidth(f.offset);
  let shape = u32(f.misc.x);
  let raw = textureLoad(sceneDepth, vec2i(f.pos.xy), 0);
  let scene = select(1e9, cam.znear / raw, raw > 0.0);
  let gap = scene - f.viewDepth;
  // A ground shadow finds its own surface, in front of its quad or behind.
  if (gap < 0.0 && shape != 4u) {
    discard;
  }
  var rgb = vec3f(0.0);
  var alpha = 0.0;
  if (shape == 0u && f.color.a > 0.0) {
    // A smoke ribbon: thick through its middle, thinning to its edges.
    let across = f.uv.y * f.uv.y;
    let cover = exp(-across * 2.0) * (1.0 - across)
      * clamp(gap / max(f.extra.z, 1e-3), 0.0, 1.0);
    alpha = f.color.a * cover;
    rgb = f.color.rgb * smokeLight(f, f.offset) * alpha;
  } else if (shape == 0u) {
    let across = f.uv.y * f.uv.y;
    let a = clamp(f.along, 0.0, 1.0);
    let fromEnd = (1.0 - abs(f.uv.x)) * f.extra.x;
    let cap = smoothstep(0.0, max(f.extra.y, 1e-3) * 2.0, fromEnd);
    // A sharp line with a little bloom of its own, or a soft glow
    // falling off across its whole width.
    let sharp = exp(-across * 9.0) + 0.3 * exp(-across * 2.5);
    let soft = exp(-across * 3.0) * (1.0 - across);
    let distancePx = abs(f.uv.y) * (f.extra.w + 2.0) * 0.5;
    let lineCoverage = clamp((f.extra.w + 1.0) * 0.5 - distancePx, 0.0, 1.0);
    let profile = select(select(sharp, soft, f.misc.w == 1.0 || f.misc.w < 0.0), lineCoverage, f.extra.w > 0.0);
    rgb = f.color.rgb * profile * a * a * cap * clamp(gap / 0.1, 0.0, 1.0);
  } else if (shape == 1u && f.color.a > 0.0) {
    // A solid disc: the round seen as an object.
    let cover = (1.0 - smoothstep(0.6, 1.0, length(f.uv))) * clamp(gap / 0.1, 0.0, 1.0);
    rgb = f.color.rgb * cover;
    alpha = f.color.a * cover;
  } else if (shape == 3u) {
    // A rotor, from its hub (0) to its blade tips (1). Each blade covers
    // the wedge it swept, back from where it is now, for the share of the
    // exposure it spent over each point of it: its chord's angle over the
    // wedge's, a blade no thinner than its narrowest pixels. With no blades,
    // a disc of even cover (its hub, or the faint grey of a spinning disc).
    let r = length(f.offset);
    let ang = atan2(f.offset.y, f.offset.x);
    let aa = length(offsetWidth) / max(r, 1e-3);
    let n = i32(f.extra.z + 0.5);
    var cover = select(0.0, f.misc.w, n == 0);
    let chordAng = max(f.extra.y, f.misc.w * length(offsetWidth)) / max(r, 1e-3);
    let half = (f.extra.x + chordAng) * 0.5;
    let share = chordAng / (f.extra.x + chordAng);
    for (var k = 0; k < n; k++) {
      let mid = 2.0 * PI * f32(k) / f32(n) - f.extra.x * 0.5;
      let d = abs(atan2(sin(ang - mid), cos(ang - mid)));
      cover += (1.0 - smoothstep(half - aa, half + aa, d)) * share;
    }
    let edge = 1.0 - smoothstep(1.0 - length(offsetWidth), 1.0, r);
    cover = min(cover, 1.0) * edge * clamp(gap / f.extra.w, 0.0, 1.0);
    alpha = cover;
    rgb = f.color.rgb * smokeLight(f, vec2f(0.0)) * alpha;
  } else if (shape == 4u) {
    // The surface this pixel shows, in the shadow's frame: its quad's point
    // carried along the view ray to the scene's depth. Shaded within the
    // band of the ground's plane, by how far inside the plan it lies.
    let k = scene / f.viewDepth - 1.0;
    let at = f.offset + f.ray.xy * k;
    let off = abs(f.ray.z * k);
    let band = 1.0 - smoothstep(0.5 * f.extra.w, f.extra.w, off);
    let blur = max(f.extra.z, 1e-3);
    let cover = 1.0 - smoothstep(-0.5 * blur, 0.5 * blur, planDistance(at, f.extra.xy));
    alpha = f.color.a * cover * band;
  } else if (shape == 1u) {
    let r = length(f.uv);
    let ang = atan2(f.uv.y, f.uv.x);
    let rays = pow(abs(cos(ang * 2.0)), 16.0) * exp(-r * 2.5) * 0.6 * f.extra.y;
    let edge = 1.0 - smoothstep(0.75, 1.0, r);
    rgb = f.color.rgb * (exp(-r * r * 8.0) + rays) * edge * clamp(gap / 0.5, 0.0, 1.0);
  } else {
    let layer = i32(f.extra.y + 0.5);
    let cols = select(5.0, 8.0, layer == 1);
    let frames = cols * cols;
    let frame = clamp(f.extra.x, 0.0, frames - 1.0);
    let f0 = floor(frame);
    let f1 = min(f0 + 1.0, frames - 1.0);
    let g0 = gx / cols;
    let g1 = gy / cols;
    let s0 = textureSampleGrad(atlas, linearSampler, cell(tuv, f0, cols), layer, g0, g1);
    let s1 = textureSampleGrad(atlas, linearSampler, cell(tuv, f1, cols), layer, g0, g1);
    let tex = mix(s0, s1, frame - f0);
    let straight = tex.rgb / max(tex.a, 1e-3);
    let lum = dot(straight, LUMA);
    let opacity = f.color.a * clamp(gap / max(f.extra.z, 1e-3), 0.0, 1.0);
    if (f.misc.z > 0.5) {
      // Lit, the sheet's own relief on top.
      let relief = mix(0.65, 1.25, smoothstep(0.08, 0.45, lum));
      rgb = f.color.rgb * smokeLight(f, f.offset) * relief * tex.a * opacity;
    } else {
      let boost = 1.0 + f.misc.y * lum * lum;
      rgb = tex.rgb * f.color.rgb * boost * opacity;
    }
    alpha = tex.a * opacity;
  }
  // A shadow is ground: it leaves the fog mask as it is.
  let strength = select(clamp(alpha + max(rgb.r, max(rgb.g, rgb.b)) * 0.25, 0.0, 1.0), 0.0, shape == 4u);
  var out: ColorFog;
  out.color = vec4f(rgb, alpha);
  out.fog = vec4f(0.0, 0.0, 0.0, strength);
  return out;
}

// @block passes_fog_modelRooms
@fragment fn item(input: ModelFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
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

// @block passes_fog_mergeHorizons
@compute @workgroup_size(FOG_BUILD_WORKGROUP) fn item(@builtin(global_invocation_id) gid: vec3u) {
  let P = fogParams;
  let AZ = P.azimuthBins;
  let AZT = P.terrainAzimuthBins;
  let R = FOG_RADIAL_BINS;
  let r = gid.x / AZ;
  let ai = gid.x % AZ;
  if (r >= P.rebuildCount) { return; }
  let e = eyes[rebuild[r * 3u]];
  let theta = (f32(ai) + 0.5) / f32(AZ) * 6.28318531 - 3.14159265;
  let dir = vec2f(cos(theta), sin(theta));
  let lnr = log(e.reach / P.firstBinM);
  // A ray squeezing past boxes on both sides with less than the least sight
  // gap between them is blocked where it passes the second (as the
  // simulation's `squeezed`; here the closest box on each side counts).
  let gap = P.minSightGapM;
  var left = vec3f(1e9, 0.0, 0.0);
  var right = vec3f(1e9, 0.0, 0.0);
  var pm: array<f32, FOG_RADIAL_BINS>;
  var pt: array<f32, FOG_RADIAL_BINS>;
  for (var k = 0u; k < R; k++) { pm[k] = -1e4; pt[k] = 0.0; }
  let sectors = rebuild[r * 3u + 2u];
  let sector = min(u32((f32(ai) + 0.5) / f32(AZ) * f32(sectors)), sectors - 1u);
  let row = rebuild[r * 3u + 1u] + sector * 2u;
  let start = rebuild[row];
  let count = rebuild[row + 1u];
  for (var i = 0u; i < count; i++) {
    let b = occluders[rebuild[start + i]];
    let o0 = e.position.xy - b.center;
    if (length(o0) - length(b.half) > e.reach) { continue; }
    let c = cos(b.yaw);
    let s = sin(b.yaw);
    let o = vec2f(o0.x * c + o0.y * s, -o0.x * s + o0.y * c);
    let dl = vec2f(dir.x * c + dir.y * s, -dir.x * s + dir.y * c);
    var tn = -1e9;
    var tf = 1e9;
    var miss = false;
    if (abs(dl.x) < 1e-7) {
      if (abs(o.x) > b.half.x) { miss = true; }
    } else {
      let t1 = (-b.half.x - o.x) / dl.x;
      let t2 = (b.half.x - o.x) / dl.x;
      tn = max(tn, min(t1, t2));
      tf = min(tf, max(t1, t2));
    }
    if (abs(dl.y) < 1e-7) {
      if (abs(o.y) > b.half.y) { miss = true; }
    } else {
      let t1 = (-b.half.y - o.y) / dl.y;
      let t2 = (b.half.y - o.y) / dl.y;
      tn = max(tn, min(t1, t2));
      tf = min(tf, max(t1, t2));
    }
    let crossed = !(miss || tn > tf || tf <= 0.0);
    if (!crossed && gap > 0.0 && length(max(abs(o) - b.half, vec2f(0.0))) >= gap * 0.5) {
      // Room beside the ray: the nearest corner ahead, (offset, distance, top).
      var near = vec3f(1e9, 0.0, b.top);
      var side = 0.0;
      for (var q = 0u; q < 4u; q++) {
        let v = vec2f(select(-1.0, 1.0, (q & 1u) == 0u), select(-1.0, 1.0, q < 2u)) * b.half - o;
        let t = dot(dl, v);
        if (t <= 0.0 || t >= e.reach) { continue; }
        let off = dl.x * v.y - dl.y * v.x;
        side = sign(off);
        if (abs(off) < near.x) { near = vec3f(abs(off), t, b.top); }
      }
      if (side > 0.0 && near.x < left.x) { left = near; }
      if (side < 0.0 && near.x < right.x) { right = near; }
    }
    // An eye inside a box (a garrison slot) sees out of it.
    if (!crossed || tn < 0.0 || tn > e.reach) { continue; }
    let rel = b.top - e.position.z;
    // Above the eye the steepest point is the entry; below it, the exit.
    // A quarter metre floors the distance of a box touching the eye.
    var te = tn;
    var sl = rel / max(tn, 0.25);
    if (rel < 0.0) {
      te = min(tf, e.reach);
      sl = rel / max(te, 0.25);
    }
    var k0 = 0u;
    if (te > P.firstBinM) { k0 = u32(ceil(log(te / P.firstBinM) / lnr * f32(R - 1u))); }
    if (k0 >= R) { continue; }
    if (sl > pm[k0]) { pm[k0] = sl; pt[k0] = te; }
  }
  if (left.x + right.x < gap) {
    let te = max(left.y, right.y);
    let sl = (min(left.z, right.z) - e.position.z) / max(te, 0.25);
    var k0 = 0u;
    if (te > P.firstBinM) { k0 = u32(ceil(log(te / P.firstBinM) / lnr * f32(R - 1u))); }
    if (k0 < R && sl > pm[k0]) { pm[k0] = sl; pt[k0] = te; }
  }
  let a = (theta + 3.14159265) / 6.28318531 * f32(AZT) - 0.5;
  let af = floor(a);
  let fa = a - af;
  let j0 = u32(i32(af) + i32(AZT)) % AZT;
  let j1 = (j0 + 1u) % AZT;
  let tb = e.slot * AZT * R;
  let base = (e.slot * AZ + ai) * R;
  var hk = -1e4;
  for (var k = 0u; k < R; k++) {
    let c0 = fogUnpack(terrain[tb + j0 * R + k]);
    let c1 = fogUnpack(terrain[tb + j1 * R + k]);
    var f = select(c0.y, c1.y, fa > 0.5);
    var v = max(hk, mix(c0.x, c1.x, fa));
    if (pm[k] > v) {
      let lo = fogBinEdge(i32(k) - 1, P.firstBinM, lnr, R);
      let hi = fogBinEdge(i32(k), P.firstBinM, lnr, R);
      f = clamp((pt[k] - lo) / max(hi - lo, 1e-6), 0.0, 1.0);
      v = pm[k];
    }
    hk = v;
    maps[base + k] = fogPack(v, f, mix(c0.z, c1.z, fa));
  }
}

// @block passes_fog_modelCards
@fragment fn item(input: CardFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
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

// @block passes_fog_composeMask
@fragment fn item(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return composeFog(pos.xy);
}

// @block passes_fog_cullEyes
@compute @workgroup_size(FOG_TILE_PX, FOG_TILE_PX) fn item(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) li: u32, @builtin(global_invocation_id) gid: vec3u) {
  let P = fogParams;
  if (li == 0u) {
    atomicStore(&cullLo[0], 2147483647);
    atomicStore(&cullLo[1], 2147483647);
    atomicStore(&cullHi[0], -2147483647);
    atomicStore(&cullHi[1], -2147483647);
    atomicStore(&cullHit, 0u);
  }
  workgroupBarrier();
  if (f32(gid.x) < P.width && f32(gid.y) < P.height) {
    // Reverse-Z: 0 is the far clear, the sky.
    let z = textureLoad(depth, vec2i(gid.xy), 0);
    if (z > 0.0) {
      let ndc = vec4f((f32(gid.x) + 0.5) / P.width * 2.0 - 1.0, 1.0 - (f32(gid.y) + 0.5) / P.height * 2.0, z, 1.0);
      let h = P.invViewProj * ndc;
      let g = h.xyz / h.w;
      let up = P.cameraEye - g;
      let q = g + up * min(1.0, P.translucentLiftM / max(up.z, 1e-3));
      // Quarter metres, as integers the workgroup can reduce atomically.
      atomicMin(&cullLo[0], i32(floor(min(g.x, q.x) * 4.0)));
      atomicMin(&cullLo[1], i32(floor(min(g.y, q.y) * 4.0)));
      atomicMax(&cullHi[0], i32(ceil(max(g.x, q.x) * 4.0)));
      atomicMax(&cullHi[1], i32(ceil(max(g.y, q.y) * 4.0)));
      atomicStore(&cullHit, 1u);
    }
  }
  workgroupBarrier();
  if (atomicLoad(&cullHit) == 0u) { return; }
  let lo = vec2f(f32(atomicLoad(&cullLo[0])), f32(atomicLoad(&cullLo[1]))) * 0.25;
  let hi = vec2f(f32(atomicLoad(&cullHi[0])), f32(atomicLoad(&cullHi[1]))) * 0.25;
  let tile = wg.x + wg.y * P.tilesX;
  for (var e = li; e < P.eyeCount; e += FOG_TILE_PX * FOG_TILE_PX) {
    let eye = eyes[e];
    let near = clamp(eye.position.xy, lo, hi);
    if (distance(near, eye.position.xy) <= eye.reach) {
      let s = atomicAdd(&counts[tile], 1u);
      if (s < P.tileEyesMax) { lists[tile * P.tileEyesMax + s] = e; }
    }
  }
}

// @block passes_fog_lit
@fragment fn lit(input: LitModelFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
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

// @block passes_fog_probeFn
@compute @workgroup_size(FOG_BUILD_WORKGROUP) fn probeFn(@builtin(global_invocation_id) gid: vec3u) {
  let P = fogParams;
  if (gid.x >= P.probeCount) { return; }
  let q = points[gid.x];
  var seen = 0u;
  for (var e = 0u; e < P.eyeCount; e++) {
    if (fogSeenSurface(e, q.position, q.normal, q.ground == 1u)) { seen = 1u; break; }
  }
  out[gid.x] = seen;
}

// @block passes_fog_shapeFn
@compute @workgroup_size(FOG_BUILD_WORKGROUP) fn shapeFn(@builtin(global_invocation_id) gid: vec3u) {
  if (gid.x >= arrayLength(&rows)) { return; }
  let r = rows[gid.x];
  out[gid.x] = fogShape(r.x, r.y, r.z, r.w);
}

// @block passes_fog_terrainFragment
@fragment fn terrainFragment(input: MeshFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  if (!(scarRegionContains(input.world.xy))) {
    discard;;
  }
  let eye = (&cam.eye);
  let n = normalize(input.normal);
  let seen = fogTerm(input.world, n, clip.xy, fogIsGround());
  let footprint = length(fwidth(input.world.xy));
  let cell = groundCell(input.world.xy, footprint);
  let paved = groundPaved(input.world.xy, cell);
  if (groundClassView()) {
    let xy = input.world.xy;
    let site = groundSite(xy, cell, paved);
    let classes = groundClasses(xy, footprint, site, groundWater(xy, cell));
    return ColorFog(vec4f(classes, 1f), vec4f(0, 1, 1, 1));
  }
  let plain = groundAlbedo(input.world, input.color, cell, paved);
  let biome = (1f - input.color.w);
  let scar = groundScarsSeen(input.world, (*eye), footprint);
  let surface = mix(plain, scarredSurface(plain, scar), biome);
  let bank = groundBank(input.world.xy, cell, footprint);
  var ground = n;
  if ((bank.z > 0f)) {
    ground = normalize(mix(n, normalize(vec3f(-(bank.x), -(bank.y), 1f)), bank.z));
  }
  let relief = groundRoadRelief(input.world.xy, footprint, paved);
  ground = normalize((ground - (vec3f(relief, 0f) * (ground.z * biome))));
  let shading = normalize(mix(ground, scarredNormal(ground, scar), biome));
  let water = groundWater(input.world.xy, cell);
  let dry = (1f - smoothstep((-(footprint) * 0.5f), (footprint * 0.5f), water));
  let paintSample = (groundPaint(input.world) * dry);
  let flecks = groundDapple(input.world.xy, footprint, cell);
  let sunlight = max(sampleSunShadow(input.world, n, clip.xy), flecks);
  let lit = shade(paintedAlbedo(surface.xyz, paintSample), vec3f(), surface.w, 0f, 0f, 1f, shading, input.world, sunlight, (*eye));
  return ColorFog(vec4f((lit.xyz + paintGlow(paintSample)), 1f), fogCoverage(paintedSeen(seen, paintSample), 1f));
}

// @block passes_fog_terrainMarch
@compute @workgroup_size(FOG_BUILD_WORKGROUP) fn terrainMarch(@builtin(global_invocation_id) gid: vec3u) {
  let P = fogParams;
  let AZT = P.terrainAzimuthBins;
  let R = FOG_RADIAL_BINS;
  let r = gid.x / AZT;
  let ai = gid.x % AZT;
  if (r >= P.rebuildCount) { return; }
  let e = eyes[rebuild[r * 3u]];
  let theta = (f32(ai) + 0.5) / f32(AZT) * 6.28318531 - 3.14159265;
  let dir = vec2f(cos(theta), sin(theta));
  let lnr = log(e.reach / P.firstBinM);
  let base = (e.slot * AZT + ai) * R;
  let playableLo = P.playable.xy;
  let playableHi = playableLo + P.playable.zw;
  var hor = -1e4;
  var hk = -1e4;
  var fol = 0.0;
  var k = 0u;
  var t = 0.0;
  var jump = 0.0;
  var jt = 0.0;
  loop {
    let st = clamp(t * P.stepFraction, P.stepMinM, P.stepMaxM);
    t = t + st;
    let q = e.position.xy + dir * t;
    let outside = q.x < playableLo.x || q.y < playableLo.y ||
      q.x > playableHi.x || q.y > playableHi.y;
    if (outside) { t = 1e9; }
    loop {
      if (k >= R) { break; }
      let hi = fogBinEdge(i32(k), P.firstBinM, lnr, R);
      if (hi > t) { break; }
      let lo = fogBinEdge(i32(k) - 1, P.firstBinM, lnr, R);
      let v = max(hk, hor);
      if (v - hk > jump) { jump = v - hk; jt = hi; }
      let f = clamp((jt - lo) / max(hi - lo, 1e-6), 0.0, 1.0);
      terrain[base + k] = fogPack(v, f, fol);
      hk = v;
      jump = 0.0;
      jt = hi;
      k = k + 1u;
    }
    if (k >= R || outside) { break; }
    let gz = fogHeight(q);
    let can = fogFoliage(q);
    let ts = (gz + P.targetHeightM - e.position.z) / t;
    if (can.x > 0.0 && e.position.z + max(hor, ts) * t < gz + can.x) { fol = fol + st * can.y; }
    let nh = max(hor, (gz - e.position.z) / t);
    if (nh - hor > jump) { jump = nh - hor; jt = t; }
    hor = nh;
  }
}

// @block passes_fog_waterFragment
@fragment fn waterFragment(input: MeshFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let eye = (&cam.eye);
  let footprint = length(fwidth(input.world.xy));
  let look = u32(round(input.color.x));
  let w = (&terrainParams.waterLooks[look]);
  let still = select(waterNormal(input.world.xy, footprint, look), vec3f(0, 0, 1), (input.color.w > 0f));
  let n = waveNormal(input.world.xy, still);
  let sunlight = sampleSunShadow(input.world, n, clip.xy);
  let paintSample = groundPaint(input.world);
  let puddle = (input.color.w > 0f);
  let middle = (length((input.world.xy - input.color.yz)) / max(input.color.w, 1e-4f));
  let up = max(normalize(((*eye) - input.world)).z, 0.2f);
  let shaped = (((*w).fades.w * max((1f - (middle * middle)), 0f)) / up);
  let waterDepthM = select(waterDepth(clip, input.world), shaped, puddle);
  let bed = waterUnder(clip, input.world, n, waterDepthM, look);
  let through = exp(((*w).surface.xyz * -(select(bed.w, waterDepthM, puddle))));
  let lit = (bed.xyz * (1f + (waterCaustics(input.world, waterDepthM, footprint, look) * sunlight)));
  let body = mix((*w).edge.xyz, (*w).channel.xyz, (1f - dot(through, vec3f(COLOR_LUMA_R, COLOR_LUMA_G, COLOR_LUMA_B))));
  let reflected = waterReflection(n, input.world, sunlight, (*eye), look);
  let dry = surfaceLight(vec3f(), vec3f(), (*w).roughness.x, 0f, 0f, 1f, n, input.world, sunlight, (*eye));
  let own = (surfaceLight(body, vec3f(), (*w).roughness.x, 0f, 0f, 1f, n, input.world, sunlight, (*eye)) - dry);
  let mark = surfaceLight(paintedAlbedo(body, (paintSample / max(paintSample.w, 1e-4f))), vec3f(), (*w).roughness.x, 0f, 0f, 1f, n, input.world, sunlight, (*eye));
  let water = ((own * (vec3f(1) - through)) + reflected);
  let seenUp = (through * waterTransmission(n, normalize(((*eye) - input.world))));
  let shadowed = mix((*w).surface.w, 1f, sunlight);
  let surfaceInAir = air((water * shadowed), input.world, (*eye)).xyz;
  let bedAir = (air(vec3f(), input.world, (*eye)).xyz * seenUp);
  let wet = ((surfaceInAir - bedAir) + (lit * seenUp));
  let facing = max(0.35f, dot(n, normalize(((*eye) - input.world))));
  let pixel = length(fwidth(atPixel(clip).xy));
  let fade = max((*w).fades.x, ((pixel * 3f) / facing));
  let rim = ((1f - middle) * input.color.w);
  let ring = max((input.color.w / 6f), (pixel * 1.5f));
  let fill = select((waterDepthM / fade), (rim / ring), puddle);
  let edge = smoothstep(0f, 1f, clamp(fill, 0f, 1f));
  let marked = (air((mark * shadowed), input.world, (*eye)).xyz * paintSample.w);
  let cover = (paintSample.w + (edge * (1f - paintSample.w)));
  let colour = (((marked + paintGlow(paintSample)) + (wet * (edge * (1f - paintSample.w)))) / max(cover, 1e-4f));
  let seen = paintedSeen(fogTerm(input.world, n, clip.xy, fogIsGround()), paintSample);
  return ColorFog(vec4f(colour, cover), fogCoverage(seen, cover));
}

// @block passes_fog_wholeFn
@compute @workgroup_size(FOG_BUILD_WORKGROUP) fn wholeFn(@builtin(workgroup_id) wg: vec3u, @builtin(local_invocation_index) li: u32) {
  let W = whole;
  let item = wg.x + wg.y * 65535u;
  if (item >= u32(W.y)) { return; }
  let index = items[item * 3u];
  let eyesStart = items[item * 3u + 1u];
  let eyesCount = items[item * 3u + 2u];
  if (li == 0u) { atomicStore(&wholeFound, 0u); }
  workgroupBarrier();
  let b = boxes[index];
  let step = W.x;
  let u = vec2f(cos(b.yaw), sin(b.yaw));
  let v = vec2f(-u.y, u.x);
  let cx = u32(ceil(2.0 * b.half.x / step)) + 1u;
  let cy = u32(ceil(2.0 * b.half.y / step)) + 1u;
  let lo = b.base + 0.3;
  let hi = max(b.top - 0.3, lo);
  let cz = u32(ceil((hi - lo) / step)) + 1u;
  let wallsX = cy * cz;
  let wallsY = cx * cz;
  let total = 2u * wallsX + 2u * wallsY + cx * cy;
  let inset = min(vec2f(0.2), b.half * 0.5);
  let reach = length(b.half);
  for (var s = li; s < total; s += FOG_BUILD_WORKGROUP) {
    if (atomicLoad(&wholeFound) != 0u) { break; }
    // Along a side: t ranges [0, 1] over n samples, kept off the corners by the inset.
    var local = vec2f(0.0);
    var z = b.top;
    var n = vec3f(0.0, 0.0, 1.0);
    var r = s;
    if (r < 2u * wallsX) {
      let side = select(-1.0, 1.0, r < wallsX);
      r = r % wallsX;
      let t = f32(r % cy) / f32(max(cy - 1u, 1u));
      local = vec2f(side * b.half.x, mix(-b.half.y + inset.y, b.half.y - inset.y, t));
      z = mix(lo, hi, f32(r / cy) / f32(max(cz - 1u, 1u)));
      n = vec3f(u * side, 0.0);
    } else if (r < 2u * wallsX + 2u * wallsY) {
      r = r - 2u * wallsX;
      let side = select(-1.0, 1.0, r < wallsY);
      r = r % wallsY;
      let t = f32(r % cx) / f32(max(cx - 1u, 1u));
      local = vec2f(mix(-b.half.x + inset.x, b.half.x - inset.x, t), side * b.half.y);
      z = mix(lo, hi, f32(r / cx) / f32(max(cz - 1u, 1u)));
      n = vec3f(v * side, 0.0);
    } else {
      r = r - 2u * wallsX - 2u * wallsY;
      let tx = f32(r % cx) / f32(max(cx - 1u, 1u));
      let ty = f32(r / cx) / f32(max(cy - 1u, 1u));
      local = vec2f(mix(-b.half.x + inset.x, b.half.x - inset.x, tx), mix(-b.half.y + inset.y, b.half.y - inset.y, ty));
    }
    let p = vec3f(b.center + u * local.x + v * local.y, z);
    for (var ei = 0u; ei < eyesCount; ei++) {
      let e = items[eyesStart + ei];
      let eye = eyes[e];
      if (distance(eye.position.xy, b.center) - reach > eye.reach) { continue; }
      if (fogSeenSurface(e, p, n, false)) { atomicStore(&wholeFound, 1u); break; }
    }
  }
  workgroupBarrier();
  if (li == 0u) { flags[index] = atomicLoad(&wholeFound); }
}

// @block passes_fog_worldFragment
@fragment fn worldFragment(input: MeshFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let eye = (&cam.eye);
  var n = normalize(input.normal);
  if ((dot(n, ((*eye) - input.world)) < 0f)) {
    n = -(n);
  }
  let albedo = srgbToLinear(input.color.xyz);
  let sunlight = sampleSunShadow(input.world, n, clip.xy);
  let lit = shade(albedo, vec3f(), 0.85f, 0f, 0f, 1f, n, input.world, sunlight, (*eye));
  let glow = highlightGlow(input.highlight);
  let seen = fogTerm(input.world, n, clip.xy, fogIsGround());
  return ColorFog(vec4f((lit.xyz + glow), input.color.w), fogCoverage(seen, input.color.w));
}
// @block passes_fog_FoliageFragmentInput
struct FoliageFragmentInput {
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block passes_fog_CardFragmentInput
struct CardFragmentInput {
  @location(0) world: vec3f,
  @location(1) uv: vec2f,
  @location(2) @interpolate(flat) layer: u32,
  @location(3) @interpolate(flat) yaw: f32,
  @location(4) @interpolate(flat) scale: vec3f,
  @location(5) tint: vec3f,
  @location(6) anchor: vec3f,
}

// @block passes_fog_BackdropFragmentInput
struct MeshFragmentInput {
  @location(0) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) local: vec3f,
  @location(4) heart: vec3f,
}

// @block passes_fog_GrassFragmentInput
struct GrassFragmentInput {
  @location(0) world: vec3f,
  @location(1) root: vec3f,
  @location(2) normal: vec3f,
  @location(3) albedo: vec3f,
  @location(4) ground: f32,
}
