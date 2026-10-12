// @vitest-environment node
// The scars' upload boundary: what the presentation's scar cache says to
// write (`crates/presentation/src/scars`, held to the browser's words in its
// tests) reaches the frame's textures where the shader reads it.
import { expect, test, vi } from "vitest";
import { Battle, initSync } from "@wasm/game_wasm.js";
import { readFileSync } from "node:fs";
import { GpuRegistry } from "@packages/battle-renderer/src/frame/registry";
import { createScarTexture } from "@packages/battle-renderer/src/frame/scarTexture";
import {
  SCAR_NORMALIZATION_WORDS,
  SCAR_POOL_WIDTH,
} from "@packages/battle-renderer/src/records.gen";
import { cellPatchRuns, groundRecords } from "./groundRuns";
import type { ObservationLayout } from "../src/battle/sim/observationView";
import { ObservationDecoder, loadPresentation } from "../src/battle/sim/presentation";
import { labScenario } from "./catalog";
import { loadMap } from "@web/maps/node";

await loadPresentation();
initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });

test("the cache's directory, pool texels and params reach the frame's textures", () => {
  vi.stubGlobal("GPUTextureUsage", { TEXTURE_BINDING: 4, COPY_DST: 2 });
  const writes: { texture: string; origin: number[]; size: number[]; bytes: number }[] = [];
  const device = {
    createBuffer: () => ({ destroy() {} }),
    createTexture: (d: GPUTextureDescriptor) => ({ label: d.label, destroy() {} }),
    limits: { maxTextureDimension2D: 8192 },
    queue: {
      writeTexture(
        target: { texture: { label: string }; origin?: number[] },
        data: ArrayBufferView,
        _layout: unknown,
        size: number[],
      ) {
        writes.push({
          texture: target.texture.label,
          origin: target.origin ?? [0, 0, 0],
          size,
          bytes: data.byteLength,
        });
      },
    },
  } as unknown as GPUDevice;
  const registry = new GpuRegistry(device);
  const battle = new Battle(labScenario(loadMap("ground").definition, []), 1);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  battle.free();
  try {
    const textures = createScarTexture(registry);
    // The normalization table first, the pool's first rows.
    expect(writes).toEqual([
      {
        texture: "ground-scar-words",
        origin: [0, 0, 0],
        size: [SCAR_POOL_WIDTH, SCAR_NORMALIZATION_WORDS / SCAR_POOL_WIDTH],
        bytes: SCAR_NORMALIZATION_WORDS * 4,
      },
    ]);
    writes.length = 0;
    const decoder = new ObservationDecoder(layout);
    const ground = decoder.ground!;
    // One varying page and one uniform tile.
    const cols = layout.ground.cols;
    decoder.decode(
      groundRecords(layout)(
        cellPatchRuns(cols, {
          epoch: 1,
          side: "blue",
          baseRevision: 0,
          revision: 1,
          full: true,
          cells: Uint32Array.of(80 * cols + 32, 80 * cols + 33),
          marks: Uint8Array.of(19, 0, 0, 0, 73, 0, 0, 0),
          cleared: new Uint8Array(2),
        }),
      ),
    );
    ground.resend();
    expect(ground.changed()).toBe(true);
    const upload = ground.prepare();
    expect(upload.params.length).toBe(20);
    expect(upload.directory).not.toBeNull();
    textures.upload(upload);
    const { width, height } = upload.directory!;
    expect(writes[0]).toEqual({
      texture: "ground-scar-directory",
      origin: [0, 0, 0],
      size: [width, height],
      bytes: width * height * 8,
    });
    // Each pool write lands at its offset, row by row.
    const pool = writes.slice(1);
    expect(pool.length).toBe(upload.writes.length / 2);
    pool.forEach((w, k) => {
      const [offset, texels] = [upload.writes[k * 2], upload.writes[k * 2 + 1]];
      expect(offset).toBeGreaterThanOrEqual(SCAR_NORMALIZATION_WORDS);
      expect(w).toEqual({
        texture: "ground-scar-words",
        origin: [offset % SCAR_POOL_WIDTH, Math.floor(offset / SCAR_POOL_WIDTH), 0],
        size: [texels, 1],
        bytes: texels * 4,
      });
    });
    expect(pool.reduce((n, w) => n + w.bytes, 0)).toBe(upload.texels.length);
    // Nothing changed: nothing to write.
    writes.length = 0;
    expect(ground.changed()).toBe(false);
    textures.upload(ground.prepare());
    expect(writes).toEqual([]);
  } finally {
    registry.release();
    vi.unstubAllGlobals();
  }
});
