// Adapted from ~/dev/game battle-renderer/src/worldDepth.ts:
// the battle's depth states without renderer-core's pipelineContracts. Local
// changes: depth is written with the repo's one reverse-Z compare
// (`depthContract`, `greater`), and a "prepassed" mode shades surfaces whose
// depth a depth prepass already wrote, which must pass at equal depth; a
// "behind" mode passes only what lies behind the depth written (an x-ray).
import {
  GPU_DEPTH_CLEAR,
  GPU_DEPTH_COMPARE,
  GPU_DEPTH_FORMAT,
} from "@packages/renderer-core/src/depthContract";

/** The compare that passes what lies behind the stored depth, keyed by the
 *  engine's one compare so the two move together. */
const BEHIND = { greater: "less" } as const satisfies Record<
  typeof GPU_DEPTH_COMPARE,
  GPUCompareFunction
>;

export function battleWorldDepth(
  mode: "read" | "read-write" | "prepassed" | "behind",
): GPUDepthStencilState {
  return {
    format: GPU_DEPTH_FORMAT,
    depthWriteEnabled: mode === "read-write",
    depthCompare:
      mode === "prepassed"
        ? "greater-equal"
        : mode === "behind"
          ? BEHIND[GPU_DEPTH_COMPARE]
          : GPU_DEPTH_COMPARE,
  };
}

export const BATTLE_DEPTH_ATTACHMENT = {
  format: GPU_DEPTH_FORMAT,
  clearValue: GPU_DEPTH_CLEAR,
  loadOp: "clear",
  storeOp: "store",
} as const;
