import { useRef, useState } from "react";
import type { OwnUnitView } from "@web/battle/sim/observationView";
import type { CameraFocusRequest } from "./viewportRender";

/** One-shot camera focuses on the player's own units: every call produces a
 *  new token, so clicking the same card again still moves the camera. */
export function useUnitFocus() {
  const token = useRef(0);
  const [cameraFocus, setCameraFocus] = useState<CameraFocusRequest | null>(null);
  const focusUnit = (unit: OwnUnitView) =>
    setCameraFocus({
      token: ++token.current,
      target: [unit.position[0], unit.position[1]],
    });
  return { cameraFocus, focusUnit };
}
