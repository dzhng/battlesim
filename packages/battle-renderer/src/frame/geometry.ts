// The one vertex shape every world and overlay mesh uses, the instance record
// that places a proxy, and the vertex stage both passes share.
import { tgpu, d, std, type TgpuBindGroup } from "typegpu";
import { VERTEX_FLOATS, type Mesh } from "../mesh";
import { PROXY_MESHES, type ProxyKind } from "../proxies";
import { typegpuCameraLayout } from "../world/camera";
import type { SceneInstance } from "../scene";
import type { GpuRegistry, GpuSlot } from "./registry";

const Vertex = d.unstruct({ position: d.float32x3, normal: d.float32x3, color: d.float32x4 });
const Instance = d.unstruct({ placement: d.float32x4, tint: d.float32x4 });
export const vertexLayout = tgpu.vertexLayout(d.disarrayOf(Vertex));
export const instanceLayout = tgpu.vertexLayout(d.disarrayOf(Instance), "instance");
export const meshAttribs = { ...vertexLayout.attrib, ...instanceLayout.attrib };

/** The frame's camera, bound at group 0 by every world and overlay draw. */
export type CameraGroup = TgpuBindGroup<(typeof typegpuCameraLayout)["entries"]>;

const INSTANCE_FLOATS = 8;
export const PROXY_KINDS = Object.keys(PROXY_MESHES) as ProxyKind[];

/** Rotates by the instance yaw and places it; static meshes use the identity
 *  instance. Reads the 48-float camera, the one group pinned to index 0. */
export const meshVertex = tgpu.vertexFn({
  in: {
    position: d.vec3f,
    normal: d.vec3f,
    color: d.vec4f,
    placement: d.vec4f,
    tint: d.vec4f,
  },
  out: {
    // Invariant, so the depth prepass and the colour pass compute the same
    // depth for the same vertex and equal-depth tests pass.
    // (TypeGPU 0.12.5 emits the attribute but its vertexFn types omit it.)
    clip: d.invariant(d.builtin.position) as unknown as typeof d.builtin.position,
    world: d.vec3f,
    normal: d.vec3f,
    color: d.vec4f,
    highlight: d.f32,
  },
})((v) => {
  "use gpu";
  const c = std.cos(v.placement.w);
  const s = std.sin(v.placement.w);
  const local = v.position;
  const world = d.vec3f(
    local.x * c - local.y * s + v.placement.x,
    local.x * s + local.y * c + v.placement.y,
    local.z + v.placement.z,
  );
  const normal = d.vec3f(
    v.normal.x * c - v.normal.y * s,
    v.normal.x * s + v.normal.y * c,
    v.normal.z,
  );
  return {
    clip: std.mul(typegpuCameraLayout.$.cam.viewProj, d.vec4f(world, 1)),
    world,
    normal,
    color: d.vec4f(std.mul(v.color.xyz, v.tint.xyz), v.color.w),
    highlight: v.tint.w,
  };
});

type Root = ReturnType<typeof tgpu.initFromDevice>;
function vertexBuffer(root: Root, floats: Mesh) {
  const count = floats.length / VERTEX_FLOATS;
  const buffer = root
    .createBuffer(vertexLayout.schemaForCount(Math.max(1, count)))
    .$usage("vertex");
  if (count > 0) buffer.write(floats.buffer);
  return buffer;
}
type VertexBuffer = ReturnType<typeof vertexBuffer>;
type InstanceBuffer = ReturnType<typeof instanceBuffer>;
function instanceBuffer(root: Root, capacity: number) {
  return root.createBuffer(instanceLayout.schemaForCount(capacity)).$usage("vertex");
}

/** Anything a bound pipeline can draw a mesh through. */
interface Drawable {
  with(layout: typeof vertexLayout, buffer: VertexBuffer): Drawable;
  with(layout: typeof instanceLayout, buffer: InstanceBuffer): Drawable;
  draw(vertices: number, instances: number): void;
}

/** One replaceable static mesh, drawn once through the identity instance. */
export class MeshSlot {
  private readonly slot: GpuSlot<VertexBuffer>;
  private count = 0;

  constructor(
    private readonly root: Root,
    registry: GpuRegistry,
    private readonly identity: InstanceBuffer,
  ) {
    this.slot = registry.slot();
  }

  get vertices(): number {
    return this.count;
  }

  set(floats: Mesh) {
    this.slot.set(vertexBuffer(this.root, floats));
    this.count = floats.length / VERTEX_FLOATS;
  }

  draw(bound: Drawable) {
    const buffer = this.slot.current;
    if (!buffer || this.count === 0) return;
    bound.with(vertexLayout, buffer).with(instanceLayout, this.identity).draw(this.count, 1);
  }
}

/** The instanced proxies (units and props), one buffer per kind. */
export class ProxyInstances {
  private readonly meshes: Record<ProxyKind, VertexBuffer>;
  private readonly slots = new Map<
    ProxyKind,
    { buffer: GpuSlot<InstanceBuffer>; capacity: number; count: number }
  >();
  private total = 0;

  constructor(
    private readonly root: Root,
    private readonly registry: GpuRegistry,
  ) {
    this.meshes = Object.fromEntries(
      PROXY_KINDS.map((kind) => [kind, registry.own(vertexBuffer(root, PROXY_MESHES[kind]))]),
    ) as Record<ProxyKind, VertexBuffer>;
  }

  get count(): number {
    return this.total;
  }

  set(list: readonly SceneInstance[]) {
    this.total = list.length;
    for (const kind of PROXY_KINDS) {
      const ofKind = list.filter((i) => i.kind === kind);
      let slot = this.slots.get(kind);
      if (ofKind.length === 0) {
        if (slot) slot.count = 0;
        continue;
      }
      if (!slot || slot.capacity < ofKind.length) {
        const capacity = Math.max(8, ofKind.length * 2);
        const buffer = slot?.buffer ?? this.registry.slot<InstanceBuffer>();
        buffer.set(instanceBuffer(this.root, capacity));
        slot = { buffer, capacity, count: 0 };
        this.slots.set(kind, slot);
      }
      const data = new Float32Array(ofKind.length * INSTANCE_FLOATS);
      ofKind.forEach((inst, i) => {
        data.set(
          [inst.x, inst.y, inst.z, inst.yaw, ...inst.color, inst.highlight ? 1 : 0],
          i * INSTANCE_FLOATS,
        );
      });
      slot.buffer.current!.write(data.buffer);
      slot.count = ofKind.length;
    }
  }

  draw(bound: Drawable) {
    for (const kind of PROXY_KINDS) {
      const slot = this.slots.get(kind);
      if (!slot || slot.count === 0) continue;
      bound
        .with(vertexLayout, this.meshes[kind])
        .with(instanceLayout, slot.buffer.current!)
        .draw(PROXY_MESHES[kind].length / VERTEX_FLOATS, slot.count);
    }
  }
}

/** The identity instance every static mesh draws through. */
export function identityInstance(root: Root, registry: GpuRegistry): InstanceBuffer {
  const identity = registry.own(instanceBuffer(root, 1));
  identity.write(Float32Array.of(0, 0, 0, 0, 1, 1, 1, 0).buffer);
  return identity;
}
