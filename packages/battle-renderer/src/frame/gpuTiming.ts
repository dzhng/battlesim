// GPU time for the whole frame, from `timestamp-query`. Two marker passes
// bracket the frame: 1×1 cleared render passes carrying `timestampWrites`,
// because empty compute passes write no timestamps on Metal. Per-pass splits
// are not reported: Apple's tile-based GPU overlaps passes, so only the total
// means anything.
import type { GpuRegistry } from "./registry";

/** Frames kept for the rolling statistics. */
const WINDOW = 240;
/** Readbacks in flight at once; a frame with none free goes unmeasured. */
const READBACKS = 3;

export interface GpuFrameTime {
  /** Frames measured in the window. */
  frames: number;
  meanMs: number;
  p95Ms: number;
}

export function createFrameTimer(device: GPUDevice, registry: GpuRegistry) {
  if (!device.features.has("timestamp-query")) return null;
  const querySet = registry.own(device.createQuerySet({ type: "timestamp", count: 2 }));
  const resolve = registry.buffer({
    label: "frame-timer-resolve",
    size: 16,
    usage: GPUBufferUsage.QUERY_RESOLVE | GPUBufferUsage.COPY_SRC,
  });
  const readbacks = Array.from({ length: READBACKS }, () => ({
    buffer: registry.buffer({
      label: "frame-timer-readback",
      size: 16,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    }),
    busy: false,
    generation: 0,
  }));
  const marker = registry
    .texture({
      label: "frame-timer-marker",
      size: [1, 1],
      format: "r8unorm",
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    })
    .createView();
  const samples: number[] = [];
  let generation = 0;
  let frameGeneration = 0;
  let pending: (typeof readbacks)[number] | undefined;

  // The frame opens where the first marker's vertex stage starts, and closes
  // where the last marker's fragment stage ends: a pass's vertex stage may
  // run ahead of the fragment work before it (Dawn holds a pass back only at
  // the stage that reads), but fragment work runs in order.
  const mark = (encoder: GPUCommandEncoder, index: number) =>
    encoder
      .beginRenderPass({
        label: index === 0 ? "frame-begin" : "frame-end",
        colorAttachments: [{ view: marker, loadOp: "clear", storeOp: "store" }],
        timestampWrites:
          index === 0
            ? { querySet, beginningOfPassWriteIndex: 0 }
            : { querySet, endOfPassWriteIndex: 1 },
      })
      .end();

  return {
    begin(encoder: GPUCommandEncoder) {
      frameGeneration = generation;
      mark(encoder, 0);
    },
    /** Close the frame and queue its readback, if one is free. */
    end(encoder: GPUCommandEncoder) {
      mark(encoder, 1);
      pending = readbacks.find((r) => !r.busy);
      encoder.resolveQuerySet(querySet, 0, 2, resolve, 0);
      if (pending) {
        pending.generation = frameGeneration;
        encoder.copyBufferToBuffer(resolve, 0, pending.buffer, 0, 16);
      }
    },
    /** After submit: read the queued frame's time back. */
    collect() {
      const readback = pending;
      pending = undefined;
      if (!readback) return;
      readback.busy = true;
      readback.buffer
        .mapAsync(GPUMapMode.READ)
        .then(() => {
          const [t0, t1] = new BigUint64Array(readback.buffer.getMappedRange().slice(0));
          readback.buffer.unmap();
          if (readback.generation === generation && t0 !== 0n && t1 > t0) {
            samples.push(Number(t1 - t0) / 1e6);
            if (samples.length > WINDOW) samples.shift();
          }
        })
        // A buffer destroyed with the frame rejects its map: nothing to read.
        .catch(() => {})
        .finally(() => (readback.busy = false));
    },
    stats(): GpuFrameTime {
      const sorted = [...samples].sort((a, b) => a - b);
      const mean = sorted.reduce((a, b) => a + b, 0) / Math.max(1, sorted.length);
      const p95 = sorted[Math.min(sorted.length - 1, Math.floor((sorted.length - 1) * 0.95))] ?? 0;
      return { frames: sorted.length, meanMs: mean, p95Ms: p95 };
    },
    /** Start a window containing only frames begun after this call. Old
     *  readbacks retain their slots until mapping finishes, but cannot publish. */
    reset() {
      generation++;
      samples.length = 0;
    },
  };
}
