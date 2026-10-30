// WGSL owner: passes_effects.

// @block passes_effects_spriteFragment
@fragment fn spriteFragment(f: SpriteVertexOutput) -> ColorFog {
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

// @block passes_effects_SpriteVertexOutput
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

// @block passes_effects_spriteVertex
@vertex fn spriteVertex(v: SpriteVertexInput) -> SpriteVertexOutput {
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

// @block passes_effects_SpriteVertexInput
struct SpriteVertexInput {
  @builtin(vertex_index) vi: u32,
  @location(0) a: vec4f,
  @location(1) b: vec4f,
  @location(2) color: vec4f,
  @location(3) misc: vec4f,
};

// @block passes_effects_OverlayVertexOutput
struct OverlayVertexOutput {
  @builtin(position) clip: vec4f,
  @location(0) @interpolate(perspective, centroid) world: vec3f,
  @location(1) normal: vec3f,
  @location(2) color: vec4f,
  @location(3) highlight: f32,
}

// @block passes_effects_PaintFragmentInput
struct PaintFragmentInput {
  @location(0) march: vec3f,
  @location(1) color: vec4f,
}

// @block passes_effects_PaintVertexOutput
struct PaintVertexOutput {
  @builtin(position) clip: vec4f,
  @location(0) march: vec3f,
  @location(1) color: vec4f,
}

// @block passes_effects_compositeOverlayFragment
@fragment fn compositeOverlayFragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return compositeOverlay(pos.xy);
}

// @block passes_effects_glowRowsFragment
@fragment fn glowRowsFragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return glowRows(pos.xy);
}

// @block passes_effects_glowColumnsFragment
@fragment fn glowColumnsFragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return glowColumns(pos.xy);
}

// @block passes_effects_overlayFragment
@fragment fn overlayFragment(input: MeshFragmentInput) -> @location(0) vec4f {
  let toEye = (cam.eye - input.world);
  let n = faceEyeNormal(normalize(input.normal), toEye);
  let key = vec3f(0.49965035915374756, -0.5496153831481934, 0.669531524181366);
  let light = max(dot(n, key), 0f);
  let shaded = (input.color.xyz * (0.3f + (0.75f * light)));
  let lit = (shaded + highlightGlow(input.highlight));
  return vec4f(lit, input.color.w);
}

// @block passes_effects_overlayVertex
@vertex fn overlayVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f, @location(3) placement: vec4f, @location(4) tint: vec4f) -> OverlayVertexOutput {
  let c = cos(placement.w);
  let s = sin(placement.w);
  let world = placedPoint(position, placement);
  let clip = (cam.viewProj * vec4f(world, 1f));
  return OverlayVertexOutput(vec4f(clip.x, clip.y, (clip.z * 1.003f), clip.w), world, vec3f(((normal.x * c) - (normal.y * s)), ((normal.x * s) + (normal.y * c)), normal.z), vec4f((color.xyz * tint.xyz), color.w), tint.w);
}

// @block passes_effects_paintFragment
@fragment fn paintFragment(input: PaintFragmentInput) -> @location(0) vec4f {
  let beat = fract(((cam.time * input.march.y) - input.march.x));
  let pulse = (0.5f + (0.5f * cos((beat * 6.2831853f))));
  return vec4f((input.color.xyz * 0.5f), (input.color.w * ((1f - input.march.z) + (input.march.z * pulse))));
}

// @block passes_effects_paintMarchVertex
@vertex fn paintMarchVertex(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) color: vec4f) -> PaintVertexOutput {
  return PaintVertexOutput(clipOf(position), normal, color);
}

// @block passes_effects_paintVertex
@vertex fn paintVertex(@location(0) position: vec3f, @location(2) color: vec4f) -> PaintVertexOutput {
  return PaintVertexOutput(clipOf(position), vec3f(), color);
}
