// Card points follow the same local scale, heading and placement as meshes.
import { tgpu, d, std } from "typegpu";

export const cardPoint = tgpu.fn(
  [d.vec3f, d.vec3f, d.f32, d.vec3f],
  d.vec3f,
)((point, scale, yaw, at) => {
  "use gpu";
  const p = std.mul(point, scale);
  const c = std.cos(yaw);
  const s = std.sin(yaw);
  return std.add(at, d.vec3f(p.x * c - p.y * s, p.x * s + p.y * c, p.z));
});

// Choose the baked view in the unscaled model frame, not the stretched world frame.
export const cardDirection = tgpu.fn(
  [d.vec3f, d.vec3f, d.f32],
  d.vec3f,
)((direction, scale, yaw) => {
  "use gpu";
  const c = std.cos(yaw);
  const s = std.sin(yaw);
  return std.div(
    d.vec3f(direction.x * c + direction.y * s, -direction.x * s + direction.y * c, direction.z),
    scale,
  );
});

export const cardNormal = tgpu.fn(
  [d.vec3f, d.vec3f, d.f32],
  d.vec3f,
)((normal, scale, yaw) => {
  "use gpu";
  return std.normalize(cardPoint(std.div(normal, scale), d.vec3f(1), yaw, d.vec3f(0)));
});

/** Restore the baked surface along its view ray. RG stores a coverage-weighted
 * little-endian UNORM16 depth; linear filtering commutes with this decoding. */
export const cardSurface = tgpu.fn(
  [d.vec3f, d.vec3f, d.vec2f, d.f32, d.f32],
  d.vec3f,
)((plane, toward, packed, coverage, radius) => {
  "use gpu";
  const depth = (packed.x + 256 * packed.y) / (257 * coverage);
  return std.add(plane, std.mul(toward, (2 * depth - 1) * radius));
});
