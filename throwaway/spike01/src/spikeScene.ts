// SPIKE 01 (throwaway): the ported renderer foundation driving our world.
// Implements our BattleScene seam so LabViewport can swap it in unchanged.
//
// Frame: [shadows: N cascades] -> [sky bg + HDR world, MSAA 4, rgba16float]
//        -> [post: bloom + grade + AgX -> canvas] -> [overlays in display space,
//        MSAA 4 against world depth, resolved, composited over the canvas].
import { tgpu, d, std, common } from "typegpu";
import type { CameraSnapshot } from "@packages/renderer-core/src/cameraUniform";
import { VERTEX_FLOATS, type Mesh } from "@packages/battle-renderer/src/mesh";
import { PROXY_MESHES, type ProxyKind } from "@packages/battle-renderer/src/proxies";
import type {
  BattleScene,
  FogField,
  SceneInstance,
  WorldMeshes,
} from "@packages/battle-renderer/src/scene";
import { Camera, typegpuCameraLayout } from "../port/packages/battle-renderer/src/world/camera";
import { createTypegpuEnvironment } from "../port/packages/battle-renderer/src/world/environment";
import { createTypegpuSunShadow } from "../port/packages/battle-renderer/src/world/shadow";
import { createTypegpuPost } from "../port/packages/battle-renderer/src/world/post";
import { frameCamera } from "../port/packages/battle-renderer/src/frameCamera";
import { battleWorldDepth, BATTLE_DEPTH_ATTACHMENT } from "../port/packages/battle-renderer/src/worldDepth";
import { battlePostGrade } from "../port/packages/game-renderer/src/environment/postParameters";
import {
  CIVSIM_ENVIRONMENTS,
  type CivsimEnvironment,
} from "../port/packages/game-renderer/src/environment/environment";
import { screenRay, viewMatrix, type Camera3DParams } from "../port/packages/renderer-core/src/camera3d";
import type { ShadowTuning } from "../port/packages/battle-renderer/src/shadowData";

const MSAA = 4;
class SpikeDisposed extends Error {}
const INSTANCE_FLOATS = 8;
const KINDS = Object.keys(PROXY_MESHES) as ProxyKind[];
const OVERLAY_FORMAT = "rgba8unorm" as const;

/** A warm afternoon on the golden preset's physical block; aerial range pushed
 *  out to our 1.6 km map (the source tunes it for miniature metres). */
export function spikeEnvironment(): CivsimEnvironment {
  const golden = CIVSIM_ENVIRONMENTS.golden;
  return {
    ...golden,
    sunAzimuth: -0.35,
    sunElevation: 0.62,
    physical: {
      ...golden.physical,
      sunIntensity: 5.0,
      environmentIntensity: 0.45,
      aerial: {
        ...golden.physical.aerial,
        clearRadiusM: 300,
        rangeFogNearM: 500,
        rangeFogFarM: 5000,
        rangeFogStrength: 0.25,
        valleyMistDistanceStartM: 2000,
        valleyMistDistanceFullM: 5000,
      },
    },
  };
}

const Vertex = d.unstruct({ position: d.float32x3, normal: d.float32x3, color: d.float32x4 });
const Instance = d.unstruct({ placement: d.float32x4, tint: d.float32x4 });
const vertexLayout = tgpu.vertexLayout(d.disarrayOf(Vertex));
const instanceLayout = tgpu.vertexLayout(d.disarrayOf(Instance), "instance");

/** The 8 m fog bitset, as today (FogTerm's short-lived source). */
const FogParams = d.struct({ cellM: d.f32, nx: d.u32, ny: d.u32, enabled: d.u32 });
const fogLayout = tgpu.bindGroupLayout({
  params: { uniform: FogParams, visibility: ["fragment"] },
  bits: {
    storage: (n: number) => d.arrayOf(d.u32, n),
    access: "readonly",
    visibility: ["fragment"],
  },
});

const overlaySourceLayout = tgpu.bindGroupLayout({
  overlay: { texture: d.texture2d(), visibility: ["fragment"] },
});

const vertexIn = {
  position: d.vec3f,
  normal: d.vec3f,
  color: d.vec4f,
  placement: d.vec4f,
  tint: d.vec4f,
};
const vertexOut = {
  clip: d.builtin.position,
  world: d.vec3f,
  normal: d.vec3f,
  color: d.vec4f,
  highlight: d.f32,
};
/** Our instanced proxy/world vertex, reading the 48-float camera at group 0. */
const vertex = tgpu.vertexFn({ in: vertexIn, out: vertexOut })((v) => {
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

/** FogTerm: 0 = unseen, 1 = seen, bilinear over the 8 m cells. */
const fogSeen = tgpu.fn(
  [d.vec3f],
  d.f32,
)((world) => {
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

const srgbToLinear = tgpu.fn(
  [d.vec3f],
  d.vec3f,
)((c) => {
  "use gpu";
  return std.pow(std.max(c, d.vec3f(0)), d.vec3f(2.2));
});

/** Today's flat overlay shading, unchanged, so colours match the old frame. */
const overlayFragment = tgpu.fragmentFn({
  in: { world: d.vec3f, normal: d.vec3f, color: d.vec4f, highlight: d.f32 },
  out: d.vec4f,
})((v) => {
  "use gpu";
  const toEye = std.sub(typegpuCameraLayout.$.cam.eye, v.world);
  let n = std.normalize(v.normal);
  if (std.dot(n, toEye) < 0) {
    n = std.neg(n);
  }
  const light = std.max(std.dot(n, std.normalize(d.vec3f(0.5, -0.55, 0.67))), 0);
  const shaded = std.mul(v.color.xyz, 0.3 + 0.75 * light);
  const lit = std.add(shaded, std.mul(d.vec3f(0.95, 0.8, 0.2), v.highlight * 0.7));
  return d.vec4f(lit, v.color.w);
});

const compositeFragment = tgpu.fragmentFn({
  in: { pos: d.builtin.position },
  out: d.vec4f,
})((v) => {
  "use gpu";
  return std.textureLoad(overlaySourceLayout.$.overlay, d.vec2i(v.pos.xy), 0);
});

export interface SpikeStats {
  gpuMs: Record<string, number>;
  cascades: { extent: number; texel: number }[];
  splitNear: number;
  cappedFar: number;
}

export interface SpikeOptions {
  shadow?: Partial<ShadowTuning>;
  /** Called when async target creation finishes, so the viewport redraws. */
  requestRedraw?: () => void;
}

export async function createSpikeScene(
  device: GPUDevice,
  format: GPUTextureFormat,
  world: WorldMeshes,
  instances: readonly SceneInstance[],
  options: SpikeOptions = {},
): Promise<BattleScene & { spikeStats(): SpikeStats }> {
  const root = tgpu.initFromDevice({ device });
  const env = spikeEnvironment();
  const tuning: ShadowTuning = {
    mapSize: 2048,
    normalBiasScale: 1,
    depthBiasScale: 1,
    ...options.shadow,
  };
  // Every allocation is owned here and destroyed explicitly (root.destroy()
  // does not free what TypeGPU created).
  const owned: { destroy(): void }[] = [];
  const own = <T extends { destroy(): void }>(r: T): T => {
    owned.push(r);
    return r;
  };
  const shadow = createTypegpuSunShadow(device, env, tuning);
  owned.push({ destroy: shadow.dispose });
  const environment = await createTypegpuEnvironment(device, env, undefined, MSAA, shadow, true);
  owned.push({ destroy: environment.dispose });

  const camera = own(root.createBuffer(Camera).$usage("uniform"));
  const cameraGroup = root.createBindGroup(typegpuCameraLayout, { cam: camera });

  // World surface shading: the ported PBR + PMREM + aerial, our sRGB vertex
  // colours as albedo, the sun shadow, then FogTerm before post.
  const worldFragment = tgpu.fragmentFn({
    in: {
      clip: d.builtin.position,
      world: d.vec3f,
      normal: d.vec3f,
      color: d.vec4f,
      highlight: d.f32,
    },
    out: d.vec4f,
  })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    let n = std.normalize(v.normal);
    if (std.dot(n, std.sub(eye, v.world)) < 0) {
      n = std.neg(n);
    }
    const base = srgbToLinear(v.color.xyz);
    const sun = environment.sampleSunShadow(v.world, n, v.clip.xy);
    const lit = environment.shade(base, d.vec3f(0), 0.85, 0, 0, 1, n, v.world, sun, eye);
    let rgb = std.add(lit.xyz, std.mul(d.vec3f(0.95, 0.8, 0.2), v.highlight * 0.7));
    const seen = fogSeen(v.world);
    const grey = std.dot(rgb, d.vec3f(0.3, 0.5, 0.2));
    const fogged = std.mul(std.mix(rgb, d.vec3f(grey, grey, grey * 1.1), 0.45), 0.68);
    rgb = std.mix(fogged, rgb, seen);
    return d.vec4f(rgb, v.color.w);
  });

  const attribs = { ...vertexLayout.attrib, ...instanceLayout.attrib };
  const base = { attribs, vertex, primitive: { topology: "triangle-list", cullMode: "none" } } as const;
  const hdrOpaque = root.createRenderPipeline({
    ...base,
    fragment: worldFragment,
    targets: { format: "rgba16float" },
    depthStencil: battleWorldDepth("read-write"),
    multisample: { count: MSAA },
  });
  const hdrTranslucent = root.createRenderPipeline({
    ...base,
    fragment: worldFragment,
    targets: {
      format: "rgba16float",
      blend: {
        color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
        alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
      },
    },
    depthStencil: battleWorldDepth("read"),
    multisample: { count: MSAA },
  });
  const caster = root.createRenderPipeline({
    ...base,
    depthStencil: battleWorldDepth("read-write"),
  });
  // Premultiplied into a cleared target so the resolve averages correctly.
  const overlayBlend = {
    color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
    alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
  } as const;
  const overlayOpaque = root.createRenderPipeline({
    ...base,
    fragment: overlayFragment,
    targets: { format: OVERLAY_FORMAT, blend: overlayBlend },
    depthStencil: battleWorldDepth("read-write"),
    multisample: { count: MSAA },
  });
  const overlayTranslucent = root.createRenderPipeline({
    ...base,
    fragment: overlayFragment,
    targets: { format: OVERLAY_FORMAT, blend: overlayBlend },
    depthStencil: battleWorldDepth("read"),
    multisample: { count: MSAA },
  });
  const composite = root.createRenderPipeline({
    vertex: common.fullScreenTriangle,
    fragment: compositeFragment,
    targets: {
      format,
      blend: {
        color: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
        alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
      },
    },
  });
  await Promise.all(
    [hdrOpaque, hdrTranslucent, caster, overlayOpaque, overlayTranslucent, composite].map((p) =>
      p.initAsync(),
    ),
  );

  // Fog storage; fog-off group for nothing (overlays do not read fog).
  const fogParams = own(root.createBuffer(FogParams).$usage("uniform"));
  let fogBits = own(root.createBuffer(d.arrayOf(d.u32, 1)).$usage("storage"));
  let fogWords = 1;
  let fogGroup = root.createBindGroup(fogLayout, { params: fogParams, bits: fogBits });
  fogParams.write({ cellM: 1, nx: 0, ny: 0, enabled: 0 });
  // Knowledge-drawn structures read over fog (the side remembers them).
  const fogOffParams = own(root.createBuffer(FogParams).$usage("uniform"));
  fogOffParams.write({ cellM: 1, nx: 0, ny: 0, enabled: 0 });
  let fogOffGroup = root.createBindGroup(fogLayout, { params: fogOffParams, bits: fogBits });

  const identity = own(root.createBuffer(instanceLayout.schemaForCount(1)).$usage("vertex"));
  identity.write(Float32Array.of(0, 0, 0, 0, 1, 1, 1, 0).buffer);
  const vertexBuffer = (floats: Mesh) => {
    const count = floats.length / VERTEX_FLOATS;
    const buffer = root.createBuffer(vertexLayout.schemaForCount(Math.max(1, count))).$usage("vertex");
    if (count > 0) buffer.write(floats.buffer);
    return { buffer, count };
  };
  type VB = ReturnType<typeof vertexBuffer>;
  const proxies = Object.fromEntries(
    KINDS.map((k) => [k, own(vertexBuffer(PROXY_MESHES[k]).buffer)]),
  ) as Record<ProxyKind, VB["buffer"]>;
  const proxyCounts = Object.fromEntries(
    KINDS.map((k) => [k, PROXY_MESHES[k].length / VERTEX_FLOATS]),
  ) as Record<ProxyKind, number>;
  const worldBuffers: { opaque?: VB; translucent?: VB } = {};
  const overlayBuffers: { opaque?: VB; translucent?: VB; structures?: VB } = {};
  const instanceSlots = new Map<ProxyKind, { buffer: ReturnType<typeof mkInstances>; capacity: number; count: number }>();
  function mkInstances(capacity: number) {
    return root.createBuffer(instanceLayout.schemaForCount(capacity)).$usage("vertex");
  }

  // GPU timing: empty compute passes as timestamp markers between phases.
  const PHASES = ["shadow", "world", "post", "overlay"] as const;
  const timing = device.features.has("timestamp-query")
    ? (() => {
        const querySet = device.createQuerySet({ type: "timestamp", count: PHASES.length + 1 });
        const resolve = device.createBuffer({
          size: 8 * (PHASES.length + 1),
          usage: GPUBufferUsage.QUERY_RESOLVE | GPUBufferUsage.COPY_SRC,
        });
        const readbacks = Array.from({ length: 3 }, () => ({
          buffer: device.createBuffer({
            size: 8 * (PHASES.length + 1),
            usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
          }),
          busy: false,
        }));
        // A 1x1 cleared target: empty compute passes write no timestamps.
        const marker = device.createTexture({
          size: [1, 1],
          format: "r8unorm",
          usage: GPUTextureUsage.RENDER_ATTACHMENT,
        });
        return { querySet, resolve, readbacks, marker, markerView: marker.createView() };
      })()
    : null;
  if (timing) {
    owned.push(timing.querySet, timing.resolve, timing.marker, ...timing.readbacks.map((r) => r.buffer));
  }
  const gpuSums: Record<string, number> = {};
  let cpuSum = 0,
    cpuN = 0;
  const timingDebug: { enabled: boolean; last?: number[]; error?: string } = { enabled: !!timing };
  let gpuSamples = 0;
  const mark = (raw: GPUCommandEncoder, index: number) => {
    if (!timing) return;
    raw
      .beginRenderPass({
        colorAttachments: [{ view: timing.markerView, loadOp: "clear", storeOp: "store" }],
        timestampWrites: { querySet: timing.querySet, beginningOfPassWriteIndex: index },
      })
      .end();
  };

  let targets: ReturnType<typeof makeTargets> | null = null;
  let postChain: Awaited<ReturnType<typeof createTypegpuPost>> | null = null;
  let width = 0;
  let height = 0;
  let frames = 0;
  let instanceCount = 0;
  let disposed = false;
  function makeTargets(w: number, h: number) {
    const tex = (desc: GPUTextureDescriptor) => device.createTexture(desc);
    return {
      hdr: tex({
        label: "spike-hdr",
        size: [w, h],
        format: "rgba16float",
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
      }),
      color: tex({
        label: "spike-hdr-msaa",
        size: [w, h],
        format: "rgba16float",
        sampleCount: MSAA,
        usage: GPUTextureUsage.RENDER_ATTACHMENT,
      }),
      depth: tex({
        label: "spike-depth",
        size: [w, h],
        format: BATTLE_DEPTH_ATTACHMENT.format,
        sampleCount: MSAA,
        usage: GPUTextureUsage.RENDER_ATTACHMENT,
      }),
      overlayMsaa: tex({
        label: "spike-overlay-msaa",
        size: [w, h],
        format: OVERLAY_FORMAT,
        sampleCount: MSAA,
        usage: GPUTextureUsage.RENDER_ATTACHMENT,
      }),
      overlay: tex({
        label: "spike-overlay",
        size: [w, h],
        format: OVERLAY_FORMAT,
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
      }),
    };
  }
  let overlayGroup: ReturnType<typeof root.createBindGroup> | null = null;
  let readyResolve: () => void = () => {};
  const ready = new Promise<void>((r) => (readyResolve = r));
  const releaseTargets = () => {
    if (targets) for (const t of Object.values(targets)) t.destroy();
    postChain?.dispose();
    targets = null;
    postChain = null;
  };
  async function ensureTargets(w: number, h: number) {
    if (targets && w === width && h === height) return;
    releaseTargets();
    const next = makeTargets(w, h);
    let post: Awaited<ReturnType<typeof createTypegpuPost>>;
    try {
      post = await createTypegpuPost(device, next.hdr.createView(), w, h, format);
    } catch (error) {
      for (const t of Object.values(next)) t.destroy();
      throw error;
    }
    if (disposed) {
      for (const t of Object.values(next)) t.destroy();
      post.dispose();
      return;
    }
    post.setGrade(battlePostGrade("golden"), env.physical.exposure);
    width = w;
    height = h;
    targets = next;
    postChain = post;
    overlayGroup = root.createBindGroup(overlaySourceLayout, { overlay: next.overlay.createView() });
    readyResolve();
    options.requestRedraw?.();
  }

  /** Map box [maxX, maxY, minZ, maxZ] from the world mesh (terrain starts at 0,0). */
  let bounds: [number, number, number, number] | null = null;
  /** View-depth range of the map box, from a grid of screen rays. */
  function receiverRange(cam: Camera3DParams): [number, number] {
    if (!bounds) return [cam.near, Infinity];
    const [maxX, maxY, minZ, maxZ] = bounds;
    const lo = [0, 0, minZ - 1],
      hi = [maxX, maxY, maxZ + 25];
    const view = viewMatrix(cam);
    let near = Infinity,
      far = 0;
    const N = 12;
    for (let iy = 0; iy <= N; iy++)
      for (let ix = 0; ix <= N; ix++) {
        const ray = screenRay(cam, (ix / N) * 2 - 1, (iy / N) * 2 - 1);
        let t0 = 0,
          t1 = Infinity;
        for (let a = 0; a < 3; a++) {
          const o = ray.origin[a],
            dd = ray.dir[a];
          if (Math.abs(dd) < 1e-9) {
            if (o < lo[a] || o > hi[a]) t1 = -1;
            continue;
          }
          let ta = (lo[a] - o) / dd,
            tb = (hi[a] - o) / dd;
          if (ta > tb) [ta, tb] = [tb, ta];
          t0 = Math.max(t0, ta);
          t1 = Math.min(t1, tb);
        }
        if (t1 < t0) continue;
        // View depth of the ray points: -z in view space.
        const depthAt = (t: number) => {
          const p = [0, 1, 2].map((a) => ray.origin[a] + ray.dir[a] * t);
          return -(view[2] * p[0] + view[6] * p[1] + view[10] * p[2] + view[14]);
        };
        near = Math.min(near, depthAt(t0));
        far = Math.max(far, depthAt(t1));
      }
    if (!Number.isFinite(near)) return [cam.near, cam.near * 2];
    return [Math.max(cam.near, near), far];
  }

  let lastCascades: SpikeStats["cascades"] = [];
  let lastSplit = { near: 0, far: 0 };

  const drawMesh = (bound: { with: Function }, mesh: VB | undefined) => {
    if (mesh && mesh.count > 0)
      (bound as any).with(vertexLayout, mesh.buffer).with(instanceLayout, identity).draw(mesh.count, 1);
  };
  const drawInstances = (bound: any) => {
    for (const kind of KINDS) {
      const slot = instanceSlots.get(kind);
      if (!slot || slot.count === 0) continue;
      bound.with(vertexLayout, proxies[kind]).with(instanceLayout, slot.buffer).draw(proxyCounts[kind], slot.count);
    }
  };

  function check() {
    // LabViewport's loop may still call in while rebuild() awaits the next
    // scene; the old flat scene never drew then, so tolerate it here.
    if (disposed) throw new SpikeDisposed();
  }

  let pendingResize: Promise<void> | null = null;
  let structureFloats: Mesh = new Float32Array(0);
  const debug: { overlayOnly: false | "black" | "white" } = { overlayOnly: false };
  const scene = {
    render(target: GPUTextureView, snap: CameraSnapshot) {
      if (disposed) return;
      if (!targets || snap.width !== width || snap.height !== height) {
        // Targets are async (post pipelines); skip this frame and draw next.
        pendingResize ??= ensureTargets(Math.max(64, snap.width), Math.max(64, snap.height)).finally(
          () => (pendingResize = null),
        );
        return;
      }
      const cpu0 = performance.now();
      const cam3d = { ...snap.camera3d, aspect: snap.width / snap.height };
      const state = frameCamera(
        {
          camera3d: cam3d,
          x: cam3d.target[0],
          y: cam3d.target[1],
          zoom: 1,
          width: snap.width,
          height: snap.height,
          sunAzimuth: env.sunAzimuth,
          sunElevation: env.sunElevation,
        },
        width,
        height,
      );
      camera.write(state.bytes.buffer);
      environment.setView(state.view, cam3d.target as [number, number, number]);
      environment.sky.setRays(state.rays);
      const range = receiverRange(cam3d);
      const shadowData = shadow.update(cam3d, range);
      lastSplit = { near: shadowData.splitNear, far: shadowData.cappedFar };
      lastCascades = shadowData.cascades.map((c) => ({ extent: c.extent, texel: c.worldUnitsPerTexel }));

      const encoder = root["~unstable"].createCommandEncoder({ label: "spike-frame" });
      const raw = root.unwrap(encoder);
      const t = targets;
      mark(raw, 0);
      shadow.encode(encoder, (pass, cascade) => {
        const bound = caster.with(pass).with(shadow.cameraGroups[cascade]);
        drawMesh(bound, worldBuffers.opaque);
        drawMesh(bound, overlayBuffers.structures);
        drawInstances(bound);
      });
      mark(raw, 1);
      environment.sky.encodeBackground(raw, t.color.createView());
      const pass = encoder.beginRenderPass({
        colorAttachments: [
          { view: t.color.createView(), resolveTarget: t.hdr.createView(), loadOp: "load", storeOp: "discard" },
        ],
        depthStencilAttachment: {
          view: t.depth.createView(),
          depthClearValue: BATTLE_DEPTH_ATTACHMENT.clearValue,
          depthLoadOp: "clear",
          depthStoreOp: "store",
        },
      });
      const opaque = hdrOpaque.with(pass).with(cameraGroup).with(environment.group).with(fogGroup);
      drawMesh(opaque, worldBuffers.opaque);
      drawInstances(opaque);
      drawMesh(
        hdrOpaque.with(pass).with(cameraGroup).with(environment.group).with(fogOffGroup),
        overlayBuffers.structures,
      );
      drawMesh(
        hdrTranslucent.with(pass).with(cameraGroup).with(environment.group).with(fogGroup),
        worldBuffers.translucent,
      );
      pass.end();
      mark(raw, 2);
      if (debug.overlayOnly) {
        const v = debug.overlayOnly === "white" ? 1 : 0;
        raw
          .beginRenderPass({
            colorAttachments: [{ view: target, loadOp: "clear", storeOp: "store", clearValue: [v, v, v, 1] }],
          })
          .end();
      } else postChain!.encode(raw, target, true, true);
      mark(raw, 3);
      const hasOverlay = (overlayBuffers.opaque?.count ?? 0) + (overlayBuffers.translucent?.count ?? 0) > 0;
      if (hasOverlay) {
        const opass = encoder.beginRenderPass({
          colorAttachments: [
            {
              view: t.overlayMsaa.createView(),
              resolveTarget: t.overlay.createView(),
              loadOp: "clear",
              storeOp: "discard",
              clearValue: [0, 0, 0, 0],
            },
          ],
          depthStencilAttachment: {
            view: t.depth.createView(),
            depthLoadOp: "load",
            depthStoreOp: "discard",
          },
        });
        drawMesh(overlayOpaque.with(opass).with(cameraGroup), overlayBuffers.opaque);
        drawMesh(overlayTranslucent.with(opass).with(cameraGroup), overlayBuffers.translucent);
        opass.end();
        composite
          .with(encoder)
          .with(overlayGroup!)
          .withColorAttachment({ view: target, loadOp: "load", storeOp: "store" } as any)
          .draw(3);
      }
      mark(raw, 4);
      let readback: { buffer: GPUBuffer; busy: boolean } | undefined;
      if (timing) {
        readback = timing.readbacks.find((r) => !r.busy);
        raw.resolveQuerySet(timing.querySet, 0, PHASES.length + 1, timing.resolve, 0);
        if (readback) raw.copyBufferToBuffer(timing.resolve, 0, readback.buffer, 0, 8 * (PHASES.length + 1));
      }
      encoder.submit();
      cpuSum += performance.now() - cpu0;
      cpuN++;
      if (readback) {
        readback.busy = true;
        const rb = readback;
        rb.buffer
          .mapAsync(GPUMapMode.READ)
          .then(() => {
            const ts = new BigUint64Array(rb.buffer.getMappedRange().slice(0));
            rb.buffer.unmap();
            rb.busy = false;
            timingDebug.last = Array.from(ts, (v) => Number(v - ts[0]));
            if (ts[4] === 0n) return;
            PHASES.forEach((name, i) => {
              gpuSums[name] = (gpuSums[name] ?? 0) + Number(ts[i + 1] - ts[i]) / 1e6;
            });
            gpuSums.total = (gpuSums.total ?? 0) + Number(ts[4] - ts[0]) / 1e6;
            gpuSamples++;
          })
          .catch((e) => {
            timingDebug.error = String(e);
            rb.busy = false;
          });
      }
      frames++;
    },
    resize(w: number, h: number) {
      if (disposed) return;
      void ensureTargets(Math.max(64, Math.floor(w)), Math.max(64, Math.floor(h)));
    },
    setWorld(next: WorldMeshes) {
      if (disposed) return;
      for (const k of ["opaque", "translucent"] as const) {
        worldBuffers[k]?.buffer.destroy();
        worldBuffers[k] = vertexBuffer(next[k]);
      }
      const b: [number, number, number, number] = [0, 0, Infinity, -Infinity];
      const m = next.opaque;
      for (let i = 0; i < m.length; i += VERTEX_FLOATS) {
        b[0] = Math.max(b[0], m[i]);
        b[1] = Math.max(b[1], m[i + 1]);
        b[2] = Math.min(b[2], m[i + 2]);
        b[3] = Math.max(b[3], m[i + 2]);
      }
      bounds = b;
    },
    setOverlay(next: WorldMeshes) {
      if (disposed) return;
      const split = next.leadingStructureFloats ?? 0;
      const parts = {
        structures: next.opaque.subarray(0, split) as Mesh,
        opaque: next.opaque.subarray(split) as Mesh,
        translucent: next.translucent,
      };
      structureFloats = parts.structures.slice() as Mesh;
      for (const k of ["opaque", "translucent", "structures"] as const) {
        overlayBuffers[k]?.buffer.destroy();
        overlayBuffers[k] = vertexBuffer(parts[k].slice() as Mesh);
      }
    },
    setInstances(list: readonly SceneInstance[]) {
      if (disposed) return;
      instanceCount = list.length;
      for (const kind of KINDS) {
        const ofKind = list.filter((i) => i.kind === kind);
        let slot = instanceSlots.get(kind);
        if (ofKind.length === 0) {
          if (slot) slot.count = 0;
          continue;
        }
        if (!slot || slot.capacity < ofKind.length) {
          slot?.buffer.destroy();
          const capacity = Math.max(8, ofKind.length * 2);
          slot = { buffer: mkInstances(capacity), capacity, count: 0 };
          instanceSlots.set(kind, slot);
        }
        const data = new Float32Array(ofKind.length * INSTANCE_FLOATS);
        ofKind.forEach((inst, i) =>
          data.set([inst.x, inst.y, inst.z, inst.yaw, ...inst.color, inst.highlight ? 1 : 0], i * INSTANCE_FLOATS),
        );
        slot.buffer.write(data.buffer);
        slot.count = ofKind.length;
      }
    },
    setFog(fog: FogField | null) {
      if (disposed) return;
      if (!fog) {
        fogParams.write({ cellM: 1, nx: 0, ny: 0, enabled: 0 });
        return;
      }
      if (fog.bits.length > fogWords) {
        fogBits.destroy();
        fogBits = root.createBuffer(d.arrayOf(d.u32, fog.bits.length)).$usage("storage");
        fogWords = fog.bits.length;
        fogGroup = root.createBindGroup(fogLayout, { params: fogParams, bits: fogBits });
        fogOffGroup = root.createBindGroup(fogLayout, { params: fogOffParams, bits: fogBits });
      }
      fogBits.write(fog.bits.buffer as ArrayBuffer);
      fogParams.write({ cellM: fog.cellM, nx: fog.nx, ny: fog.ny, enabled: 1 });
    },
    stats() {
      return {
        width,
        height,
        frames,
        instances: instanceCount,
        worldVertices: (worldBuffers.opaque?.count ?? 0) + (worldBuffers.translucent?.count ?? 0),
        depth: { format: BATTLE_DEPTH_ATTACHMENT.format, clearValue: 0, compare: "greater-equal" as GPUCompareFunction },
      };
    },
    spikeStats(): SpikeStats {
      const gpuMs: Record<string, number> = {};
      for (const [k, v] of Object.entries(gpuSums)) gpuMs[k] = gpuSamples ? v / gpuSamples : 0;
      gpuMs.samples = gpuSamples;
      gpuMs.cpuRenderMs = cpuN ? cpuSum / cpuN : 0;
      (gpuMs as Record<string, unknown>).debug = timingDebug;
      return { gpuMs, cascades: lastCascades, splitNear: lastSplit.near, cappedFar: lastSplit.far };
    },
    debug,
    ready: () => ready,
    /** World AABBs of standing structures (36 vertices per box). */
    structureBoxes() {
      const boxes: number[][] = [];
      const per = 36 * VERTEX_FLOATS;
      for (let o = 0; o + per <= structureFloats.length; o += per) {
        const b = [Infinity, Infinity, Infinity, -Infinity, -Infinity, -Infinity];
        for (let i = o; i < o + per; i += VERTEX_FLOATS)
          for (let a = 0; a < 3; a++) {
            b[a] = Math.min(b[a], structureFloats[i + a]);
            b[a + 3] = Math.max(b[a + 3], structureFloats[i + a]);
          }
        boxes.push(b);
      }
      return boxes;
    },
    sunDirection: () => {
      const e = env.sunElevation,
        a = env.sunAzimuth;
      return [Math.cos(e) * Math.cos(a), Math.cos(e) * Math.sin(a), Math.sin(e)];
    },
    resetGpuTiming() {
      for (const k of Object.keys(gpuSums)) delete gpuSums[k];
      gpuSamples = 0;
      cpuSum = 0;
      cpuN = 0;
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      releaseTargets();
      for (const b of [...Object.values(worldBuffers), ...Object.values(overlayBuffers)]) b?.buffer.destroy();
      for (const s of instanceSlots.values()) s.buffer.destroy();
      fogBits.destroy();
      for (const r of owned.reverse()) r.destroy();
      root.destroy();
    },
  };
  scene.setWorld(world);
  scene.setInstances(instances);
  (window as unknown as { __spike: typeof scene }).__spike = scene;
  return scene;
}
