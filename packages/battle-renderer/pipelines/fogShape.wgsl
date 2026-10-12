@group(0) @binding(0) var<storage, read> rows: array<vec4f>;

@group(0) @binding(1) var<storage, read_write> out: array<f32>;

fn fogShape(front: f32, side: f32, rear: f32, off: f32) -> f32 {
  let c = cos(off);
  let end = select(rear, front, c >= 0.0);
  return side * (1.0 - c * c) + end * c * c;
}

@compute @workgroup_size(64) fn shapeFn(@builtin(global_invocation_id) gid: vec3u) {
  if (gid.x >= arrayLength(&rows)) { return; }
  let r = rows[gid.x];
  out[gid.x] = fogShape(r.x, r.y, r.z, r.w);
}