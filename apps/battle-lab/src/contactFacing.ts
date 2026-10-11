// How a view's aloft contact signs face its camera (D11): the camera's
// screen axes, followed in steps of a couple of degrees so a turning camera
// rebuilds the overlay only when a sign would visibly turn, and the scale
// the view draws its strokes at.
import { useCallback, useMemo, useRef, useState } from "react";
import { screenAxes, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { ContactFacing } from "@packages/battle-renderer/src/contactGlyph";
import { gameStroke } from "./gameOverlay";

/** A sign turned this far from the camera is still a circle to the eye
 *  (its narrow axis shrinks by under 0.1%). */
const STEP_RAD = (2 * Math.PI) / 180;

type Orbit = Pick<Camera3DParams, "yaw" | "pitch">;
const stepOf = (v: Orbit): Orbit => ({
  yaw: Math.round(v.yaw / STEP_RAD) * STEP_RAD,
  pitch: Math.round(v.pitch / STEP_RAD) * STEP_RAD,
});

/** The facing of signs seen through a camera opening at `initial`, drawn
 *  where a pixel spans `metresPerPx`. Call `follow` with the camera each
 *  frame. */
export function useContactFacing(initial: Camera3DParams, metresPerPx: number) {
  const [orbit, setOrbit] = useState(() => stepOf(initial));
  const shown = useRef(orbit);
  const follow = useCallback((camera: Camera3DParams) => {
    const next = stepOf(camera);
    if (next.yaw === shown.current.yaw && next.pitch === shown.current.pitch) return;
    shown.current = next;
    setOrbit(next);
  }, []);
  const facing = useMemo<ContactFacing>(
    () => ({ ...screenAxes(orbit), stroke: gameStroke(metresPerPx) }),
    [orbit, metresPerPx],
  );
  return { facing, follow };
}
