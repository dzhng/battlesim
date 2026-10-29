struct GrassParams {
  planes: array<vec4f, 4>,
  eye: vec4f,
  window: vec4f,
  grid: vec4u,
  density: vec4f,
  tiers: vec4f,
  shading: vec4f,
  clear: vec4f,
  counts: vec4u,
  ground: vec4f,
  sway: vec4f,
  growth: array<vec4f, 16>,
  patches: array<vec4f, 16>,
  mixes: array<vec4f, 64>,
  patchScales: vec4f,
  grain: vec4f,
  kinds: array<vec4f, 16>,
  bases: array<vec4u, 16>,
}

@group(0) @binding(0) var<uniform> params: GrassParams;

@group(0) @binding(4) var<storage, read_write> args: array<atomic<u32>, 12>;

@compute @workgroup_size(1) fn finishFn() {
  let P = &params;
  for (var t = 0u; t < 2u; t++) {
    let raw = atomicLoad(&args[t * 5u + 1u]);
    atomicStore(&args[10u + t], raw);
    atomicStore(&args[t * 5u + 1u], min(raw, select((*P).grid.w, (*P).grid.z, t == 0u)));
  }
}