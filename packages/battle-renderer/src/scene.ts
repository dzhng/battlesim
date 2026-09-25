import { tgpu, d, std } from "typegpu";
import { cameraUniformData, type CameraSnapshot } from "@packages/renderer-core/src/cameraUniform";
import {
  GPU_DEPTH_CLEAR,
  GPU_DEPTH_COMPARE,
  GPU_DEPTH_FORMAT,
} from "@packages/renderer-core/src/depthContract";
import { VERTEX_FLOATS, type Mesh } from "./mesh";
import { PROXY_MESHES, type ProxyKind } from "./proxies";

/** A drawn proxy: world placement plus presentation tint. */
export interface SceneInstance {
  kind: ProxyKind;
  x: number;
  y: number;
  z: number;
  /** Heading of local +X, radians counter-clockwise from world +X. */
  yaw: number;
  color: readonly [number, number, number];
  highlight?: boolean;
}

/** Static world geometry in world space. Translucent triangles draw after
 *  everything opaque without writing depth. */
export interface WorldMeshes {
  opaque: Mesh;
  translucent: Mesh;
}

export interface InstalledDepthState {
  format: GPUTextureFormat;
  clearValue: number;
  compare: GPUCompareFunction;
}

export interface SceneStats {
  width: number;
  height: number;
  frames: number;
  instances: number;
  worldVertices: number;
  depth: InstalledDepthState;
}

export interface BattleScene {
  render(target: GPUTextureView, camera: CameraSnapshot): void;
  resize(width: number, height: number): void;
  setWorld(world: WorldMeshes): void;
  setInstances(instances: readonly SceneInstance[]): void;
  stats(): SceneStats;
  dispose(): void;
}

/** WGSL mirror of renderer-core cameraUniform packing. */
export const Camera = d.struct({
  viewProj: d.mat4x4f,
  invViewProj: d.mat4x4f,
  eye: d.vec3f,
  znear: d.f32,
  width: d.f32,
  height: d.f32,
  pad0: d.f32,
  pad1: d.f32,
});
const cameraLayout = tgpu.bindGroupLayout({
  cam: { uniform: Camera, visibility: ["vertex", "fragment"] },
});

const Vertex = d.unstruct({ position: d.float32x3, normal: d.float32x3, color: d.float32x4 });
const Instance = d.unstruct({ placement: d.float32x4, tint: d.float32x4 });
const vertexLayout = tgpu.vertexLayout(d.disarrayOf(Vertex));
const instanceLayout = tgpu.vertexLayout(d.disarrayOf(Instance), "instance");

const INSTANCE_FLOATS = 8;
// 4× is the sample count WebGPU core guarantees for every renderable format.
const MSAA_SAMPLES = 4;
const KINDS = Object.keys(PROXY_MESHES) as ProxyKind[];
const SUN = [0.5, -0.55, 0.67] as const;
const SKY: GPUColor = [0.55, 0.64, 0.72, 1];

const vertex = tgpu.vertexFn({
  in: {
    position: d.vec3f,
    normal: d.vec3f,
    color: d.vec4f,
    placement: d.vec4f,
    tint: d.vec4f,
  },
  out: {
    clip: d.builtin.position,
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
    clip: std.mul(cameraLayout.$.cam.viewProj, d.vec4f(world, 1)),
    world,
    normal,
    color: d.vec4f(std.mul(v.color.xyz, v.tint.xyz), v.color.w),
    highlight: v.tint.w,
  };
});

const fragment = tgpu.fragmentFn({
  in: { world: d.vec3f, normal: d.vec3f, color: d.vec4f, highlight: d.f32 },
  out: d.vec4f,
})((v) => {
  "use gpu";
  const toEye = std.sub(cameraLayout.$.cam.eye, v.world);
  let n = std.normalize(v.normal);
  if (std.dot(n, toEye) < 0) {
    n = std.neg(n);
  }
  const light = std.max(std.dot(n, std.normalize(d.vec3f(SUN[0], SUN[1], SUN[2]))), 0);
  const shaded = std.mul(v.color.xyz, 0.3 + 0.75 * light);
  const lit = std.add(shaded, std.mul(d.vec3f(0.95, 0.8, 0.2), v.highlight * 0.7));
  return d.vec4f(lit, v.color.w);
});

// The installed depth state is read back from these descriptors, not restated.
const depthStencil = {
  format: GPU_DEPTH_FORMAT,
  depthWriteEnabled: true,
  depthCompare: GPU_DEPTH_COMPARE,
} satisfies GPUDepthStencilState;
const depthClearValue = GPU_DEPTH_CLEAR;

/** Caller owns the device and canvas; the scene owns every allocation it makes. */
export async function createScene(
  device: GPUDevice,
  format: GPUTextureFormat,
  world: WorldMeshes,
  instances: readonly SceneInstance[],
): Promise<BattleScene> {
  const root = tgpu.initFromDevice({ device });
  // TypeGPU's root.destroy() does not free buffers it created, so every
  // allocation is registered and released explicitly.
  const owned: { destroy(): void }[] = [];
  const own = <T extends { destroy(): void }>(resource: T): T => {
    owned.push(resource);
    return resource;
  };
  const instanceBuffers = new Map<
    ProxyKind,
    { buffer: InstanceBuffer; capacity: number; count: number }
  >();
  let depthTexture: GPUTexture | null = null;
  let colorTexture: GPUTexture | null = null;
  let disposed = false;
  const check = () => {
    if (disposed) throw new Error("Battle scene disposed");
  };
  const dispose = () => {
    if (disposed) return;
    disposed = true;
    depthTexture?.destroy();
    colorTexture?.destroy();
    for (const slot of instanceBuffers.values()) slot.buffer.destroy();
    for (const mesh of Object.values(worldBuffers)) mesh?.buffer.destroy();
    for (const resource of owned) resource.destroy();
    root.destroy();
  };

  type InstanceBuffer = ReturnType<typeof createInstanceBuffer>;
  function createInstanceBuffer(capacity: number) {
    return root.createBuffer(instanceLayout.schemaForCount(capacity)).$usage("vertex");
  }
  const createVertexBuffer = (floats: Mesh) => {
    const count = floats.length / VERTEX_FLOATS;
    const buffer = root
      .createBuffer(vertexLayout.schemaForCount(Math.max(1, count)))
      .$usage("vertex");
    if (count > 0) buffer.write(floats.buffer);
    return { buffer, count };
  };
  type VertexBuffer = ReturnType<typeof createVertexBuffer>;
  const worldBuffers: { opaque?: VertexBuffer; translucent?: VertexBuffer } = {};

  const allocate = async () => {
    const cameraBuffer = own(root.createBuffer(Camera)).$usage("uniform");
    const identity = own(root.createBuffer(instanceLayout.schemaForCount(1))).$usage("vertex");
    identity.write(Float32Array.of(0, 0, 0, 0, 1, 1, 1, 0).buffer);
    const base = {
      attribs: { ...vertexLayout.attrib, ...instanceLayout.attrib },
      vertex,
      fragment,
      primitive: { topology: "triangle-list", cullMode: "none" },
      multisample: { count: MSAA_SAMPLES },
    } as const;
    const opaque = root.createRenderPipeline({ ...base, targets: { format }, depthStencil });
    const translucent = root.createRenderPipeline({
      ...base,
      targets: {
        format,
        blend: {
          color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
          alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
        },
      },
      depthStencil: { ...depthStencil, depthWriteEnabled: false },
    });
    await Promise.all([opaque.initAsync(), translucent.initAsync()]);
    return {
      cameraBuffer,
      cameraGroup: root.createBindGroup(cameraLayout, { cam: cameraBuffer }),
      identity,
      proxies: Object.fromEntries(
        KINDS.map((kind) => [kind, own(createVertexBuffer(PROXY_MESHES[kind]).buffer)]),
      ) as Record<ProxyKind, VertexBuffer["buffer"]>,
      pipelines: { opaque, translucent },
    };
  };
  const { cameraBuffer, cameraGroup, identity, proxies, pipelines } = await allocate().catch(
    (error) => {
      dispose();
      throw error;
    },
  );
  const proxyCounts = Object.fromEntries(
    KINDS.map((kind) => [kind, PROXY_MESHES[kind].length / VERTEX_FLOATS]),
  ) as Record<ProxyKind, number>;

  let width = 1;
  let height = 1;
  let frames = 0;
  let instanceCount = 0;

  function resize(w: number, h: number) {
    check();
    const nw = Math.max(1, Math.floor(w));
    const nh = Math.max(1, Math.floor(h));
    if (depthTexture && nw === width && nh === height) return;
    depthTexture?.destroy();
    colorTexture?.destroy();
    width = nw;
    height = nh;
    depthTexture = device.createTexture({
      label: "battle-scene-depth",
      size: [width, height],
      format: GPU_DEPTH_FORMAT,
      sampleCount: MSAA_SAMPLES,
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    });
    colorTexture = device.createTexture({
      label: "battle-scene-msaa-color",
      size: [width, height],
      format,
      sampleCount: MSAA_SAMPLES,
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    });
  }

  function setWorld(next: WorldMeshes) {
    check();
    for (const key of ["opaque", "translucent"] as const) {
      worldBuffers[key]?.buffer.destroy();
      worldBuffers[key] = createVertexBuffer(next[key]);
    }
  }

  function setInstances(list: readonly SceneInstance[]) {
    check();
    instanceCount = list.length;
    for (const kind of KINDS) {
      const ofKind = list.filter((i) => i.kind === kind);
      let slot = instanceBuffers.get(kind);
      if (ofKind.length === 0) {
        if (slot) slot.count = 0;
        continue;
      }
      if (!slot || slot.capacity < ofKind.length) {
        slot?.buffer.destroy();
        const capacity = Math.max(8, ofKind.length * 2);
        slot = { buffer: createInstanceBuffer(capacity), capacity, count: 0 };
        instanceBuffers.set(kind, slot);
      }
      const data = new Float32Array(ofKind.length * INSTANCE_FLOATS);
      ofKind.forEach((inst, i) => {
        data.set(
          [inst.x, inst.y, inst.z, inst.yaw, ...inst.color, inst.highlight ? 1 : 0],
          i * INSTANCE_FLOATS,
        );
      });
      slot.buffer.write(data.buffer);
      slot.count = ofKind.length;
    }
  }

  setWorld(world);
  setInstances(instances);

  return {
    render(target, camera) {
      check();
      if (!depthTexture || camera.width !== width || camera.height !== height) {
        resize(camera.width, camera.height);
      }
      cameraBuffer.write(cameraUniformData(camera).buffer);
      const encoder = device.createCommandEncoder({ label: "battle-scene" });
      const pass = encoder.beginRenderPass({
        colorAttachments: [
          {
            view: colorTexture!.createView(),
            resolveTarget: target,
            loadOp: "clear",
            storeOp: "discard",
            clearValue: SKY,
          },
        ],
        depthStencilAttachment: {
          view: depthTexture!.createView(),
          depthClearValue,
          depthLoadOp: "clear",
          depthStoreOp: "store",
        },
      });
      const opaque = pipelines.opaque.with(pass).with(cameraGroup);
      const drawWorld = (bound: typeof opaque, mesh: VertexBuffer | undefined) => {
        if (mesh && mesh.count > 0) {
          bound.with(vertexLayout, mesh.buffer).with(instanceLayout, identity).draw(mesh.count, 1);
        }
      };
      drawWorld(opaque, worldBuffers.opaque);
      for (const kind of KINDS) {
        const slot = instanceBuffers.get(kind);
        if (!slot || slot.count === 0) continue;
        opaque
          .with(vertexLayout, proxies[kind])
          .with(instanceLayout, slot.buffer)
          .draw(proxyCounts[kind], slot.count);
      }
      drawWorld(pipelines.translucent.with(pass).with(cameraGroup), worldBuffers.translucent);
      pass.end();
      device.queue.submit([encoder.finish()]);
      frames++;
    },
    resize,
    setWorld,
    setInstances,
    stats: () => ({
      width,
      height,
      frames,
      instances: instanceCount,
      worldVertices: (worldBuffers.opaque?.count ?? 0) + (worldBuffers.translucent?.count ?? 0),
      depth: {
        format: depthTexture?.format ?? depthStencil.format,
        clearValue: depthClearValue,
        compare: depthStencil.depthCompare,
      },
    }),
    dispose,
  };
}
