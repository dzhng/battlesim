import type { ScenePresentation } from "@web/battle/sim/presentation";

/** Read-only pose diagnostics used by browser scenes to inspect model rows. */
export function unitPoseProbe(current: () => ScenePresentation | null) {
  return {
    vehicles: () => current()?.probe().vehicles ?? [],
    soldiers: () => current()?.probe().soldiers ?? [],
  };
}
