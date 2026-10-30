// WGSL owner: scenery.

// @block scenery_surface
fn surface(world: vec3f, normal: vec3f, color: vec4f, local: vec3f, sunlight: f32) -> vec3f {
  let eye = (&cam.eye);
  var n = normalize(normal);
  var clumpShade = 1f;
  var roughness = 0.8999999761581421f;
  let fade = (1f - smoothstep(0.08f, 0.45f, length(fwidth(world))));
  if (((color.w > 0.5f) && (fade > 0f))) {
    let clumps = leafClumps(local, n, fade);
    n = clumps.xyz;
    clumpShade = clumps.w;
    roughness = 0.85f;
  }
  let lit = shade((color.xyz * clumpShade), vec3f(), roughness, 0f, 0f, 1f, n, world, sunlight, (*eye));
  return lit.xyz;
}

// @block scenery_leafClumps
fn leafClumps(p: vec3f, n: vec3f, fade: f32) -> vec4f {
  let q = p * 2.1;
  let bend = vec3f(
    valueNoise3(q),
    valueNoise3(q + vec3f(17.3, 5.1, 9.7)),
    valueNoise3(q + vec3f(3.7, 29.3, 13.1)),
  ) - vec3f(0.5);
  let bent = normalize(n + bend * (0.7 * fade));
  let gap = valueNoise3(p * 3.4 + vec3f(7.0, 1.0, 3.0));
  let shade = mix(1.0, 0.72 + 0.4 * gap, fade);
  return vec4f(bent, shade);
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

// @block scenery_placedPoint
fn placedPoint(local: vec3f, pose: vec4f) -> vec3f {
  let c = cos(pose.w);
  let s = sin(pose.w);
  return vec3f((((local.x * c) - (local.y * s)) + pose.x), (((local.x * s) + (local.y * c)) + pose.y), (local.z + pose.z));
}
