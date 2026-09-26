// FogTerm: the one fog-of-war term every world material applies, in the HDR
// world before post. `fogTerm(world, normal, pixel, isGround)` says how seen a
// fragment is; `unseenLook` is what unseen looks like.
//
// The signature is slice 14's (spike 02): per-pixel sight lights probe ground
// at the simulation's target height and faces just outside along their normal,
// and cull eyes per screen tile by pixel. Until slice 14 replaces the source,
// the term reads the observing side's 8 m ground bitset, bilinear, as the old
// one-shader frame did, and ignores the last three arguments. Nothing else
// reads the bitset for drawing.
//
// Every world layer takes fog, structures included: with sight lights a
// building's faces must show sight shadows (spike 02, landmine 3).
import { tgpu, d, std } from "typegpu";
import type { FogField } from "../scene";
import type { GpuRegistry, GpuSlot } from "./registry";

const FogParams = d.struct({ cellM: d.f32, nx: d.u32, ny: d.u32, enabled: d.u32 });
/** What the drawn layer is: 1 for ground, 0 for faces standing on it. */
const FogLayer = d.struct({ ground: d.u32 });
export const fogLayout = tgpu.bindGroupLayout({
  params: { uniform: FogParams, visibility: ["fragment"] },
  layer: { uniform: FogLayer, visibility: ["fragment"] },
  bits: {
    storage: (n: number) => d.arrayOf(d.u32, n),
    access: "readonly",
    visibility: ["fragment"],
  },
});

/** How seen a fragment is: 0 unseen, 1 seen. */
export const fogTerm = tgpu.fn(
  [d.vec3f, d.vec3f, d.vec2f, d.bool],
  d.f32,
)((world, _normal, _pixel, _isGround) => {
  "use gpu";
  const fog = fogLayout.$.params;
  if (fog.enabled !== 1) {
    return 1;
  }
  const gx = world.x / fog.cellM - 0.5;
  const gy = world.y / fog.cellM - 0.5;
  const i0 = d.i32(std.floor(gx));
  const j0 = d.i32(std.floor(gy));
  const fx = gx - std.floor(gx);
  const fy = gy - std.floor(gy);
  let seen = d.f32(0);
  for (let dj = 0; dj < 2; dj++) {
    for (let di = 0; di < 2; di++) {
      const i = i0 + di;
      const j = j0 + dj;
      if (i >= 0 && j >= 0 && i < d.i32(fog.nx) && j < d.i32(fog.ny)) {
        const k = d.u32(j) * fog.nx + d.u32(i);
        if ((fogLayout.$.bits[k >> 5] & (d.u32(1) << (k & 31))) !== 0) {
          const wx = std.select(1 - fx, fx, di === 1);
          const wy = std.select(1 - fy, fy, dj === 1);
          seen = seen + wx * wy;
        }
      }
    }
  }
  return std.smoothstep(0.25, 0.75, seen);
});

/** Whether the layer being drawn is ground, for `fogTerm`'s last argument. */
export const fogIsGround = tgpu.fn(
  [],
  d.bool,
)(() => {
  "use gpu";
  return fogLayout.$.layer.ground === 1;
});

/** Unseen is darker and flatter, but its shading still reads. Slice 15 owns
 *  this look. */
export const unseenLook = tgpu.fn(
  [d.vec3f, d.f32],
  d.vec3f,
)((lit, seen) => {
  "use gpu";
  const grey = std.dot(lit, d.vec3f(0.3, 0.5, 0.2));
  const fogged = std.mul(std.mix(lit, d.vec3f(grey, grey, grey * 1.1), 0.45), 0.68);
  return std.mix(fogged, lit, seen);
});

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** The observing side's fog source: the bitset storage and one bind group per
 *  layer kind, `ground` (terrain) and `faces` (everything standing on it). */
export function createFogSource(root: Root, registry: GpuRegistry) {
  const params = registry.own(root.createBuffer(FogParams).$usage("uniform"));
  const layerOf = (ground: number) => {
    const layer = registry.own(root.createBuffer(FogLayer).$usage("uniform"));
    layer.write({ ground });
    return layer;
  };
  const layers = { ground: layerOf(1), faces: layerOf(0) };
  const createBits = (words: number) =>
    root.createBuffer(d.arrayOf(d.u32, Math.max(1, words))).$usage("storage");
  const bits: GpuSlot<ReturnType<typeof createBits>> = registry.slot();
  const groupOf = (layer: typeof layers.ground) =>
    root.createBindGroup(fogLayout, { params, layer, bits: bits.current! });
  let words = 0;
  const source = {
    ground: null as unknown as ReturnType<typeof groupOf>,
    faces: null as unknown as ReturnType<typeof groupOf>,
    set(fog: FogField | null) {
      if (!fog) {
        params.write({ cellM: 1, nx: 0, ny: 0, enabled: 0 });
        return;
      }
      if (fog.bits.length > words) grow(fog.bits.length);
      bits.current!.write(fog.bits.buffer as ArrayBuffer);
      params.write({ cellM: fog.cellM, nx: fog.nx, ny: fog.ny, enabled: 1 });
    },
  };
  const grow = (n: number) => {
    bits.set(createBits(n));
    words = n;
    source.ground = groupOf(layers.ground);
    source.faces = groupOf(layers.faces);
  };
  grow(1);
  source.set(null);
  return source;
}
export type FogSource = ReturnType<typeof createFogSource>;
