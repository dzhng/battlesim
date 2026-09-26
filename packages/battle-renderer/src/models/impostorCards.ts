// Impostor cards: a model too small on screen for its coarsest mesh draws as
// one quad cut from its impostor atlas (`impostor.ts`), relit like the world.
//
// Each skinned appearance carries two atlases at runtime, baked at install by
// the frame's own model path: its far pose (a standing soldier) and its corpse.
// Every atlas is a ring of yaws at the battle camera's pitches; a card picks
// the cell nearest the direction it is seen from, relative to the model's
// heading, and stands square to that cell's view, so the card shows exactly
// the pixels that view baked. Albedo is display-encoded with coverage in
// alpha; the normal atlas carries the model-space normal and, in alpha, how
// much of the side's tint the surface takes, so one atlas serves both sides.
//
// A card is alpha-tested, not blended: it writes depth in the colour pass
// (at the size a card is drawn, a missing prepass entry never moves a fog
// tile's eyes) and casts no sun shadow.
import { tgpu, d, std } from "typegpu";
import { typegpuCameraLayout } from "../world/camera";
import type { EnvironmentFrame } from "../frame/environmentFrame";
import { fogCoverage } from "../frame/fogTerm";
import { WORLD_OUT } from "../frame/targets";
import { modelSeen } from "./modelFog";
import type { GpuRegistry } from "../frame/registry";
import type { ImpostorAtlas, ImpostorSpec } from "./impostor";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** The runtime atlases' views: eight headings at the camera's far pitch
 *  (the pitch curve's 0.85 rad plateau, where cards are drawn) and a lower
 *  one, at 64 px a cell (a card is drawn under `impostor_px`, a few pixels). */
export const CARD_SPEC: ImpostorSpec = {
  yaws: 8,
  pitches: [0.85, 0.5],
  cell: 64,
  supersample: 2,
};
const YAWS = CARD_SPEC.yaws;
const CELL_YAW = (2 * Math.PI) / YAWS;
const PITCH_HIGH = CARD_SPEC.pitches[0];
const PITCH_LOW = CARD_SPEC.pitches[1];
const PITCH_SPLIT = (PITCH_HIGH + PITCH_LOW) / 2;
/** A card's surface: rough cloth and kit. */
const CARD_ROUGHNESS = 0.85;
/** Selection glow, as the models'. */
const HIGHLIGHT = [0.95, 0.8, 0.2] as const;

export const cardLayout = tgpu.bindGroupLayout({
  albedo: { texture: d.texture2dArray(), visibility: ["fragment"] },
  normal: { texture: d.texture2dArray(), visibility: ["fragment"] },
  linear: { sampler: "filtering", visibility: ["fragment"] },
  /** Per atlas layer: the atlas frame's centre in model space (xyz) and radius. */
  frames: {
    storage: (n: number) => d.arrayOf(d.vec4f, n),
    access: "readonly",
    visibility: ["vertex"],
  },
});

/** Cards read the models' 48-byte record: placement, data (highlight in w)
 *  and tint (the atlas layer in w). */
export const cardVertex = tgpu.vertexFn({
  in: {
    vertex: d.builtin.vertexIndex,
    placement: d.vec4f,
    data: d.vec4f,
    tint: d.vec4f,
  },
  out: {
    clip: d.builtin.position,
    world: d.vec3f,
    uv: d.vec2f,
    layer: d.interpolate("flat", d.u32),
    yaw: d.interpolate("flat", d.f32),
    tint: d.vec3f,
    highlight: d.f32,
    anchor: d.vec3f,
  },
})((v) => {
  "use gpu";
  const layer = d.u32(v.tint.w);
  const frame = cardLayout.$.frames[layer];
  const yaw = v.placement.w;
  const c = std.cos(yaw);
  const s = std.sin(yaw);
  const center = d.vec3f(
    v.placement.x + frame.x * c - frame.y * s,
    v.placement.y + frame.x * s + frame.y * c,
    v.placement.z + frame.z,
  );
  const toEye = std.sub(typegpuCameraLayout.$.cam.eye, center);
  // The baked view nearest the eye's direction, in the model's own frame.
  const heading = std.atan2(toEye.y, toEye.x) - yaw;
  let column = std.round(heading / CELL_YAW);
  column = column - YAWS * std.floor(column / YAWS);
  const elevation = std.atan2(toEye.z, std.length(d.vec2f(toEye.x, toEye.y)));
  let row = d.f32(1);
  let pitch = d.f32(PITCH_LOW);
  if (elevation > PITCH_SPLIT) {
    row = 0;
    pitch = PITCH_HIGH;
  }
  const cellYaw = column * CELL_YAW + yaw;
  const cp = std.cos(pitch);
  const toward = d.vec3f(cp * std.cos(cellYaw), cp * std.sin(cellYaw), std.sin(pitch));
  // The bake's look-at basis: right = up × toward, up' = toward × right.
  const right = std.normalize(std.cross(d.vec3f(0, 0, 1), toward));
  const up = std.cross(toward, right);
  // Two triangles: corners 0 1 2, 0 2 3 of (−1,−1) (1,−1) (1,1) (−1,1).
  let k = v.vertex;
  if (k === 3) {
    k = 0;
  }
  if (k > 3) {
    k = k - 2;
  }
  let u = d.f32(-1);
  if (k === 1 || k === 2) {
    u = 1;
  }
  let w = d.f32(-1);
  if (k >= 2) {
    w = 1;
  }
  const world = std.add(center, std.mul(std.add(std.mul(right, u), std.mul(up, w)), frame.w));
  return {
    clip: std.mul(typegpuCameraLayout.$.cam.viewProj, d.vec4f(world, 1)),
    world,
    uv: d.vec2f((column + 0.5 + 0.5 * u) / YAWS, (row + 0.5 - 0.5 * w) / 2),
    layer,
    yaw,
    tint: v.tint.xyz,
    highlight: v.data.w,
    anchor: v.placement.xyz,
  };
});

const cardVaryings = {
  clip: d.builtin.position,
  world: d.vec3f,
  uv: d.vec2f,
  layer: d.interpolate("flat", d.u32),
  yaw: d.interpolate("flat", d.f32),
  tint: d.vec3f,
  highlight: d.f32,
  anchor: d.vec3f,
};

export function createCardFragment(environment: EnvironmentFrame) {
  return tgpu.fragmentFn({ in: cardVaryings, out: WORLD_OUT })((v) => {
    "use gpu";
    // Both samples before the test: sampling needs uniform control flow.
    const albedo = std.textureSample(cardLayout.$.albedo, cardLayout.$.linear, v.uv, v.layer);
    const packed = std.textureSample(cardLayout.$.normal, cardLayout.$.linear, v.uv, v.layer);
    if (albedo.w < 0.5) {
      std.discard();
    }
    const local = std.sub(std.mul(packed.xyz, 2), d.vec3f(1));
    const c = std.cos(v.yaw);
    const s = std.sin(v.yaw);
    let n = std.normalize(d.vec3f(local.x * c - local.y * s, local.x * s + local.y * c, local.z));
    const eye = typegpuCameraLayout.$.cam.eye;
    if (std.dot(n, std.sub(eye, v.world)) < 0) {
      n = std.neg(n);
    }
    // The atlas stores display colour; light wants it linear, then the side's tint.
    const linear = std.pow(std.max(albedo.xyz, d.vec3f(0)), d.vec3f(2.2));
    const tinted = std.mul(linear, std.mix(d.vec3f(1), v.tint, packed.w));
    const sun = environment.sampleSunShadow(v.world, n, v.clip.xy);
    const shaded = environment.shade(
      tinted,
      d.vec3f(0),
      CARD_ROUGHNESS,
      0,
      0,
      1,
      n,
      v.world,
      sun,
      eye,
    );
    const glow = std.mul(d.vec3f(HIGHLIGHT[0], HIGHLIGHT[1], HIGHLIGHT[2]), v.highlight * 0.7);
    const seen = modelSeen(v.world, n, v.anchor, v.clip.xy);
    return { color: d.vec4f(std.add(shaded.xyz, glow), 1), fog: fogCoverage(seen, 1) };
  });
}

/** Where an atlas layer's card stands in its model's frame. */
export interface CardFrame {
  center: readonly [number, number, number];
  radius: number;
}

/** The runtime atlases on the GPU: one array layer per baked atlas, and the
 *  bind group cards draw through. */
export function uploadCardAtlases(
  root: Root,
  scope: GpuRegistry,
  atlases: readonly ImpostorAtlas[],
) {
  const width = CARD_SPEC.yaws * CARD_SPEC.cell;
  const height = CARD_SPEC.pitches.length * CARD_SPEC.cell;
  const layers = Math.max(1, atlases.length);
  const texture = () =>
    scope.own(
      root.createTexture({ size: [width, height, layers], format: "rgba8unorm" }).$usage("sampled"),
    );
  const albedo = texture();
  const normal = texture();
  const device = root.device;
  atlases.forEach((atlas, layer) => {
    if (atlas.width !== width || atlas.height !== height)
      throw new Error(`card atlas ${atlas.appearance} is ${atlas.width}×${atlas.height}`);
    // The normal layer's alpha is the tint mask; coverage stays in albedo's.
    const packed = new Uint8Array(atlas.normal);
    for (let i = 3; i < packed.length; i += 4) packed[i] = atlas.mask[i - 3];
    for (const [target, bytes] of [
      [albedo, atlas.albedo],
      [normal, packed],
    ] as const)
      device.queue.writeTexture(
        { texture: root.unwrap(target), origin: [0, 0, layer] },
        bytes,
        { bytesPerRow: width * 4, rowsPerImage: height },
        [width, height, 1],
      );
  });
  const frames = scope.own(root.createBuffer(d.arrayOf(d.vec4f, layers)).$usage("storage"));
  frames.write(
    Array.from({ length: layers }, (_, i) => {
      const a = atlases[i];
      return a ? d.vec4f(a.center[0], a.center[1], a.center[2], a.radius) : d.vec4f(0, 0, 0, 1);
    }),
  );
  const linear = root.createSampler({
    minFilter: "linear",
    magFilter: "linear",
    addressModeU: "clamp-to-edge",
    addressModeV: "clamp-to-edge",
  });
  return root.createBindGroup(cardLayout, {
    albedo: albedo.createView(d.texture2dArray()),
    normal: normal.createView(d.texture2dArray()),
    linear,
    frames,
  });
}
export type CardGroup = ReturnType<typeof uploadCardAtlases>;
