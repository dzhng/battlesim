// @vitest-environment node
import { expect, test } from "vitest";
import { tgpu } from "typegpu";
import { typegpuCameraLayout } from "@packages/battle-renderer/src/world/camera";
import { createWorldPass } from "@packages/battle-renderer/src/frame/worldPass";
import { createEnvironmentFrame } from "@packages/battle-renderer/src/frame/environmentFrame";
import { createModelLayer } from "@packages/battle-renderer/src/models/modelLayer";
import { GpuRegistry } from "@packages/battle-renderer/src/frame/registry";
import { allocateFrameTargets } from "@packages/battle-renderer/src/frame/targets";
import {
  CAMERA_WORDS,
  CameraAt,
  Camera,
  VIEW_WORDS,
  SUN_SHADOW_WORDS,
} from "@packages/battle-renderer/src/records.gen";
import { resolveLight } from "@web/battle/sim/presentation";
import { gameLight } from "@apps/battle-lab/src/gameLight";
import { gameFogGeometry } from "@apps/battle-lab/src/gameFog";
import { gamePaint, gameXrayMinHiddenFragmentFraction } from "@apps/battle-lab/src/gameOverlay";
import { gameGlass, gameModelDetail } from "@apps/battle-lab/src/gameModels";
import { benchWorld } from "@apps/battle-lab/src/workbench/benchWorld";
import { installGpuGlobals, recordingGpuDevice } from "./support/recordingDevice";

installGpuGlobals(1);

const triangle = (x: number, z: number) =>
  new Float32Array([
    x,
    0,
    z,
    0,
    0,
    1,
    1,
    0,
    0,
    1,
    x + 0.25,
    0,
    z,
    0,
    0,
    1,
    1,
    0,
    0,
    1,
    x,
    0.25,
    z,
    0,
    0,
    1,
    1,
    0,
    0,
    1,
  ]);
const camera = (x: number) => {
  const words = new Float32Array(CAMERA_WORDS);
  words.set([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, -x, 0, 0, 1], CameraAt.view_proj);
  return words;
};

// Record the final GPU boundary, independently of the renderer's counters.
function recordDraws(device: GPUDevice) {
  const calls: { pipeline: string; pass: string; vertices: number; bytes: number[] }[] = [];
  const buffers = new WeakMap<GPUBuffer, Uint8Array>();
  const pipelines = new WeakMap<GPURenderPipeline, string>();
  const createBuffer = device.createBuffer.bind(device);
  device.createBuffer = (desc) => {
    const buffer = createBuffer(desc);
    if (desc.usage & GPUBufferUsage.VERTEX) buffers.set(buffer, new Uint8Array(desc.size));
    return buffer;
  };
  const writeBuffer = device.queue.writeBuffer.bind(device.queue);
  device.queue.writeBuffer = (buffer, offset, data, dataOffset = 0, size) => {
    const target = buffers.get(buffer);
    if (target) {
      const view = ArrayBuffer.isView(data);
      const stride = view && "BYTES_PER_ELEMENT" in data ? Number(data.BYTES_PER_ELEMENT) : 1;
      const start = (view ? data.byteOffset : 0) + dataOffset * stride;
      const length = size === undefined ? data.byteLength - dataOffset * stride : size * stride;
      target.set(new Uint8Array(view ? data.buffer : data, start, length), offset);
    }
    writeBuffer(buffer, offset, data, dataOffset, size);
  };
  const createPipeline = device.createRenderPipelineAsync.bind(device);
  device.createRenderPipelineAsync = async (desc) => {
    const pipeline = await createPipeline(desc);
    pipelines.set(pipeline, desc.label ?? "");
    return pipeline;
  };
  const createEncoder = device.createCommandEncoder.bind(device);
  device.createCommandEncoder = (desc) => {
    const encoder = createEncoder(desc);
    return new Proxy(encoder, {
      get(target, key) {
        if (key !== "beginRenderPass") return Reflect.get(target, key);
        return (desc: GPURenderPassDescriptor) => {
          const pass = target.beginRenderPass(desc);
          let pipeline = "";
          let mesh: GPUBuffer | null = null;
          const overrides = {
            setPipeline(value: GPURenderPipeline) {
              pipeline = pipelines.get(value) ?? "";
            },
            setVertexBuffer(slot: number, value: GPUBuffer | null) {
              if (slot === 0) mesh = value;
            },
            draw(vertices: number, instances = 1, first = 0) {
              if (pipeline.startsWith("world"))
                calls.push({
                  pipeline,
                  pass: desc.label ?? "",
                  vertices: vertices * instances,
                  bytes: Array.from(buffers.get(mesh!)!.slice(first * 40, (first + vertices) * 40)),
                });
            },
          };
          return new Proxy(pass, {
            get(target, key) {
              return key in overrides
                ? overrides[key as keyof typeof overrides]
                : Reflect.get(target, key);
            },
          });
        };
      },
    });
  };
  return calls;
}

test("world passes bound water and skirt/debug draws to camera and independent cascades", async () => {
  const { device, recording } = recordingGpuDevice();
  const calls = recordDraws(device);
  const registry = new GpuRegistry(device);
  const root = tgpu.initFromDevice({ device });
  const environment = await createEnvironmentFrame(device, registry, await resolveLight(gameLight));
  const models = await createModelLayer(root, registry, gameModelDetail);
  const world = await createWorldPass(
    root,
    registry,
    environment,
    gameFogGeometry,
    models,
    gamePaint,
    gameXrayMinHiddenFragmentFraction,
    gameGlass,
    1,
    "full",
  );
  const targets = allocateFrameTargets(registry, 64, 64, "full");
  const cameraGroup = root.createBindGroup(typegpuCameraLayout, {
    cam: registry.own(root.createBuffer(Camera).$usage("uniform")),
  });
  const layers = benchWorld(null);
  layers.terrain.mesh = triangle(0, 0.5);
  layers.terrain.grid = null;
  layers.terrain.water = true;
  layers.props = new Float32Array([...triangle(0, 0.4), ...triangle(1024, 0.4)]);
  let waterMesh = new Float32Array([...triangle(0, 0.3), ...triangle(1024, 0.3)]);
  world.setWorld(layers);
  // The feed is the external presentation seam; material tables are absent in
  // this draw-routing fixture, while its water has camera and off-camera cells.
  world.setPresentedGround({
    ground_terrain: () => undefined,
    ground_plots: () => new Uint32Array(24),
    ground_water_mesh: () => waterMesh,
    resend_fog() {},
    fog_stats: () => undefined,
  } as never);
  const casters = new Float32Array(CAMERA_WORDS * 4);
  [1024, 0, 5000, 1024].forEach((x, i) => casters.set(camera(x), i * CAMERA_WORDS));
  for (const remoteCount of [0, 64, 1024]) {
    calls.length = 0;
    const props = [...triangle(0, 0.4), ...triangle(1024, 0.4)];
    const water = [...triangle(0, 0.3), ...triangle(1024, 0.3)];
    for (let i = 0; i < remoteCount; i++) {
      props.push(...triangle(10_000 + i * 512, 0.4));
      water.push(...triangle(10_000 + i * 512, 0.3));
    }
    layers.props = new Float32Array(props);
    waterMesh = new Float32Array(water);
    world.setWorld(layers);
    world.prepare(
      camera(0),
      new Float32Array(VIEW_WORDS),
      new Float32Array(SUN_SHADOW_WORDS),
      casters,
      4,
    );
    const encoder = root["~unstable"].createCommandEncoder();
    world.encodeShadows(encoder);
    world.encodeDepth(encoder, root.unwrap(encoder), targets, cameraGroup);
    world.encode(encoder, root.unwrap(encoder), targets, cameraGroup);
    const frameStats = world.stats();
    const height = Math.ceil(Math.log2(remoteCount + 2));
    expect(frameStats.waterVisitedNodes).toBeLessThanOrEqual(4 * height + 3);
    expect(frameStats.skirtVisitedNodes).toBeLessThanOrEqual(5 * (4 * height + 3));
    const stats = frameStats.submitted;
    expect(stats.color.water.triangles).toBe(1);
    expect(stats.depth.skirtAndDebugProps.triangles).toBe(1);
    expect(stats.color.skirtAndDebugProps).toEqual(stats.depth.skirtAndDebugProps);
    expect(stats.cascades.map((c) => c.skirtAndDebugProps.triangles)).toEqual([1, 1, 0, 1]);
    const waterCalls = calls.filter((c) => c.pipeline === "worldWater");
    expect(waterCalls.map((c) => c.vertices)).toEqual([3]);
    expect(waterCalls[0].bytes).toEqual(Array.from(new Uint8Array(triangle(0, 0.3).buffer)));
    const skirtCalls = calls.filter((c) => c.pipeline === "worldOpaque");
    expect(skirtCalls.map((c) => c.vertices)).toEqual([3]);
    expect(skirtCalls[0].bytes).toEqual(Array.from(new Uint8Array(triangle(0, 0.4).buffer)));
    expect(calls.filter((c) => c.pipeline === "worldCaster").map((c) => c.vertices)).toEqual([
      3, 3, 3, 3,
    ]);
    expect(stats.depth.water.triangles).toBe(0);
    expect(stats.cascades.map((c) => c.water.triangles)).toEqual([0, 0, 0, 0]);
    expect(stats.color.backdrop.triangles).toBe(8);
    expect(stats.depth.backdrop.triangles).toBe(8);
    const expectedCasters = [
      triangle(1024, 0.4),
      triangle(0, 0.5),
      triangle(0, 0.4),
      triangle(1024, 0.4),
    ];
    expect(calls.filter((c) => c.pipeline === "worldCaster").map((c) => c.bytes)).toEqual(
      expectedCasters.map((t) => Array.from(new Uint8Array(t.buffer))),
    );
  }
  const water = recording.pipelines.find((p) => p.label === "worldWater")!;
  expect(water.depthStencil?.depthWriteEnabled).toBe(false);
  expect(water.fragment?.targets[0]?.blend?.color).toEqual({
    srcFactor: "src-alpha",
    dstFactor: "one-minus-src-alpha",
  });
  registry.release();
  root.destroy();
});
