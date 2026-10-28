// One immutable GPU copy of the public sampled ground, shared by fog and grass.
// The directory and height bits occupy one storage binding, so paging does not
// consume another binding on passes already at the device's storage limit.
import { tgpu, d, type TgpuFn } from "typegpu";
import { terrain_heights } from "@wasm/renderer_browser";
import type { TerrainGrid } from "../terrain/terrainGrid";
import type { GpuRegistry } from "./registry";

type WordReader = TgpuFn<(index: typeof d.u32) => typeof d.u32>;

/** Read the same vertex page convention as groundSample on the CPU. */
export function terrainSample(readWord: WordReader) {
  return tgpu
    .fn(
      [d.u32, d.u32],
      d.f32,
    )(/* wgsl */ `(i: u32, j: u32) -> f32 {
    let size = readWord(0u);
    let cols = readWord(1u);
    let count = readWord(2u);
    if (count == 0u) { return 0.0; }
    let page = readWord(3u + (j / size) * cols + i / size);
    if (page == 0u) { return 0.0; }
    let at = 3u + count + (page - 1u) * size * size + (j % size) * size + i % size;
    return bitcast<f32>(readWord(at));
  }`)
    .$uses({ readWord });
}

export function createTerrainHeights(registry: GpuRegistry) {
  const slot = registry.slot<GPUBuffer>();
  let current: TerrainGrid | null = null;
  return {
    forGrid(grid: TerrainGrid): GPUBuffer {
      if (
        current?.heights !== grid.heights ||
        current?.pageIds !== grid.pageIds ||
        current?.nx !== grid.nx ||
        current?.ny !== grid.ny ||
        current?.pageSize !== grid.pageSize
      ) {
        // Header (page size, columns, directory length), directory (page + 1,
        // or 0), then the present pages' f32 height bits (`Grid::pages`).
        const words = terrain_heights(grid.nx, grid.ny, grid.pageSize, grid.pageIds, grid.heights);
        slot.set(
          registry.device.createBuffer({
            label: "terrain-heights",
            size: words.byteLength,
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
          }),
        );
        registry.device.queue.writeBuffer(slot.current!, 0, words);
        current = grid;
      }
      return slot.current!;
    },
  };
}

export type TerrainHeights = ReturnType<typeof createTerrainHeights>;
