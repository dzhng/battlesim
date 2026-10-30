// WGSL owner: camera.

// @block camera_clipOf
fn clipOf(position: vec3f) -> vec4f {
  let toEye = normalize((cam.eye - position));
  let drawn = (position + (toEye * 1f));
  return (cam.viewProj * vec4f(drawn, 1f));
}

// @block camera_Camera
struct Camera {
  viewProj: mat4x4f,
  invViewProj: mat4x4f,
  eye: vec3f,
  znear: f32,
  width: f32,
  height: f32,
  time: f32,
  wind: array<Wind, 1>,
  waves: vec4f,
}

// @block camera_Wind
struct Wind {
  toward: vec3f,
  driftMps: f32,
  gustMps: f32,
  gustPerM: f32,
}

// @block camera_cameraWind
fn cameraWind() -> Wind { return cam.wind[0]; }
// @block camera_NEAR_W
const NEAR_W = 0.05;

// @block camera_Rays
struct Rays {
  origin: vec3f,
  dx: vec3f,
  dy: vec3f,
}

// @block camera_cam
@group(0) @binding(0) var<uniform> cam: Camera;

// @block camera_projScale
/** Clip-space units per metre at unit depth, across and up. */
fn projScale() -> vec2f {
  let m = cam.viewProj;
  return vec2f(length(vec3f(m[0].x, m[1].x, m[2].x)), length(vec3f(m[0].y, m[1].y, m[2].y)));
}

// @block camera_rays
@group(0) @binding(3) var<uniform> rays: Rays;

// @block camera_ndcAt
fn ndcAt(at: vec2i) -> vec2f {
    let cam = cam;
    let centre = vec2f(at) + 0.5;
    return vec2f(centre.x / cam.width * 2.0 - 1.0, 1.0 - centre.y / cam.height * 2.0);
  }
