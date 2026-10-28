// Where each piece of scenery stands: the instance records the scenery
// layer draws from, and the vertex stages that place a standing tree, a
// piece of forest-floor dressing and a felled tree from them.
import { tgpu, d, std } from "typegpu";
import { typegpuCameraLayout } from "../world/camera";
import { KICK_RADII } from "../scenery/appearance";
import { vertexLayout } from "./geometry";
import { sway } from "./sway";
import { SWAY } from "../records.gen";

/** The frame's instance record (`Instance`) as the scenery reads it: its
 *  placement is the pose, its scale the shape (with the heart), its tint the
 *  colour and seed. */
const PlacedInstance = d.unstruct({
  pose: d.float32x4,
  data: d.float32x4,
  tint: d.float32x4,
  shape: d.float32x4,
});
export const placedLayout = tgpu.vertexLayout(d.disarrayOf(PlacedInstance), "instance");
export const placedAttribs = {
  ...vertexLayout.attrib,
  pose: placedLayout.attrib.pose,
  tint: placedLayout.attrib.tint,
  shape: placedLayout.attrib.shape,
};
/** A felled tree's two records: the tree, with its fall (way, angle,
 *  hinge) in the first's data; then its rest (press up-and-down and across,
 *  trunk, landing) and lie (crown base). */
const FelledInstance = d.unstruct({
  pose: d.float32x4,
  fall: d.float32x4,
  tint: d.float32x4,
  shape: d.float32x4,
  rest: d.float32x4,
  lie: d.float32x4,
  unused0: d.float32x4,
  unused1: d.float32x4,
});
export const felledLayout = tgpu.vertexLayout(d.disarrayOf(FelledInstance), "instance");
export const felledAttribs = {
  ...vertexLayout.attrib,
  pose: felledLayout.attrib.pose,
  tint: felledLayout.attrib.tint,
  shape: felledLayout.attrib.shape,
  fall: felledLayout.attrib.fall,
  rest: felledLayout.attrib.rest,
  lie: felledLayout.attrib.lie,
};
/** A piece of dressing is gone at this share of the height it starts to
 *  shrink at (`ground::walk`'s). */
const DRESSING_GONE = 0.5;

export const placedVaryings = {
  clip: d.builtin.position,
  world: d.vec3f,
  normal: d.vec3f,
  color: d.vec4f,
  local: d.vec3f,
  heart: d.vec3f,
};

const placedIn = {
  position: d.vec3f,
  normal: d.vec3f,
  color: d.vec4f,
  pose: d.vec4f,
  shape: d.vec4f,
  tint: d.vec4f,
};
const placedOut = {
  // Invariant, so the depth prepass and the colour pass agree exactly.
  clip: d.invariant(d.builtin.position) as unknown as typeof d.builtin.position,
  world: d.vec3f,
  normal: d.vec3f,
  color: d.vec4f,
  local: d.vec3f,
  heart: d.vec3f,
};

/** A point of the appearance, already scaled, turned by the instance's yaw
 *  about +Z and stood on its foot. */
const placedPoint = tgpu.fn(
  [d.vec3f, d.vec4f],
  d.vec3f,
)((local, pose) => {
  "use gpu";
  const c = std.cos(pose.w);
  const s = std.sin(pose.w);
  return d.vec3f(
    local.x * c - local.y * s + pose.x,
    local.x * s + local.y * c + pose.y,
    local.z + pose.z,
  );
});

/** The appearance's normal under the instance's scale (inverted) and yaw. */
const placedNormal = tgpu.fn(
  [d.vec3f, d.vec4f, d.f32],
  d.vec3f,
)((normal, shape, yaw) => {
  "use gpu";
  const c = std.cos(yaw);
  const s = std.sin(yaw);
  const nx = normal.x / shape.x;
  const ny = normal.y / shape.y;
  return std.normalize(d.vec3f(nx * c - ny * s, nx * s + ny * c, normal.z / shape.z));
});

/** Where a point of a standing appearance stands: placed (`placedPoint`),
 *  then moved in the wind (`sway`); `leaf` is the point's leaf flag, zero in
 *  a shadow caster. Its height is its crown's heart over `SWAY.heart`. */
const standing = tgpu.fn(
  [d.vec3f, d.vec4f, d.vec4f, d.f32],
  d.vec3f,
)((local, pose, shape, leaf) => {
  "use gpu";
  const at = placedPoint(local, pose);
  const height = std.max(shape.w / SWAY.heart, 1);
  return std.add(at, sway(pose.xyz, at, local.z / height, height, leaf));
});

/** Places the appearance: scale per axis, yaw about +Z, then the instance's
 *  foot, bent in the wind with its leaves shivering. Normals take the
 *  inverse scale. The depth prepass and the colour passes share it. */
export const placedVertex = tgpu.vertexFn({ in: placedIn, out: placedOut })((v) => {
  "use gpu";
  const local = d.vec3f(
    v.position.x * v.shape.x,
    v.position.y * v.shape.y,
    v.position.z * v.shape.z,
  );
  const world = standing(local, v.pose, v.shape, std.select(0, 1, v.color.w > 0.5));
  return {
    clip: std.mul(typegpuCameraLayout.$.cam.viewProj, d.vec4f(world, 1)),
    world,
    normal: placedNormal(v.normal, v.shape, v.pose.w),
    color: d.vec4f(std.mul(v.color.xyz, v.tint.xyz), v.color.w),
    local: d.vec3f(local.x + v.tint.w, local.y, local.z),
    heart: d.vec3f(v.pose.x, v.pose.y, v.pose.z + v.shape.w),
  };
});

/** `placedVertex` for a sun cascade: the trunk bends, the leaves hold
 *  still, so a shadow follows the tree without shivering. */
export const casterVertex = tgpu.vertexFn({ in: placedIn, out: placedOut })((v) => {
  "use gpu";
  const local = d.vec3f(
    v.position.x * v.shape.x,
    v.position.y * v.shape.y,
    v.position.z * v.shape.z,
  );
  const world = standing(local, v.pose, v.shape, 0);
  return {
    clip: std.mul(typegpuCameraLayout.$.cam.viewProj, d.vec4f(world, 1)),
    world,
    normal: placedNormal(v.normal, v.shape, v.pose.w),
    color: d.vec4f(std.mul(v.color.xyz, v.tint.xyz), v.color.w),
    local,
    heart: d.vec3f(v.pose.x, v.pose.y, v.pose.z + v.shape.w),
  };
});

/** Places a piece of dressing as a tree is placed, shrunk about its foot as
 *  it nears a few pixels: its tint's last float is its height over the
 *  projected height it starts to shrink at (`ground::walk`), so its
 *  size on screen over that height is that float times the pixels a metre
 *  covers at its distance. */
export const dressingVertex = tgpu.vertexFn({ in: placedIn, out: placedOut })((v) => {
  "use gpu";
  const cam = typegpuCameraLayout.$.cam;
  // The projection's vertical scale is the length of the view-projection's
  // second row over the world's axes; times half the viewport's height it is
  // the pixels a metre covers a metre away.
  const row = d.vec3f(
    std.mul(cam.viewProj, d.vec4f(1, 0, 0, 0)).y,
    std.mul(cam.viewProj, d.vec4f(0, 1, 0, 0)).y,
    std.mul(cam.viewProj, d.vec4f(0, 0, 1, 0)).y,
  );
  const pixelsPerMetre = 0.5 * cam.height * std.length(row);
  const size = (v.tint.w * pixelsPerMetre) / std.max(std.distance(v.pose.xyz, cam.eye), 0.001);
  const grow = std.smoothstep(DRESSING_GONE, 1, size);
  const local = d.vec3f(
    v.position.x * v.shape.x * grow,
    v.position.y * v.shape.y * grow,
    v.position.z * v.shape.z * grow,
  );
  const world = placedPoint(local, v.pose);
  return {
    clip: std.mul(cam.viewProj, d.vec4f(world, 1)),
    world,
    normal: placedNormal(v.normal, v.shape, v.pose.w),
    color: d.vec4f(std.mul(v.color.xyz, v.tint.xyz), v.color.w),
    local,
    heart: d.vec3f(v.pose.x, v.pose.y, v.pose.z + v.shape.w * grow),
  };
});

/** `d` metres from a trunk's axis with what lies past its `radius` scaled
 *  by `k`. */
const pastTrunk = tgpu.fn(
  [d.f32, d.f32, d.f32],
  d.f32,
)(/* wgsl */ `(d: f32, radius: f32, k: f32) -> f32 {
  let m = abs(d);
  return sign(d) * (min(m, radius) + max(m - radius, 0.0) * k);
}`);

/** Places a felled tree as a standing one is placed, then poses it by the
 *  pose `ground::walk` gave it: tipped `fall.z`
 *  radians toward `fall.xy` about the hinge `fall.w` above its foot, kicked
 *  off its stump and lowered as it lands (`rest.w`); its leaves, and its
 *  bark above the crown's base (`lie.x`), pressed by `rest.x` up-and-down
 *  and `rest.y` across; its trunk below keeps its radius `rest.z`. */
export const felledVertex = tgpu.vertexFn({
  in: { ...placedIn, fall: d.vec4f, rest: d.vec4f, lie: d.vec4f },
  out: placedOut,
})((v) => {
  "use gpu";
  const local = d.vec3f(
    v.position.x * v.shape.x,
    v.position.y * v.shape.y,
    v.position.z * v.shape.z,
  );
  const yc = std.cos(v.pose.w);
  const ys = std.sin(v.pose.w);
  const p = d.vec3f(local.x * yc - local.y * ys, local.x * ys + local.y * yc, local.z);
  const t = v.fall.xy;
  const c = std.cos(v.fall.z);
  const s = std.sin(v.fall.z);
  const hinge = v.fall.w;
  const trunk = v.rest.z;
  const landed = v.rest.w;
  const core = std.select(trunk, 0, v.color.w > 0.5 || local.z > v.lie.x);
  const a = std.dot(p.xy, t);
  const across = std.sub(p.xy, std.mul(t, a));
  const side = std.length(across);
  // Below the cut the stump stands in its place.
  const z = std.max(p.z - hinge, 0);
  const flat = std.add(
    std.mul(t, a * c + z * s + KICK_RADII * trunk * landed),
    std.mul(across, pastTrunk(side, core, v.rest.y) / std.max(side, 1e-6)),
  );
  const world = d.vec3f(
    v.pose.x + flat.x,
    v.pose.y + flat.y,
    v.pose.z + trunk * landed + hinge * (1 - landed) + z * c - pastTrunk(a, core, v.rest.x) * s,
  );
  // The normal turns with the tree; what was pressed takes its inverse.
  const n = placedNormal(v.normal, v.shape, v.pose.w);
  const na = std.dot(n.xy, t);
  const nAcross = std.sub(n.xy, std.mul(t, na));
  const pressAcross = std.select(1, v.rest.y, side > core);
  const pressUp = std.select(1, v.rest.x, std.abs(a) > core);
  const nFlat = std.add(std.mul(t, na * c + n.z * s), std.mul(nAcross, 1 / pressAcross));
  const heart = v.shape.w - hinge;
  return {
    clip: std.mul(typegpuCameraLayout.$.cam.viewProj, d.vec4f(world, 1)),
    world,
    normal: std.normalize(d.vec3f(nFlat.x, nFlat.y, (n.z * c - na * s) / pressUp)),
    color: d.vec4f(std.mul(v.color.xyz, v.tint.xyz), v.color.w),
    local: d.vec3f(local.x + v.tint.w, local.y, local.z),
    heart: d.vec3f(
      v.pose.x + t.x * (heart * s + KICK_RADII * trunk * landed),
      v.pose.y + t.y * (heart * s + KICK_RADII * trunk * landed),
      v.pose.z + trunk * landed + hinge * (1 - landed) + heart * c,
    ),
  };
});
