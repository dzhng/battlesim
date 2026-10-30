// WGSL owner: passes_water.

// @block passes_water_waterFragment
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
  let pixel = length(fwidth(worldAtPixel(clip).xy));
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

// @block passes_water_sceneColourMipFragment
@fragment fn sceneColourMipFragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return sceneMip(pos.xy);
}

// @block passes_water_waterGuardMipFragment
@fragment fn waterGuardMipFragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return guardMip(pos.xy);
}

// @block passes_water_waterGuardSeedFragment
@fragment fn waterGuardSeedFragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return guardSeed(pos.xy);
}

// @block passes_water_waterNormalsBakeFragment
@fragment fn waterNormalsBakeFragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return waterNormalsSlope(fract(input.uv));
}

// @block passes_water_waveStepFragment
@fragment fn waveStepFragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return waveStep(pos.xy);
}
