/** The session borrows the viewport's live renderer handle for telemetry. */
import { useCallback, useRef } from "react";
import type { AllocationCounts } from "@web/battle/render/sceneStats";
import type { ViewportGpu } from "./viewportRender";

export function useViewportGpu(onViewportReady: () => void) {
  const gpu = useRef<ViewportGpu | null>(null);
  const onReady = useCallback(
    (viewport: ViewportGpu) => {
      gpu.current = viewport;
      onViewportReady();
    },
    [onViewportReady],
  );
  const gpuAllocations = useCallback(
    (): AllocationCounts | null => gpu.current?.allocations() ?? null,
    [],
  );
  const gpuFrame = useCallback(() => gpu.current?.frame().stats().gpu ?? null, []);
  const onDispose = useCallback(() => {
    gpu.current = null;
  }, []);

  return { onReady, onDispose, gpuAllocations, gpuFrame };
}
