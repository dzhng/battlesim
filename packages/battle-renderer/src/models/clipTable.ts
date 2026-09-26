// Skinned clips laid out for the pose kernel: per body, every clip's frames
// sampled densely (joint-major within a frame), each joint as two vec4s —
// rotation (x, y, z, w) and translation plus uniform scale. Channels a clip
// leaves absent take the body's bind, so the kernel never branches on them.
// Built once per installed appearance; the per-frame work is `clipFrames`.

import type { Clip, Joint, SkeletonClips } from "@packages/scene-assets/src/schema";
import { CHANNEL_ABSENT, CHANNEL_ANIMATED } from "@packages/scene-assets/src/schema";

/** Floats per joint sample: rotation vec4, translation vec3 + scale. */
export const SAMPLE_FLOATS = 8;

export interface TableClip {
  name: string;
  /** First float of the clip's frame 0 in the table. */
  offset: number;
  frames: number;
  duration: number;
  /** Samples per second: the skeleton's `sample_hz`. */
  rate: number;
  loop: boolean;
  stride_m: number | null;
}

export interface ClipTable {
  joints: number;
  data: Float32Array<ArrayBuffer>;
  clips: Map<string, TableClip>;
}

/** Dense samples for one body on its skeleton's clips. */
export function buildClipTable(skeleton: SkeletonClips, joints: readonly Joint[]): ClipTable {
  const n = joints.length;
  const total = skeleton.clips.reduce((sum, c) => sum + c.frames, 0) * n * SAMPLE_FLOATS;
  const data = new Float32Array(total);
  const clips = new Map<string, TableClip>();
  let offset = 0;
  for (const clip of skeleton.clips) {
    clips.set(clip.name, {
      name: clip.name,
      offset,
      frames: clip.frames,
      duration: clip.duration,
      rate: skeleton.sample_hz,
      loop: clip.loop,
      stride_m: clip.stride_m ?? null,
    });
    writeClip(data, offset, clip, joints);
    offset += clip.frames * n * SAMPLE_FLOATS;
  }
  return { joints: n, data, clips };
}

function writeClip(data: Float32Array, offset: number, clip: Clip, joints: readonly Joint[]) {
  const n = joints.length;
  let r = 0;
  let t = 0;
  for (let j = 0; j < n; j++) {
    const bind = joints[j].bind;
    const rMode = clip.rotation_modes[j];
    const tMode = clip.translation_modes[j];
    for (let f = 0; f < clip.frames; f++) {
      const o = offset + (f * n + j) * SAMPLE_FLOATS;
      if (rMode === CHANNEL_ABSENT) data.set(bind.r, o);
      else {
        const s = (r + (rMode === CHANNEL_ANIMATED ? f : 0)) * 4;
        for (let c = 0; c < 4; c++) data[o + c] = clip.rotations[s + c] / 32767;
      }
      if (tMode === CHANNEL_ABSENT) data.set(bind.t, o + 4);
      else {
        const s = (t + (tMode === CHANNEL_ANIMATED ? f : 0)) * 3;
        for (let c = 0; c < 3; c++) data[o + 4 + c] = clip.translations[s + c];
      }
      // Scale always comes from the body's bind; binds are uniform-scaled.
      data[o + 7] = bind.s[0];
    }
    if (rMode !== CHANNEL_ABSENT) r += rMode === CHANNEL_ANIMATED ? clip.frames : 1;
    if (tMode !== CHANNEL_ABSENT) t += tMode === CHANNEL_ANIMATED ? clip.frames : 1;
  }
}

export interface ClipFrames {
  f0: number;
  f1: number;
  /** Weight of `f1`. */
  w: number;
}

/**
 * The two frames either side of `phase` (0..1 of the clip's duration) and the
 * weight between them, as scene-assets' `sampleClip` reads them. A looping
 * clip wraps its phase (the bake samples both ends, so they agree); a one-shot
 * clip holds its ends.
 */
export function clipFrames(out: ClipFrames, clip: TableClip, phase: number): ClipFrames {
  const last = clip.frames - 1;
  if (last <= 0) {
    out.f0 = 0;
    out.f1 = 0;
    out.w = 0;
    return out;
  }
  const p = clip.loop ? phase - Math.floor(phase) : Math.min(1, Math.max(0, phase));
  const frame = Math.min(last, p * clip.duration * clip.rate);
  const f0 = Math.min(last, Math.floor(frame));
  out.f0 = f0;
  out.f1 = Math.min(last, f0 + 1);
  out.w = frame - f0;
  return out;
}
