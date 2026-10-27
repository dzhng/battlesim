pub const WORLD: &str = r#"
struct Camera { view_proj: mat4x4<f32> };
struct Environment {
  sun_direction: vec4<f32>,
  sun_radiance: vec4<f32>,
  settings: vec4<f32>,
  fill: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> environment: Environment;
@group(2) @binding(0) var albedo: texture_2d<f32>;
@group(2) @binding(1) var linear: sampler;
struct Out { @builtin(position) position: vec4<f32>, @location(0) color: vec4<f32>, @location(1) world: vec3<f32> };
@vertex fn terrain_vertex(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> Out {
  var out: Out;
  out.position = camera.view_proj * vec4<f32>(position, 1.0);
  out.color = color;
  out.world = position;
  return out;
}
@vertex fn model_vertex(
  @location(0) position: vec3<f32>, @location(1) color: vec4<f32>, @location(2) uv: vec2<f32>,
  @location(3) placement: vec4<f32>, @location(4) tint: vec4<f32>,
) -> Out {
  let c = cos(placement.w); let s = sin(placement.w);
  let world = vec3<f32>(c * position.x - s * position.y + placement.x,
    s * position.x + c * position.y + placement.y, position.z + placement.z);
  var out: Out;
  out.position = camera.view_proj * vec4<f32>(world, 1.0);
  out.color = color * tint;
  out.world = world;
  return out;
}
@fragment fn world_fragment(in: Out) -> @location(0) vec4<f32> {
  let sun = max(dot(vec3<f32>(0.0, 0.0, 1.0), normalize(environment.sun_direction.xyz)), 0.0);
  let lit = environment.fill.xyz + environment.sun_radiance.xyz * sun;
  return vec4<f32>(in.color.rgb * lit, in.color.a);
}
@fragment fn model_fragment(in: Out) -> @location(0) vec4<f32> {
  let sun = max(dot(vec3<f32>(0.0, 0.0, 1.0), normalize(environment.sun_direction.xyz)), 0.0);
  let lit = environment.fill.xyz + environment.sun_radiance.xyz * sun;
  return vec4<f32>(in.color.rgb * textureSample(albedo, linear, in.world.xy).rgb * lit, in.color.a);
}
"#;

pub const SHADOW: &str = r#"
struct Camera { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;
struct Out { @builtin(position) position: vec4<f32> };
@vertex fn terrain_vertex(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> Out {
  var out: Out; out.position = camera.view_proj * vec4<f32>(position, 1.0); return out;
}
@vertex fn model_vertex(
  @location(0) position: vec3<f32>, @location(1) color: vec4<f32>, @location(2) uv: vec2<f32>,
  @location(3) placement: vec4<f32>, @location(4) tint: vec4<f32>,
) -> Out {
  let c = cos(placement.w); let s = sin(placement.w);
  let world = vec3<f32>(c * position.x - s * position.y + placement.x,
    s * position.x + c * position.y + placement.y, position.z + placement.z);
  var out: Out; out.position = camera.view_proj * vec4<f32>(world, 1.0); return out;
}
"#;

pub const FOG: &str = r#"
struct Fog { colour: vec4<f32>, params: vec4<f32> };
@group(0) @binding(0) var<uniform> fog: Fog;
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vertex(@builtin(vertex_index) index: u32) -> Out {
  var positions = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
  var out: Out; out.position = vec4<f32>(positions[index], 0.0, 1.0);
  out.uv = vec2<f32>(positions[index].x * 0.5 + 0.5, -positions[index].y * 0.5 + 0.5); return out;
}
@fragment fn fragment(in: Out) -> @location(0) vec4<f32> {
  let edge = distance(in.uv, vec2<f32>(0.5)) * 1.41421356;
  let coverage = clamp(fog.params.x + edge * fog.params.y, 0.0, 1.0);
  return vec4<f32>(coverage, fog.params.z, fog.params.w, 1.0);
}
"#;

pub const COMPOSITE: &str = r#"
struct Fog { colour: vec4<f32>, params: vec4<f32> };
@group(0) @binding(0) var world: texture_2d<f32>;
@group(0) @binding(1) var mask: texture_2d<f32>;
@group(0) @binding(2) var linear: sampler;
@group(1) @binding(0) var<uniform> fog: Fog;
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vertex(@builtin(vertex_index) index: u32) -> Out {
  var positions = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
  var out: Out; out.position = vec4<f32>(positions[index], 0.0, 1.0);
  out.uv = vec2<f32>(positions[index].x * 0.5 + 0.5, -positions[index].y * 0.5 + 0.5); return out;
}
@fragment fn fragment(in: Out) -> @location(0) vec4<f32> {
  let lit = textureSample(world, linear, in.uv);
  let coverage = textureSample(mask, linear, in.uv).x;
  let graded = mix(lit.rgb, fog.colour.rgb, coverage * fog.params.z);
  let exposure = max(fog.params.w, 0.01);
  return vec4<f32>(1.0 - exp(-graded * exposure), lit.a);
}
"#;

pub const EFFECTS: &str = r#"
struct Camera { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;
struct Out { @builtin(position) position: vec4<f32>, @location(0) colour: vec4<f32> };
@vertex fn vertex(@builtin(vertex_index) vertex: u32, @location(1) effect: vec4<f32>, @location(2) colour: vec4<f32>) -> Out {
  var corners = array<vec2<f32>, 6>(vec2<f32>(-1,-1),vec2<f32>(1,-1),vec2<f32>(1,1),vec2<f32>(-1,-1),vec2<f32>(1,1),vec2<f32>(-1,1));
  let p = effect.xyz + vec3<f32>(corners[vertex] * effect.w, 0.0);
  var out: Out; out.position = camera.view_proj * vec4<f32>(p, 1.0); out.colour = colour; return out;
}
@fragment fn fragment(in: Out) -> @location(0) vec4<f32> { return in.colour; }
"#;
