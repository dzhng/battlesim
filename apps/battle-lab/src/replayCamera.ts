/** Human view history is presentation data, independent of commands and digests. */
import { deltaAngle, lerp, vec3 } from "math";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { SideName } from "@web/battle/sim/protocol";

export interface ReplayCameraTrack {
  player: string;
  side: SideName;
  /** Presentation time in battle seconds; the resolved camera the human saw. */
  samples: { time: number; camera: Camera3DParams }[];
}

/** Files are imported JSON; invalid framing must fail before it reaches the viewport. */
export function validateReplayCamera(track: ReplayCameraTrack): void {
  if (
    !track ||
    typeof track.player !== "string" ||
    !["blue", "red"].includes(track.side) ||
    !Array.isArray(track.samples)
  )
    throw new Error("invalid replay camera");
  let previous = -1;
  for (const sample of track.samples) {
    const c = sample?.camera;
    if (
      !sample ||
      !Number.isFinite(sample.time) ||
      sample.time < 0 ||
      sample.time <= previous ||
      !c ||
      !Array.isArray(c.target) ||
      c.target.length !== 3 ||
      !c.target.every(Number.isFinite) ||
      ![c.distance, c.pitch, c.yaw, c.fovY, c.near].every(Number.isFinite) ||
      c.distance <= 0 ||
      c.near <= 0 ||
      c.fovY <= 0 ||
      c.fovY >= Math.PI ||
      (c.far !== undefined && (!Number.isFinite(c.far) || c.far <= c.near))
    ) {
      throw new Error("invalid replay camera sample");
    }
    previous = sample.time;
  }
}

function sameCamera(a: Camera3DParams, b: Camera3DParams): boolean {
  return (
    vec3.exactEquals(a.target, b.target) &&
    a.distance === b.distance &&
    a.pitch === b.pitch &&
    a.yaw === b.yaw &&
    a.fovY === b.fovY &&
    a.near === b.near &&
    a.far === b.far
  );
}

/** Sample movement at most ten times per battle second. A stationary span
 *  keeps both ends, so playback never creeps toward a much later move. */
export function recordCamera(track: ReplayCameraTrack, time: number, camera: Camera3DParams) {
  const samples = track.samples;
  const last = samples.at(-1);
  if (last && time - last.time < 0.1) return;
  const previous = samples.at(-2);
  if (last && previous && sameCamera(last.camera, camera) && sameCamera(previous.camera, camera)) {
    last.time = time;
    return;
  }
  samples.push({ time, camera: { ...camera, target: vec3.clone(camera.target) } });
}

/** Read by battle time, including backwards seeks and frames past the recording. */
export function replayCameraAt(
  out: Camera3DParams,
  track: ReplayCameraTrack,
  time: number,
): boolean {
  const samples = track.samples;
  if (!samples.length) return false;
  let lo = 0;
  let hi = samples.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (samples[mid].time <= time) lo = mid + 1;
    else hi = mid;
  }
  const a = samples[Math.max(0, lo - 1)];
  const b = samples[Math.min(samples.length - 1, lo)];
  const t = a === b ? 0 : Math.max(0, (time - a.time) / (b.time - a.time));
  vec3.lerp(out.target, a.camera.target, b.camera.target, t);
  out.distance = lerp(a.camera.distance, b.camera.distance, t);
  out.pitch = lerp(a.camera.pitch, b.camera.pitch, t);
  out.yaw = a.camera.yaw + deltaAngle(a.camera.yaw, b.camera.yaw) * t;
  out.fovY = lerp(a.camera.fovY, b.camera.fovY, t);
  out.near = a.camera.near;
  out.far = a.camera.far;
  return true;
}
