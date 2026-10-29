// FogVisibility: the GPU half of sight-light fog (spike 02's technique). The
// presentation crate decides everything (`presentation::fog::sight`): each
// frame it hands a `FogUpload`, which this uploads as it is, then runs:
//
// - the rebuilt eyes' horizon maps, `azimuth_bins × radial_bins` words an
//   eye: a coarse terrain march at `terrain_azimuth_bins` rays, then a merge
//   at full azimuth that meets only the known occluders within reach;
// - the structures that take fog whole: one workgroup per reachable
//   occluder samples its walls and roof against every eye in reach and flags
//   it seen if any sample is; the flags join the occluders' boxes and a
//   lookup grid in the `wholes` texture `fogWholeSeen` reads;
// - per frame, a cull over the depth prepass: each screen tile bounds the
//   ground it shows and lists the eyes whose reach touches it.
//
// Every buffer is owned by the frame's registry; the tile lists live in the
// size-dependent scope beside the targets. `probe` evaluates the same lookup
// at given points against every eye (the agreement metric and the lab's
// tests), and `probeShape` runs the WGSL sight shape against Rust's vectors.
// Probes read back: lab only, never in the frame.
import { tgpu, d } from "typegpu";
import { GPU_DEPTH_FORMAT } from "@packages/renderer-core/src/depthContract";
import { FRAME_MSAA, HDR_FORMAT } from "./targets";
import type { FogUpload, Presentation } from "@wasm/presentation/presentation_wasm.js";
import type { GpuRegistry, GpuSlot } from "./registry";
import type { TerrainHeights } from "./terrainHeights";
import type { TerrainGrid } from "../terrain/terrainGrid";
import {
  FogParams,
  FOG_BOX_WORDS,
  FOG_EYE_WORDS,
  FOG_PARAMS_WORDS,
  WHOLE_TEXTURE_WIDTH,
} from "../records.gen";
import { FogLayer, GroundPaintStyle, fogLayout } from "./fogTerm";

import {
  BUILD_WORKGROUP,
  buildLayout,
  cullFn,
  cullLayout,
  mergeFn,
  probeFn,
  probeLayout,
  shapeFn,
  shapeLayout,
  terrainMarch,
  wholeFn,
  wholeLayout,
} from "./fogCompute";

type Root = ReturnType<typeof tgpu.initFromDevice>;

const STORAGE = 0x80; // GPUBufferUsage.STORAGE
const COPY_DST = 0x08;
const COPY_SRC = 0x04;
const MAP_READ = 0x01;
const WORD = 4;
const PROBE_BYTES = 32;

/** A point to probe: ground (at the target height above it) or a face. */
export interface FogProbeInput {
  position: readonly [number, number, number];
  /** Zero for ground. */
  normal?: readonly [number, number, number];
}

/** What fog is drawn from: the scene's presentation, which decides it. */
export type FogSource = Pick<
  Presentation,
  "prepare_fog" | "resend_fog" | "rebuild_fog" | "fog_stats" | "fog_occluders"
>;

/** The pipelines' text constants (`presentation.fog_geometry`): the cull's
 *  screen tile and the merge's radial bins. */
export interface FogShaders {
  tile_px: number;
  radial_bins: number;
}

/** A structure that takes fog whole, as the lab reads it back. */
export interface FogOccluder {
  x: number;
  y: number;
  yaw: number;
  hx: number;
  hy: number;
  base: number;
  top: number;
}

export interface FogVisibilityStats {
  /** Whether fog is drawn. */
  enabled: boolean;
  eyes: number;
  /** Map slots allocated (eyes that fit before the maps grow). */
  slots: number;
  occluders: number;
  /** Eye maps rebuilt last frame, and moved eyes still waiting their turn. */
  rebuilt: number;
  pending: number;
  /** Every rebuild since the maps were allocated. */
  rebuiltTotal: number;
  mapBytes: number;
  terrainMapBytes: number;
  tileListBytes: number;
  tiles: [number, number];
}

/** The tile lists for one frame size, made at its first fogged frame. */
export interface FogTiles {
  scope: GpuRegistry;
  tilesX: number;
  tilesY: number;
  lists: GPUBuffer | null;
  counts: GPUBuffer | null;
  depthView: GPUTextureView;
  /** The frame's ground paint, which the painted layers read. */
  paintView: GPUTextureView;
  /** The world before the water, which the water reads over its depth,
   *  and its guard, where the water refracts. */
  sceneColourView: GPUTextureView;
  seenUnderView: GPUTextureView | null;
  /** The wave field's latest heights, which the water reads. */
  wavesView: GPUTextureView;
}

/** `Presentation.fog_occluders`' seven numbers a box, as boxes. */
export function occluderBoxes(n: Float64Array): FogOccluder[] {
  const boxes: FogOccluder[] = [];
  for (let i = 0; i < n.length; i += 7) {
    const [x, y, yaw, hx, hy, base, top] = n.subarray(i, i + 7);
    boxes.push({ x, y, yaw, hx, hy, base, top });
  }
  return boxes;
}

function probeRecords(points: readonly FogProbeInput[]): ArrayBuffer {
  const bytes = new ArrayBuffer(Math.max(1, points.length) * PROBE_BYTES);
  const f = new Float32Array(bytes);
  const u = new Uint32Array(bytes);
  points.forEach((p, i) => {
    const o = (i * PROBE_BYTES) / WORD;
    f.set(p.position, o);
    u[o + 3] = p.normal ? 0 : 1;
    f.set(p.normal ?? [0, 0, 0], o + 4);
  });
  return bytes;
}

/** Ground with no height pages: flat at zero (a scene's bare ground). */
const FLAT: TerrainGrid = {
  nx: 2,
  ny: 2,
  spacing: 1,
  pageSize: 1,
  minHeight: 0,
  pageIds: new Uint32Array(0),
  heights: new Float32Array(0),
};

export async function createFogVisibility(
  root: Root,
  registry: GpuRegistry,
  shaders: FogShaders,
  terrainHeights: TerrainHeights,
) {
  const device = registry.device;
  const pipelines = {
    terrain: root.createComputePipeline({ compute: terrainMarch }).$name("fogTerrain"),
    merge: root.createComputePipeline({ compute: mergeFn(shaders.radial_bins) }).$name("fogMerge"),
    cull: root.createComputePipeline({ compute: cullFn(shaders.tile_px) }).$name("fogCull"),
    probe: root.createComputePipeline({ compute: probeFn }).$name("fogProbe"),
    shape: root.createComputePipeline({ compute: shapeFn }).$name("fogShape"),
    whole: root.createComputePipeline({ compute: wholeFn }).$name("fogWhole"),
  };
  await Promise.all(Object.values(pipelines).map((p) => p.initAsync()));

  const params = registry.own(root.createBuffer(FogParams).$usage("uniform"));
  const layerOf = (ground: number, seen = 0, painted = 0) => {
    const layer = registry.own(root.createBuffer(FogLayer).$usage("uniform"));
    layer.write({ ground, seen, painted, pad: 0 });
    return layer;
  };
  // Ground versus bodies: the one place that says which
  // layers the ground paint lies on. `paintedGround` is the terrain, its
  // grass and the backdrop; `paintedFaces` the surfaces movers stand on that
  // fog takes face by face: the water, and a prop whose body stops no mover
  // (a bridge deck, rubble); `ground` is what lies on the ground but is a
  // body (the fallen); faces and units are bodies.
  const layers = {
    paintedGround: layerOf(1, 0, 1),
    paintedFaces: layerOf(0, 0, 1),
    ground: layerOf(1),
    faces: layerOf(0),
    units: layerOf(0, 1),
  };
  const paintStyle = registry.own(root.createBuffer(GroundPaintStyle).$usage("uniform"));
  const noPaint = registry.texture({
    label: "fog-no-paint",
    size: [1, 1],
    format: "rgba8unorm",
    usage: GPUTextureUsage.TEXTURE_BINDING,
  });
  // What the groups the world pass binds hold for the water's views.
  const noDepth = registry.texture({
    label: "fog-no-depth",
    size: [1, 1],
    format: GPU_DEPTH_FORMAT,
    sampleCount: FRAME_MSAA,
    usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.RENDER_ATTACHMENT,
  });
  const noScene = registry.texture({
    label: "fog-no-scene",
    size: [1, 1],
    format: HDR_FORMAT,
    usage: GPUTextureUsage.TEXTURE_BINDING,
  });
  const sceneTaps = device.createSampler({
    label: "fog-scene-taps",
    magFilter: "linear",
    minFilter: "linear",
  });
  const storage = (label: string, bytes: number) =>
    device.createBuffer({ label, size: Math.max(16, bytes), usage: STORAGE | COPY_DST });
  const wholeTexture = (rows: number) =>
    device.createTexture({
      label: "fog-wholes",
      size: [WHOLE_TEXTURE_WIDTH, rows],
      format: "r32uint",
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
    });
  /** A flag word per structure, `rows` texture rows of them. */
  const wholeFlagBuffer = (rows: number) =>
    device.createBuffer({
      label: "fog-whole-flags",
      size: rows * WHOLE_TEXTURE_WIDTH * WORD,
      usage: STORAGE | COPY_SRC | COPY_DST,
    });
  const wholeUniform = registry.own(root.createBuffer(d.vec4f).$usage("uniform"));

  const slot = <T extends { destroy(): void }>(initial: T): GpuSlot<T> => {
    const s = registry.slot<T>();
    s.set(initial);
    return s;
  };
  const buffers = {
    // A storage binding holds at least one of its records.
    eyes: slot(storage("fog-eyes", FOG_EYE_WORDS * WORD)),
    maps: slot(storage("fog-maps", WORD)),
    terrain: slot(storage("fog-terrain-maps", WORD)),
    foliage: slot(storage("fog-foliage", 16)),
    occluders: slot(storage("fog-occluders", FOG_BOX_WORDS * WORD)),
    rebuild: slot(storage("fog-rebuild", WORD)),
    wholes: slot(wholeTexture(1)),
    wholeFlags: slot(wholeFlagBuffer(1)),
    wholeItems: slot(storage("fog-whole-items", WORD)),
  };
  /** Each buffer's size; a write that outgrows one replaces it. */
  const sizes = new Map<GpuSlot<GPUBuffer>, number>();
  /** Bind groups are made again when a buffer they bind is replaced. */
  let generation = 0;
  /** Writes `words` into `target`, growing it (by doubling) to fit; an
   *  `exact` buffer is the words' size, for a shader that reads its length. */
  const write = (
    target: GpuSlot<GPUBuffer>,
    label: string,
    words: Uint32Array | Float32Array,
    exact = false,
  ) => {
    const size = sizes.get(target) ?? 0;
    if (exact ? words.byteLength !== size : words.byteLength > size) {
      const bytes = exact ? words.byteLength : Math.max(words.byteLength, 2 * size);
      target.set(storage(label, bytes));
      sizes.set(target, bytes);
      generation++;
    }
    device.queue.writeBuffer(target.current!, 0, words);
  };

  let source: FogSource | null = null;
  let grid: TerrainGrid = FLAT;
  let active = false;
  let mapWords = [0, 0];
  let flagRows = 1;
  let tiles: FogTiles | null = null;
  let groupsFor: { generation: number; tiles: FogTiles | null; value: FogGroups } | null = null;

  /** Upload what the presentation decided; returns the rebuild's
   *  workgroups, if any eye rebuilds. */
  const upload = (up: FogUpload): Uint32Array | null => {
    active = up.active();
    const [maps, terrainMaps] = up.map_words();
    if (maps !== mapWords[0] || terrainMaps !== mapWords[1]) {
      // A new size rebuilds every map: the old ones are not kept.
      buffers.maps.set(storage("fog-maps", maps * WORD));
      buffers.terrain.set(storage("fog-terrain-maps", terrainMaps * WORD));
      mapWords = [maps, terrainMaps];
      generation++;
    }
    const foliage = up.foliage();
    // The foliage lookup searches the whole buffer (`arrayLength`).
    if (foliage) write(buffers.foliage, "fog-foliage", foliage, true);
    const occluders = up.occluders();
    if (occluders) write(buffers.occluders, "fog-occluders", occluders);
    const wholes = up.wholes();
    if (wholes) {
      const rows = wholes.length / WHOLE_TEXTURE_WIDTH;
      buffers.wholes.set(wholeTexture(rows));
      device.queue.writeTexture(
        { texture: buffers.wholes.current! },
        wholes,
        { bytesPerRow: WHOLE_TEXTURE_WIDTH * WORD },
        [WHOLE_TEXTURE_WIDTH, rows],
      );
      generation++;
    }
    const eyes = up.eyes();
    if (eyes.length) write(buffers.eyes, "fog-eyes", eyes);
    const rebuild = up.rebuild();
    if (rebuild.length) write(buffers.rebuild, "fog-rebuild", rebuild);
    device.queue.writeBuffer(root.unwrap(params), 0, up.params());
    return rebuild.length ? up.rebuild_dispatch() : null;
  };

  const buildGroup = () =>
    root.createBindGroup(buildLayout, {
      params,
      eyes: buffers.eyes.current!,
      heights: terrainHeights.forGrid(grid),
      foliage: buffers.foliage.current!,
      occluders: buffers.occluders.current!,
      rebuild: buffers.rebuild.current!,
      terrain: buffers.terrain.current!,
      maps: buffers.maps.current!,
    });
  let build: { generation: number; group: ReturnType<typeof buildGroup> } | null = null;
  /** The rebuilt eyes' terrain march and merge. */
  const rebuildMaps = (encoder: GPUCommandEncoder, dispatch: Uint32Array) => {
    if (!build || build.generation !== generation) build = { generation, group: buildGroup() };
    pipelines.terrain.with(build.group).with(encoder).dispatchWorkgroups(dispatch[0]);
    pipelines.merge.with(build.group).with(encoder).dispatchWorkgroups(dispatch[1]);
  };

  interface FogGroups {
    paintedGround: ReturnType<typeof fragmentGroup>;
    paintedFaces: ReturnType<typeof fragmentGroup>;
    ground: ReturnType<typeof fragmentGroup>;
    faces: ReturnType<typeof fragmentGroup>;
    units: ReturnType<typeof fragmentGroup>;
    /** The water's: the painted faces', with what lies under the water. */
    water: ReturnType<typeof fragmentGroup>;
    cull: ReturnType<typeof cullGroup> | null;
  }
  const fragmentGroup = (layer: typeof layers.ground, t: FogTiles | null, under = false) =>
    root.createBindGroup(fogLayout, {
      params,
      layer,
      eyes: buffers.eyes.current!,
      maps: buffers.maps.current!,
      lists: t?.lists ?? buffers.rebuild.current!,
      counts: t?.counts ?? buffers.rebuild.current!,
      paintStyle,
      paint: t?.paintView ?? noPaint.createView(),
      wholes: buffers.wholes.current!.createView(),
      depth: (under && t?.depthView) || noDepth.createView(),
      sceneColour: (under && t?.sceneColourView) || noScene.createView(),
      sceneTaps,
      seenUnder: (under && t?.seenUnderView) || noScene.createView(),
      waves: (under && t?.wavesView) || noScene.createView(),
    });
  const cullGroup = (t: FogTiles) =>
    root.createBindGroup(cullLayout, {
      params,
      eyes: buffers.eyes.current!,
      depth: t.depthView,
      lists: t.lists!,
      counts: t.counts!,
    });
  const groups = (): FogGroups => {
    if (!groupsFor || groupsFor.generation !== generation || groupsFor.tiles !== tiles) {
      groupsFor = {
        generation,
        tiles,
        value: {
          paintedGround: fragmentGroup(layers.paintedGround, tiles),
          paintedFaces: fragmentGroup(layers.paintedFaces, tiles),
          ground: fragmentGroup(layers.ground, tiles),
          faces: fragmentGroup(layers.faces, tiles),
          units: fragmentGroup(layers.units, tiles),
          water: fragmentGroup(layers.paintedFaces, tiles, true),
          cull: tiles?.lists ? cullGroup(tiles) : null,
        },
      };
    }
    return groupsFor.value;
  };

  /** The structures pass: clear the flags, flag each row's structure, copy
   *  the flags into the `wholes` texture's flag rows. */
  const encodeWholes = (encoder: GPUCommandEncoder, up: FogUpload, faces: FogGroups["faces"]) => {
    const [count, x, y, flagsRow, rows] = up.whole();
    if (rows !== flagRows) {
      buffers.wholeFlags.set(wholeFlagBuffer(rows));
      flagRows = rows;
    }
    // An eye leaving a neighbourhood must clear its previously seen flags.
    encoder.clearBuffer(buffers.wholeFlags.current!);
    if (count) {
      write(buffers.wholeItems, "fog-whole-items", up.whole_items());
      wholeUniform.write(
        d.vec4f(...(up.whole_uniform() as unknown as [number, number, number, number])),
      );
      pipelines.whole
        .with(faces)
        .with(
          root.createBindGroup(wholeLayout, {
            boxes: buffers.occluders.current!,
            whole: wholeUniform,
            flags: buffers.wholeFlags.current!,
            items: buffers.wholeItems.current!,
          }),
        )
        .with(encoder)
        .dispatchWorkgroups(x, y);
    }
    encoder.copyBufferToTexture(
      { buffer: buffers.wholeFlags.current!, bytesPerRow: WHOLE_TEXTURE_WIDTH * WORD },
      { texture: buffers.wholes.current!, origin: [0, flagsRow] },
      [WHOLE_TEXTURE_WIDTH, rows],
    );
  };

  /** The frame size's tile lists, made for the cull's tiles. */
  const ensureTiles = (t: FogTiles, [tilesX, tilesY, eyesMax]: Uint32Array) => {
    if (t.lists && t.tilesX === tilesX && t.tilesY === tilesY) return;
    const n = tilesX * tilesY;
    t.lists = t.scope.buffer({ label: "fog-tile-lists", size: n * eyesMax * WORD, usage: STORAGE });
    t.counts = t.scope.buffer({
      label: "fog-tile-counts",
      size: n * WORD,
      usage: STORAGE | COPY_DST | COPY_SRC,
    });
    t.tilesX = tilesX;
    t.tilesY = tilesY;
    generation++;
  };

  const readback = async (from: GPUBuffer, bytes: number): Promise<ArrayBuffer> => {
    const read = device.createBuffer({ size: bytes, usage: MAP_READ | COPY_DST });
    try {
      const encoder = device.createCommandEncoder({ label: "fog-readback" });
      encoder.copyBufferToBuffer(from, 0, read, 0, bytes);
      device.queue.submit([encoder.finish()]);
      await read.mapAsync(1);
      return read.getMappedRange().slice(0);
    } finally {
      read.destroy();
    }
  };
  const outBuffer = (bytes: number) =>
    device.createBuffer({ size: Math.max(16, bytes), usage: STORAGE | COPY_SRC });

  return {
    /** What fog is drawn from: the scene's presentation over the ground
     *  `heights` (none: flat), or none (everything seen). */
    setSource(next: FogSource | null, heights: TerrainGrid | null) {
      grid = heights ?? FLAT;
      generation++;
      if (next === source) return;
      source = next;
      // This renderer holds nothing of it yet.
      source?.resend_fog();
    },
    /** The ground paint's look on the painted layers. */
    setPaintStyle(style: {
      albedo: number;
      emissive: number;
      fog_keep: number;
      grass_glow: number;
      grass_falloff_m: number;
      saturation: number;
    }) {
      paintStyle.write({
        albedo: style.albedo,
        emissive: style.emissive,
        fogKeep: style.fog_keep,
        grassGlow: style.grass_glow,
        grassFalloff: style.grass_falloff_m,
        saturation: style.saturation,
        pad1: 0,
        pad2: 0,
      });
    },
    /** The tile lists for a frame size, owned by that size's scope. */
    sized(
      scope: GpuRegistry,
      depth: GPUTexture,
      paint: GPUTexture,
      sceneColour: GPUTexture,
      seenUnder: GPUTexture | null,
      waves: GPUTextureView,
    ): FogTiles {
      return {
        scope,
        tilesX: 0,
        tilesY: 0,
        lists: null,
        counts: null,
        depthView: depth.createView(),
        paintView: paint.createView(),
        sceneColourView: sceneColour.createView(),
        seenUnderView: seenUnder?.createView() ?? null,
        wavesView: waves,
      };
    },
    /** This frame's rebuilds, structures and tile cull over the prepass
     *  depth. `camera` is the frame's camera record. */
    encode(
      encoder: GPUCommandEncoder,
      frameTiles: FogTiles,
      camera: Float32Array,
      width: number,
      height: number,
    ) {
      tiles = frameTiles;
      const up = source?.prepare_fog(camera, width, height, 0);
      if (!up) {
        if (active)
          device.queue.writeBuffer(root.unwrap(params), 0, new Uint32Array(FOG_PARAMS_WORDS));
        active = false;
        return;
      }
      try {
        const dispatch = upload(up);
        if (dispatch) rebuildMaps(encoder, dispatch);
        if (!active) return;
        ensureTiles(frameTiles, up.tiles());
        const { cull, faces } = groups();
        if (up.whole().length) encodeWholes(encoder, up, faces);
        encoder.clearBuffer(frameTiles.counts!);
        pipelines.cull
          .with(cull!)
          .with(encoder)
          .dispatchWorkgroups(frameTiles.tilesX, frameTiles.tilesY);
      } finally {
        up.free();
      }
    },
    /** The fragment bind groups: ground, faces standing on it, and units
     *  (never fogged). */
    groups,
    stats(): FogVisibilityStats {
      const rust = source?.fog_stats();
      const s = rust
        ? (JSON.parse(rust) as Omit<FogVisibilityStats, "tileListBytes" | "tiles">)
        : null;
      const eyesMax = tiles?.lists ? tiles.lists.size / WORD / (tiles.tilesX * tiles.tilesY) : 0;
      return {
        enabled: (s?.enabled ?? false) && active,
        eyes: s?.eyes ?? 0,
        slots: s?.slots ?? 0,
        occluders: s?.occluders ?? 0,
        rebuilt: s?.rebuilt ?? 0,
        pending: s?.pending ?? 0,
        rebuiltTotal: s?.rebuiltTotal ?? 0,
        mapBytes: s?.mapBytes ?? 0,
        terrainMapBytes: s?.terrainMapBytes ?? 0,
        tileListBytes: tiles?.lists ? tiles.tilesX * tiles.tilesY * (eyesMax + 1) * WORD : 0,
        tiles: tiles?.lists ? [tiles.tilesX, tiles.tilesY] : [0, 0],
      };
    },
    /** Fog at `points` against every eye, after building any maps still due. */
    async probe(points: readonly FogProbeInput[]): Promise<Uint8Array> {
      const up = source?.prepare_fog(undefined, 0, 0, points.length);
      if (!up?.active()) {
        up?.free();
        return new Uint8Array(points.length).fill(1);
      }
      const records = probeRecords(points);
      const input = storage("fog-probe-points", records.byteLength);
      const out = outBuffer(points.length * WORD);
      try {
        device.queue.writeBuffer(input, 0, records);
        const encoder = device.createCommandEncoder({ label: "fog-probe" });
        // Uploading can replace the buffers the groups bind: bind after it.
        const dispatch = upload(up);
        if (dispatch) rebuildMaps(encoder, dispatch);
        pipelines.probe
          .with(groups().faces)
          .with(root.createBindGroup(probeLayout, { points: input, out }))
          .with(encoder)
          .dispatchWorkgroups(Math.max(1, Math.ceil(points.length / BUILD_WORKGROUP)));
        device.queue.submit([encoder.finish()]);
        return Uint8Array.from(new Uint32Array(await readback(out, points.length * WORD)));
      } finally {
        up.free();
        input.destroy();
        out.destroy();
      }
    },
    /** Rebuild every eye's maps on the next frame, as if all had moved (a
     *  cost probe). */
    rebuildAll() {
      source?.rebuild_fog();
    },
    /** The per-tile eye counts of the last frame (a debug readback). */
    async tileCounts(): Promise<Uint32Array> {
      if (!tiles?.counts) return new Uint32Array(0);
      const bytes = tiles.tilesX * tiles.tilesY * WORD;
      return new Uint32Array(await readback(tiles.counts, bytes));
    },
    /** The structures that take fog whole, and whether each was flagged
     *  seen by the last frame (a debug readback). */
    async wholes(): Promise<{ boxes: readonly FogOccluder[]; seen: Uint8Array }> {
      const boxes = occluderBoxes(source?.fog_occluders() ?? new Float64Array(0));
      if (!boxes.length) return { boxes, seen: new Uint8Array(0) };
      const flags = new Uint32Array(
        await readback(buffers.wholeFlags.current!, boxes.length * WORD),
      );
      return { boxes, seen: Uint8Array.from(flags) };
    },
    /** The WGSL sight shape at `[front, side, rear, off]` rows. */
    async probeShape(rows: Float32Array): Promise<Float32Array> {
      const n = rows.length / 4;
      const input = storage("fog-shape-rows", rows.byteLength);
      const out = outBuffer(n * WORD);
      try {
        device.queue.writeBuffer(input, 0, rows);
        const encoder = device.createCommandEncoder({ label: "fog-shape-probe" });
        pipelines.shape
          .with(root.createBindGroup(shapeLayout, { rows: input, out }))
          .with(encoder)
          .dispatchWorkgroups(Math.max(1, Math.ceil(n / BUILD_WORKGROUP)));
        device.queue.submit([encoder.finish()]);
        return new Float32Array(await readback(out, n * WORD));
      } finally {
        input.destroy();
        out.destroy();
      }
    },
  };
}
export type FogVisibility = Awaited<ReturnType<typeof createFogVisibility>>;
/** The lab's hold on the sight lights: debug readbacks, never in a frame. */
export type FogProbes = Pick<
  FogVisibility,
  "probe" | "probeShape" | "tileCounts" | "rebuildAll" | "wholes"
>;
