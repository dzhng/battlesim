// Held static prop records: publication keys own buffers until no longer listed.
import type { GpuRegistry } from "../frame/registry";
import type { WorldAppearance } from "./modelLayer";
import {
  BATCH_WORDS,
  Batch,
  CAST_PASS,
  NONE,
  INSTANCE_WORDS,
  type frameViews,
} from "../records.gen";

const RECORD_FLOATS = INSTANCE_WORDS;
const VERTEX_USAGE = 0x20 | 0x08; // VERTEX | COPY_DST
/** A prop's row (`Appearance.draws`): fogged face by face, or a surface
 *  movers stand on, which takes the ground paint. */
const PROP = 7;
const PROP_GROUND = 8;

const isProp = (row: WorldAppearance | undefined) =>
  !!row?.placed && (row.draws === PROP || row.draws === PROP_GROUND);

/** One of the props' static batches as last written: its records' buffer
 *  (a power of two of records, in a scope of its own), its row, tier,
 *  caster tier and fog class (`Batch`), and how many it holds. */
export interface HeldProps {
  scope: GpuRegistry | null;
  buffer: GPUBuffer | null;
  row: number;
  tier: number;
  pass: number;
  caster: number;
  cascadeMask: number;
  fog: number;
  count: number;
}

export function createPropBatches() {
  return {
    held: new Map<number, HeldProps>(),
    listed: [] as HeldProps[],
    scope: null as GpuRegistry | null,
  };
}
export type PropBatches = ReturnType<typeof createPropBatches>;

export function clearPropBatches(state: PropBatches) {
  state.scope?.release();
  state.scope = null;
  state.held.clear();
  state.listed = [];
}
/** The props' batches of a presentation frame: their records copied
 *  where the frame wrote them, and those it no longer lists dropped (the
 *  frame writes one anew when it comes back). The buffers held are those
 *  of the batches listed, a power of two of records each: the same view
 *  holds the same buffers, however it was reached. True when what is
 *  drawn changed. */
export function setPropBatches(
  state: PropBatches,
  frame: ReturnType<typeof frameViews> | null,
  worldRows: readonly WorldAppearance[],
  registry: GpuRegistry,
  device: GPUDevice,
): boolean {
  let changed = false;
  const next: HeldProps[] = [];
  const listed = new Set<number>();
  const words = frame?.batches.u32;
  for (let b = 0; b < (frame?.batches.count ?? 0); b++) {
    const o = b * BATCH_WORDS;
    const row = words![o + Batch.appearance];
    if (!isProp(worldRows[row])) continue;
    const key = words![o + Batch.key];
    let h = state.held.get(key);
    if (!h)
      state.held.set(
        key,
        (h = {
          scope: null,
          buffer: null,
          row,
          tier: 0,
          pass: 0,
          caster: NONE,
          cascadeMask: 0,
          fog: 0,
          count: 0,
        }),
      );
    const [first, count] = [words![o + Batch.first], words![o + Batch.count]];
    [h.row, h.tier, h.pass, h.caster, h.cascadeMask, h.fog, h.count] = [
      row,
      words![o + Batch.tier],
      words![o + Batch.pass],
      words![o + Batch.caster],
      words![o + Batch.cascade_mask],
      words![o + Batch.fog],
      count,
    ];
    if (first !== NONE) {
      const size = 2 ** Math.ceil(Math.log2(Math.max(1, count))) * RECORD_FLOATS * 4;
      if (h.buffer?.size !== size) {
        h.scope?.release();
        state.scope ??= registry.scope();
        h.scope = state.scope.scope();
        h.buffer = h.scope.buffer({ label: "model-prop-records", size, usage: VERTEX_USAGE });
      }
      const records = frame!.instances.f32.subarray(
        first * RECORD_FLOATS,
        (first + count) * RECORD_FLOATS,
      );
      device.queue.writeBuffer(h.buffer!, 0, records as Float32Array<ArrayBuffer>);
      changed = true;
    }
    listed.add(key);
    next.push(h);
  }
  for (const [key, h] of state.held)
    if (!listed.has(key)) {
      h.scope?.release();
      state.held.delete(key);
      changed = true;
    }
  state.listed = next;
  return changed;
}

/** Camera prop counts exclude independent caster submissions. */
export function drawnProps(state: PropBatches, worldRows: readonly WorldAppearance[]) {
  const drawn = new Map<string, number[]>();
  for (const h of state.listed) {
    if (h.pass === CAST_PASS) continue;
    const name = worldRows[h.row]?.name;
    if (!name) continue;
    let levels = drawn.get(name);
    if (!levels) drawn.set(name, (levels = [0, 0, 0, 0]));
    levels[h.tier] += h.count;
  }
  return [...drawn]
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([appearance, levels]) => ({ appearance, levels }));
}
