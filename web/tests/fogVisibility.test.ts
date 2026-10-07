// @vitest-environment node
import { expect, test } from "vitest";
import { wholeWords, type FogOccluder } from "@packages/battle-renderer/src/frame/fogInputs";
import { fogOccludersInReach } from "@packages/battle-renderer/src/frame/fogVisibility";

const box = (x: number, y: number, hx = 2, hy = 3, yaw = 0): FogOccluder => ({
  x,
  y,
  hx,
  hy,
  yaw,
  base: 0,
  top: 8,
});

test("eye candidates retain crossed and touching boxes once, in original order, across separated regions", () => {
  const boxes = [
    box(1000, 1000),
    box(105, 0),
    box(0, 0),
    box(150, 0, 60, 2, 0.6),
    box(-104, 0, 4, 3),
    box(0, 1000),
  ];
  const grid = wholeWords(boxes, 0.2);
  expect(fogOccludersInReach(grid, [0, 0, 2], 100)).toEqual([2, 3, 4]);
  expect(fogOccludersInReach(grid, [1000, 1000, 2], 100)).toEqual([0]);
  expect(fogOccludersInReach(grid, [-5000, -5000, 2], 100)).toEqual([]);
  expect(fogOccludersInReach(wholeWords([], 0.2), [0, 0, 2], 100)).toEqual([]);
});

test("changing, removing or adding occluders invalidates only eyes in their old or new reach", async () => {
  const { fogAffectedEyes } = await import("@packages/battle-renderer/src/frame/fogVisibility");
  const near = box(40, 0);
  const far = box(2040, 0);
  const eyes = [
    { position: [0, 0, 2] as const, reach: 100 },
    { position: [2000, 0, 2] as const, reach: 100 },
  ];
  const before = [near, far];
  const check = (after: FogOccluder[], expected: number[]) =>
    expect(
      fogAffectedEyes(before, wholeWords(before, 0.2), after, wholeWords(after, 0.2), eyes),
    ).toEqual(expected);
  check([{ ...far }, { ...near }], []); // new records and reordered rows do not change geometry
  check([near, { ...far, top: 1 }], [1]);
  check([far], [0]);
  check([near, far, box(0, 20)], [0]);
  check([{ ...near, x: 2000 }, far], [0, 1]);
  check([near, far, box(5000, 5000)], []);
});

test("an occluder update immediately rebuilds only affected built maps through the frame owner", async () => {
  const { createFogVisibility } = await import("@packages/battle-renderer/src/frame/fogVisibility");
  const { GpuRegistry } = await import("@packages/battle-renderer/src/frame/registry");
  const { vi } = await import("vitest");
  vi.stubGlobal("GPUTextureUsage", { TEXTURE_BINDING: 4, COPY_DST: 2 });
  // Only the external GPU boundary is fake; scheduling, packing and lifetime
  // ownership run through the same frame owner as a rendered battle.
  const resource = () => ({
    destroyed: false,
    destroy() {
      this.destroyed = true;
    },
    createView() {
      return {};
    },
  });
  const pipeline = {
    async initAsync() {},
    with(group: { bindings?: Record<string, { destroyed?: boolean }> }) {
      if (group.bindings && Object.values(group.bindings).some((b) => b.destroyed))
        throw new Error("bound buffer destroyed");
      return this;
    },
    dispatchWorkgroups() {},
  };
  const root = {
    createComputePipeline() {
      return pipeline;
    },
    createBindGroup(_layout: unknown, bindings: Record<string, { destroyed?: boolean }>) {
      return { bindings };
    },
    createBuffer() {
      return {
        ...resource(),
        $usage() {
          return this;
        },
        write() {},
      };
    },
  } as unknown as Parameters<typeof createFogVisibility>[0];
  const device = {
    createBuffer(d: GPUBufferDescriptor) {
      return {
        ...resource(),
        size: d.size,
        async mapAsync() {},
        getMappedRange() {
          return new ArrayBuffer(d.size);
        },
      };
    },
    createTexture() {
      return resource();
    },
    queue: { writeBuffer() {}, writeTexture() {}, submit() {} },
    createCommandEncoder() {
      return {
        copyBufferToBuffer() {},
        finish() {
          return {};
        },
      };
    },
  } as unknown as GPUDevice;
  const registry = new GpuRegistry(device);
  try {
    const fog = await createFogVisibility(
      root,
      registry,
      {
        azimuth_bins: 8,
        terrain_azimuth_bins: 8,
        radial_bins: 2,
        first_bin_m: 1,
        terrain_step_m: [1, 5],
        terrain_step_fraction: 0.1,
        tile_px: 1,
        tile_eyes_max: 8,
        face_probe_m: 0.1,
        roof_reach_m: 0,
        rebuild_eyes_per_frame: 1,
        whole_step_m: 1,
      },
      {
        forGrid() {
          return {};
        },
      } as unknown as Parameters<typeof createFogVisibility>[3],
    );
    const world = {
      nx: 2,
      ny: 2,
      spacing: 5000,
      pageSize: 2,
      minHeight: 0,
      pageIds: new Uint32Array(0),
      heights: new Float32Array(0),
      foliage: new Float32Array(0),
      targetHeightM: 0,
      foliageFullBlock: 1,
      minSightGapM: 0,
    };
    const eyes = [0, 2000].map((x, i) => ({
      key: `${i}`,
      position: [x, 0, 2] as const,
      forward: 0,
      shape: { front: 1, side: 1, rear: 1 },
      range: 100,
    }));
    const texture = resource() as unknown as GPUTexture;
    const tiles = fog.sized(registry, 1, 1, texture, texture);
    const encoder = { clearBuffer() {}, copyBufferToTexture() {} } as unknown as GPUCommandEncoder;
    const frame = () => fog.encode(encoder, tiles, new Float32Array(48), 1, 1);
    const boxes = [box(40, 0), box(2040, 0)];
    fog.set({ world, sight: { eyes, occluders: boxes } });
    await fog.probe([{ position: [0, 0, 0] }]);
    expect(fog.stats().rebuilt).toBe(2);
    frame();
    expect(fog.stats().rebuilt).toBe(0);
    fog.set({ world, sight: { eyes, occluders: [boxes[0], { ...boxes[1], top: 1 }] } });
    frame();
    expect(fog.stats().rebuilt).toBe(1);
    expect(fog.stats().pending).toBe(0);
    fog.set({ world, sight: { eyes, occluders: [boxes[1], boxes[0]] } });
    frame();
    expect(fog.stats().rebuilt).toBe(1); // restores the second box's height
    fog.set({ world, sight: { eyes, occluders: [...boxes] } });
    frame();
    expect(fog.stats().rebuilt).toBe(0); // same geometry, different row order
    const moved = eyes.map((e) => ({ ...e, position: [e.position[0] + 1, 0, 2] as const }));
    fog.set({ world, sight: { eyes: moved, occluders: boxes } });
    fog.set({ world, sight: { eyes: moved, occluders: [{ ...boxes[0], top: 1 }, boxes[1]] } });
    frame();
    expect(fog.stats().rebuilt).toBe(2); // affected eye plus one moved eye within the budget
    expect(fog.stats().pending).toBe(0);
    frame();
    expect(fog.stats().rebuilt).toBe(0); // no stale movement entry repeats the immediate rebuild
  } finally {
    registry.release();
    vi.unstubAllGlobals();
  }
});

test("eye candidates come once each, in index order, and asking again gives the same answer", () => {
  // Boxes of every size, some spanning many cells, past one 32-box word.
  const boxes = Array.from({ length: 300 }, (_, i) =>
    box((i * 97) % 900, (i * 61) % 700, 1 + (i % 7) * 6, 1 + (i % 5) * 9, i * 0.3),
  );
  const grid = wholeWords(boxes, 0.2);
  for (const [x, y, reach] of [
    [100, 100, 80],
    [450, 350, 250],
    [880, 20, 40],
  ]) {
    const got = fogOccludersInReach(grid, [x, y, 2], reach);
    expect(got.length).toBeGreaterThan(0);
    expect(got).toEqual([...new Set(got)].sort((a, b) => a - b));
    // Nothing is left marked between queries.
    expect(fogOccludersInReach(grid, [x, y, 2], reach)).toEqual(got);
  }
});

test("whole-surface candidates retain boxes whose roof probes can reach inward from outside the eye circle", () => {
  const boxes = [box(0, 0), box(132, 0, 1, 100), box(3000, 3000)];
  const grid = wholeWords(boxes, 0.2);
  expect(fogOccludersInReach(grid, [0, 0, 2], 100)).toEqual([0]);
  // The whole-structure pass first accepts this tall narrow box by its
  // bounding circle; its roof rule can then probe inward toward the eye.
  expect(fogOccludersInReach(grid, [0, 0, 2], 100, 101)).toEqual([0, 1]);
});

test("whole-fog records pair each reachable structure only with eyes that can reach it", async () => {
  const { wholeFogRecords } = await import("@packages/battle-renderer/src/frame/fogVisibility");
  const boxes = [box(40, 0), box(2040, 0), box(5000, 5000), box(132, 0, 1, 100)];
  const eyes = [
    { position: [0, 0, 2] as const, reach: 100 },
    { position: [2000, 0, 2] as const, reach: 100 },
  ];
  const records = wholeFogRecords(wholeWords(boxes, 0.2), eyes, 101);
  expect(records.count).toBe(3);
  // GPU records: [box, eye-list offset, eye count], then the eye indices.
  expect([...records.words]).toEqual([0, 9, 1, 1, 10, 1, 3, 11, 1, 0, 1, 0]);
  const none = wholeFogRecords(wholeWords(boxes, 0.2), [], 101);
  expect(none.count).toBe(0);
  expect(none.words).toEqual(new Uint32Array(0));
});

test("whole-fog records reusing the eyes' last lists match records made afresh, as eyes move and the grid changes", async () => {
  const { wholeFogRecords } = await import("@packages/battle-renderer/src/frame/fogVisibility");
  const boxes = Array.from({ length: 60 }, (_, i) => box((i * 37) % 400, (i * 53) % 400));
  let grid = wholeWords(boxes, 0.2);
  let kept: import("@packages/battle-renderer/src/frame/fogVisibility").EyeOccluders | undefined;
  for (let frame = 0; frame < 6; frame++) {
    // Half the eyes stand still; the rest walk, and once the grid changes.
    const eyes = Array.from({ length: 8 }, (_, e) => ({
      position: [e * 50 + (e % 2 ? frame * 7 : 0), 200, 2] as const,
      reach: 90 + e,
    }));
    if (frame === 3) grid = wholeWords(boxes.slice(10), 0.2);
    const fresh = wholeFogRecords(grid, eyes, 101);
    const reused = wholeFogRecords(grid, eyes, 101, kept);
    expect(reused.count).toBe(fresh.count);
    expect([...reused.words]).toEqual([...fresh.words]);
    kept = reused.kept;
  }
});

test("horizon sectors conservatively retain slab intersections across west wrap and turned boxes", async () => {
  const { fogHorizonRecords } = await import("@packages/battle-renderer/src/frame/fogVisibility");
  const boxes = [box(-40, 0), box(40, 0), box(0, 0), box(15, 30, 12, 2, 0.8)];
  const records = fogHorizonRecords(wholeWords(boxes, 0.2), [{ position: [0, 0, 2], reach: 100 }]);
  const sectors = records[2];
  const list = (sector: number) => {
    const row = records[1] + sector * 2;
    return [...records.subarray(records[row], records[row] + records[row + 1])];
  };
  expect(list(0)).toEqual([0, 2]);
  expect(list(sectors - 1)).toEqual([0, 2]);
  expect(list(sectors / 2)).toEqual([1, 2]);
  for (let ai = 0; ai < 8192; ai++) {
    const theta = ((ai + 0.5) / 8192) * Math.PI * 2 - Math.PI;
    const candidates = list(Math.floor(((ai + 0.5) / 8192) * sectors));
    boxes.forEach((b, i) => {
      const c = Math.cos(b.yaw),
        s = Math.sin(b.yaw);
      const ox = -b.x * c - b.y * s,
        oy = b.x * s - b.y * c;
      const dx = Math.cos(theta) * c + Math.sin(theta) * s;
      const dy = -Math.cos(theta) * s + Math.sin(theta) * c;
      const tx = [(-b.hx - ox) / dx, (b.hx - ox) / dx].sort((a, z) => a - z);
      const ty = [(-b.hy - oy) / dy, (b.hy - oy) / dy].sort((a, z) => a - z);
      const near = Math.max(tx[0], ty[0]),
        far = Math.min(tx[1], ty[1]);
      if (near <= far && far > 0 && near <= 100) expect(candidates).toContain(i);
    });
  }
});
