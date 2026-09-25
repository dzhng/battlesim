import type { Camera3DParams } from "./camera3d";

// Player camera gestures as pure transforms of the one camera3d parameter set.
// Pan moves the orbit target across the ground in screen-aligned directions.

const MIN_PITCH = 0.12;
const MAX_PITCH = Math.PI / 2 - 0.02;

export function panCamera(p: Camera3DParams, right: number, forward: number): Camera3DParams {
  // Eye sits at yaw around the target, so screen-forward is −(cos yaw, sin yaw).
  const fx = -Math.cos(p.yaw),
    fy = -Math.sin(p.yaw);
  // Screen-right = forward × up.
  const rx = fy,
    ry = -fx;
  const scale = p.distance;
  return {
    ...p,
    target: [
      p.target[0] + (rx * right + fx * forward) * scale,
      p.target[1] + (ry * right + fy * forward) * scale,
      p.target[2],
    ],
  };
}

export function orbitCamera(p: Camera3DParams, dYaw: number, dPitch: number): Camera3DParams {
  return {
    ...p,
    yaw: p.yaw + dYaw,
    pitch: Math.min(MAX_PITCH, Math.max(MIN_PITCH, p.pitch + dPitch)),
  };
}

export function zoomCamera(
  p: Camera3DParams,
  factor: number,
  minDistance = 5,
  maxDistance = 4000,
): Camera3DParams {
  return { ...p, distance: Math.min(maxDistance, Math.max(minDistance, p.distance * factor)) };
}
