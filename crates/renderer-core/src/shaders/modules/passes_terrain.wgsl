// WGSL owner: passes_terrain.

// @block passes_terrain_SpriteVertexOutput
struct SpriteVertexOutput {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
  @location(1) color: vec4f,
  @location(2) @interpolate(flat) misc: vec4f,
  @location(3) @interpolate(flat) extra: vec4f,
  @location(4) along: f32,
  @location(5) viewDepth: f32,
  /** A lit sprite's sun in its own frame (right, up, toward the eye), and
   *  the sky's light on its top and on its side. */
  @location(6) @interpolate(flat) sun: vec3f,
  @location(8) @interpolate(flat) skyTop: vec3f,
  @location(9) @interpolate(flat) skySide: vec3f,
  /** Where on the sprite, screen-aligned, -1..1. */
  @location(7) offset: vec2f,
  /** A lit sprite's cast light (castAt), along a ribbon from end to end. */
  @location(10) castLit: vec3f,
  /** A ground shadow's view ray (from the eye to its quad), in the shadow's
   *  own frame: along its nose, across it, and up from its ground. */
  @location(11) ray: vec3f,
};

// @block passes_terrain_fragment
@fragment fn fragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return backgroundColor(input.uv);
}

// @block passes_terrain_vs
@vertex fn vs(v: SpriteVertexInput) -> SpriteVertexOutput {
  var corners = array<vec2f, 6>(
    vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0),
    vec2f(-1.0, -1.0), vec2f(1.0, 1.0), vec2f(-1.0, 1.0));
  let c = corners[v.vi];
  let half = vec2f(cam.width, cam.height) * 0.5;
  let s = projScale();
  var out: SpriteVertexOutput;
  out.uv = c;
  out.color = v.color;
  out.misc = v.misc;
  out.extra = vec4f(0.0);
  out.along = 0.0;
  out.viewDepth = 1.0;
  out.sun = vec3f(0.0, 0.0, 1.0);
  out.skyTop = vec3f(0.0);
  out.skySide = vec3f(0.0);
  out.castLit = vec3f(0.0);
  out.ray = vec3f(0.0);
  out.offset = c;
  let shape = u32(v.misc.x);
  if (shape == 0u) {
    var ca = cam.viewProj * vec4f(v.a.xyz, 1.0);
    var cb = cam.viewProj * vec4f(v.b.xyz, 1.0);
    if (ca.w < NEAR_W && cb.w < NEAR_W) {
      out.pos = vec4f(0.0, 0.0, -2.0, 1.0);
      return out;
    }
    var alongA = v.misc.y;
    var alongB = v.misc.z;
    if (ca.w < NEAR_W) {
      let t = (NEAR_W - ca.w) / (cb.w - ca.w);
      ca = mix(ca, cb, t);
      alongA = mix(alongA, alongB, t);
    }
    if (cb.w < NEAR_W) {
      let t = (NEAR_W - cb.w) / (ca.w - cb.w);
      cb = mix(cb, ca, t);
      alongB = mix(alongB, alongA, t);
    }
    let sa = ca.xy / ca.w * half;
    let sb = cb.xy / cb.w * half;
    let d = sb - sa;
    let len = length(d);
    let dir = select(vec2f(1.0, 0.0), d / max(len, 1e-6), len > 1e-4);
    let perp = vec2f(-dir.y, dir.x);
    let end = select(ca, cb, c.x > 0.0);
    // Negative width encodes a fixed screen-space line, independent of zoom.
    let truePx = select(v.a.w * s.y * half.y / end.w, -v.a.w, v.a.w < 0.0);
    let fixedLine = v.a.w < 0.0;
    // Cover neighboring pixel centers so a one-pixel diagonal does not stipple.
    let px = select(max(truePx, v.b.w), truePx + 2.0, fixedLine);
    let coverage = min(1.0, truePx / px);
    // Zoom-readable rounds get brighter as their world width shrinks below
    // the pixel floor, so both the halo and post bloom survive strategic zoom.
    let boost = abs(v.misc.w);
    let k = select(coverage, mix(boost, 1.0, coverage), boost > 1.0);
    let energy = select(k, 1.0, fixedLine);
    let ribbon = v.color.a > 0.0;
    // A light streak overhangs its ends by a quarter of its width and fades
    // over the overhang, so stretches laid end to end add up to one line; a
    // ribbon, blended over rather than added, is cut square instead.
    let overhang = select(0.25, 0.0, ribbon);
    let off = (perp * c.y + dir * c.x * overhang * 2.0) * px * 0.5;
    out.pos = vec4f(end.xy + off / half * end.w, end.zw);
    out.color = select(vec4f(v.color.rgb * energy, 0.0), v.color * energy, ribbon);
    out.along = select(alongA, alongB, c.x > 0.0);
    out.viewDepth = end.w;
    // Pixels from the quad's middle to each end, and the overhang; a
    // ribbon's soft-particle depth, half its width.
    out.extra = vec4f(len * 0.5 + px * overhang, px * overhang, v.a.w * 0.5, select(0.0, truePx, fixedLine));
    if (ribbon) {
      out.offset = perp * c.y;
      let l = spriteLight();
      out.sun = l.sun;
      out.skyTop = l.skyTop;
      out.skySide = l.skySide;
      out.castLit = castAt(select(v.a.xyz, v.b.xyz, c.x > 0.0));
    }
    return out;
  }
  if (shape == 3u) {
    // A rotor: the quad round its hub in the plane across its axis, its
    // first blade along +x of that plane.
    let axis = normalize(v.b.xyz);
    let u = normalize(v.color.xyz - axis * dot(v.color.xyz, axis));
    let w = cross(axis, u);
    let clip = cam.viewProj * vec4f(v.a.xyz + (u * c.x + w * c.y) * v.a.w, 1.0);
    if (clip.w < NEAR_W) {
      out.pos = vec4f(0.0, 0.0, -2.0, 1.0);
      return out;
    }
    out.pos = clip;
    out.viewDepth = clip.w;
    out.color = vec4f(vec3f(v.color.w), 1.0);
    // Its sweep, a blade's chord over its reach, its blades, and how deep
    // it fades into what it meets. misc.w: a bladed disc's narrowest blade,
    // pixels; a bare disc's cover.
    out.extra = vec4f(v.b.w, v.misc.y, v.misc.z, ROTOR_SOFT_M);
    let l = spriteLight();
    out.sun = l.sun;
    out.skyTop = l.skyTop;
    out.skySide = l.skySide;
    out.castLit = castAt(v.a.xyz);
    return out;
  }
  if (shape == 4u) {
    // A high-air aircraft's shadow: a quad in the ground's plane round the
    // middle of its plan, along its nose, reaching its blur past its plan.
    let up = v.b.xyz;
    let along = v.color.xyz;
    let across = cross(up, along);
    let blur = v.misc.y;
    let local = c * vec2f(v.a.w + blur, v.b.w + blur);
    let p = v.a.xyz + along * local.x + across * local.y;
    let clip = cam.viewProj * vec4f(p, 1.0);
    if (clip.w < NEAR_W) {
      out.pos = vec4f(0.0, 0.0, -2.0, 1.0);
      return out;
    }
    out.pos = clip;
    out.viewDepth = clip.w;
    out.offset = local;
    let toward = p - cam.eye;
    out.ray = vec3f(dot(toward, along), dot(toward, across), dot(toward, up));
    // Its half length and half span, its blur, and its ground's band.
    out.extra = vec4f(v.a.w, v.b.w, blur, v.misc.z);
    out.color = vec4f(0.0, 0.0, 0.0, v.color.w);
    return out;
  }
  let clip = cam.viewProj * vec4f(v.a.xyz, 1.0);
  if (clip.w < NEAR_W) {
    out.pos = vec4f(0.0, 0.0, -2.0, 1.0);
    return out;
  }
  let cr = cos(v.b.x);
  let sr = sin(v.b.x);
  let turned = vec2f(c.x * cr - c.y * sr, c.x * sr + c.y * cr);
  let truePx = v.a.w * s.y * half.y / clip.w;
  let minPx = select(0.0, v.b.y, shape == 1u);
  let px = max(truePx, minPx);
  let radius = v.a.w * px / max(truePx, 1e-6);
  out.pos = vec4f(clip.xy + turned * radius * s, clip.zw);
  if (shape == 1u) {
    // A glow dims as the square of its shrink below the least size; a
    // solid disc (opacity in alpha) thins its coverage with it.
    let k = truePx / px;
    out.color = select(vec4f(v.color.rgb * k * k, 0.0), v.color * k, v.color.a > 0.0);
  }
  out.extra = vec4f(v.b.y, v.b.z, v.b.w, 0.0);
  out.viewDepth = clip.w;
  out.offset = turned;
  if (shape == 2u && v.misc.z > 0.5) {
    let l = spriteLight();
    out.sun = l.sun;
    out.skyTop = l.skyTop;
    out.skySide = l.skySide;
    out.castLit = castAt(v.a.xyz);
  }
  return out;
}
// @block passes_terrain_SpriteVertexInput
struct SpriteVertexInput {
  @builtin(vertex_index) vi: u32,
  @location(0) a: vec4f,
  @location(1) b: vec4f,
  @location(2) color: vec4f,
  @location(3) misc: vec4f,
};

// @block passes_terrain_sceneColourMip_fragment
@fragment fn fragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return sceneMip(pos.xy);
}

// @block passes_terrain_waterGuardMip_fragment
@fragment fn fragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return guardMip(pos.xy);
}

// @block passes_terrain_waterGuardSeed_fragment
@fragment fn fragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return guardSeed(pos.xy);
}

// @block passes_terrain_waterNormalsBake_fragment
@fragment fn fragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return waterNormalsSlope(fract(input.uv));
}

// @block passes_terrain_waveStep_fragment
@fragment fn fragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return waveStep(pos.xy);
}
