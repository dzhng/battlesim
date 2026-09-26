// SPIKE 01 adaptation: the battle depth decision without renderer-core's
// pipelineContracts (whose frame-shell types we do not port).
import { GPU_DEPTH_CLEAR, GPU_DEPTH_FORMAT } from "../../renderer-core/src/depthContract";

/** Equal-depth overlaps must draw, so the world compare is `greater-equal`. */
export function battleWorldDepth(mode: "read" | "read-write"): GPUDepthStencilState {
  return {
    format: GPU_DEPTH_FORMAT,
    depthWriteEnabled: mode === "read-write",
    depthCompare: "greater-equal",
  };
}

export const BATTLE_DEPTH_ATTACHMENT = {
  format: GPU_DEPTH_FORMAT,
  clearValue: GPU_DEPTH_CLEAR,
  loadOp: "clear",
  storeOp: "store",
} as const;
