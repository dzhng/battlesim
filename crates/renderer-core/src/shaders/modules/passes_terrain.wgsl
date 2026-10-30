// WGSL owner: passes_terrain.

// @block passes_terrain_groundBackdropFragment
@fragment fn groundBackdropFragment(input: MeshFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
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

// @block passes_terrain_terrainFragment
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

// @block passes_terrain_worldFragment
@fragment fn worldFragment(input: MeshFragmentInput, @builtin(position) clip: vec4f) -> ColorFog {
  let eye = (&cam.eye);
  let n = faceEyeNormal(normalize(input.normal), ((*eye) - input.world));
  let albedo = srgbToLinear(input.color.xyz);
  let sunlight = sampleSunShadow(input.world, n, clip.xy);
  let lit = shade(albedo, vec3f(), 0.85f, 0f, 0f, 1f, n, input.world, sunlight, (*eye));
  let glow = highlightGlow(input.highlight);
  let seen = fogTerm(input.world, n, clip.xy, fogIsGround());
  return ColorFog(vec4f((lit.xyz + glow), input.color.w), fogCoverage(seen, input.color.w));
}
