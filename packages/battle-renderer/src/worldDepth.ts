// Adapted from ~/dev/game battle-renderer/src/worldDepth.ts (reuse manifest):
// the battle's depth states without renderer-core's pipelineContracts. Local
// changes: depth is written with the repo's one reverse-Z compare
// (`depthContract`, `greater`), and a "prepassed" mode shades surfaces whose
// depth a depth prepass already wrote, which must pass at equal depth.
import {
  GPU_DEPTH_CLEAR,
  GPU_DEPTH_COMPARE,
  GPU_DEPTH_FORMAT,
} from "@packages/renderer-core/src/depthContract";

export function battleWorldDepth(mode: "read" | "read-write" | "prepassed"): GPUDepthStencilState {
  return {
    format: GPU_DEPTH_FORMAT,
    depthWriteEnabled: mode === "read-write",
    depthCompare: mode === "prepassed" ? "greater-equal" : GPU_DEPTH_COMPARE,
  };
}

export const BATTLE_DEPTH_ATTACHMENT = {
  format: GPU_DEPTH_FORMAT,
  clearValue: GPU_DEPTH_CLEAR,
  loadOp: "clear",
  storeOp: "store",
} as const;
