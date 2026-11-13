// What a `ShowcaseStage` drew, read back for the browser scenes (the showcase
// lab's and the Armory's): each model in its last frame, how far each stands
// off the ground, and where a unit falls on screen.
import { mat4, type Vec3 } from "math";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { farPoseBounds } from "@packages/scene-assets/src/pose";
import {
  createProjectedPoint,
  projectPoint,
  viewProjMatrix,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d";
import {
  Batch,
  BATCH_WORDS,
  INSTANCE_WORDS,
  JOINT_WORDS,
} from "@packages/battle-renderer/src/records.gen";
import type { ShowcaseStage } from "@web/battle/render/showcaseStage";

export interface DrawnModel {
  name: string;
  at: Vec3;
  yaw: number;
  /** Its palette's first joints: its pose. */
  pose: number[];
}

/** Each model `stage` drew last frame: its appearance, where it stands and
 *  faces, and its pose. */
export function drawnModels(stage: ShowcaseStage): DrawnModel[] {
  const { frame, rows } = stage.records();
  const b = frame.batches.u32;
  const inst = frame.instances.f32;
  const joints = frame.palettes.f32;
  const out: DrawnModel[] = [];
  for (let i = 0; i < frame.batches.count; i++) {
    const w = i * BATCH_WORDS;
    const first = b[w + Batch.first];
    if (first === 0xffffffff) continue;
    const name = rows[b[w + Batch.appearance]]?.name;
    if (!name) continue;
    for (let k = first; k < first + b[w + Batch.count]; k++) {
      const v = k * INSTANCE_WORDS;
      const palette = inst[v + 4] * JOINT_WORDS;
      out.push({
        name,
        at: [inst[v], inst[v + 1], inst[v + 2]],
        yaw: inst[v + 3],
        pose: Array.from(joints.slice(palette, palette + 8 * JOINT_WORDS)),
      });
    }
  }
  return out;
}

/** `name`'s bounds at rest in its own frame, from `installed`; null when it
 *  is not installed. */
export function restBounds(installed: InstalledAppearances, name: string) {
  const a = installed.appearances.get(name);
  if (!a) return null;
  const skeleton =
    a.bundle.kind === "skinned" ? (installed.skeletons.get(a.bundle.skeleton) ?? null) : null;
  return farPoseBounds(a.bundle, skeleton);
}

/** How far each of `models`' lowest point at rest is from the ground under
 *  it, metres: its placement's height plus its far pose's lowest point. */
export function grounded(models: readonly DrawnModel[], installed: InstalledAppearances) {
  return models.map(({ name, at }) => {
    const bounds = restBounds(installed, name);
    return { name, gap: bounds ? at[2] + bounds.min[2] : null };
  });
}

/** The world box round `models` at rest, each turned by its yaw; null for
 *  none. */
export function worldBox(models: readonly DrawnModel[], installed: InstalledAppearances) {
  const min: Vec3 = [Infinity, Infinity, Infinity];
  const max: Vec3 = [-Infinity, -Infinity, -Infinity];
  for (const { name, at, yaw } of models) {
    const bounds = restBounds(installed, name);
    if (!bounds) continue;
    const [c, s] = [Math.cos(yaw), Math.sin(yaw)];
    for (const x of [bounds.min[0], bounds.max[0]])
      for (const y of [bounds.min[1], bounds.max[1]])
        for (const z of [bounds.min[2], bounds.max[2]]) {
          const p: Vec3 = [at[0] + c * x - s * y, at[1] + s * x + c * y, at[2] + z];
          for (let i = 0; i < 3; i++) {
            min[i] = Math.min(min[i], p[i]);
            max[i] = Math.max(max[i], p[i]);
          }
        }
  }
  return min[0] <= max[0] ? { min, max } : null;
}

/** Where world point `at` falls in a `width` × `height` view from `orbit`,
 *  in its pixels; null behind the eye. */
export function project(
  orbit: Camera3DParams,
  width: number,
  height: number,
  at: Vec3,
): [number, number] | null {
  const viewProj = viewProjMatrix(mat4.create(), { ...orbit, aspect: width / height });
  const q = projectPoint(createProjectedPoint(), viewProj, at);
  if (q.clipW <= 0) return null;
  return [((q.ndc[0] + 1) / 2) * width, ((1 - q.ndc[1]) / 2) * height];
}

/** The screen rectangle round `box`'s corners in a `width` × `height` view
 *  from `orbit`; null when a corner is behind the eye. */
export function screenBox(
  orbit: Camera3DParams,
  width: number,
  height: number,
  box: { min: Vec3; max: Vec3 },
) {
  const points: [number, number][] = [];
  for (const x of [box.min[0], box.max[0]])
    for (const y of [box.min[1], box.max[1]])
      for (const z of [box.min[2], box.max[2]]) {
        const q = project(orbit, width, height, [x, y, z]);
        if (!q) return null;
        points.push(q);
      }
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const [x0, y0] = [Math.min(...xs), Math.min(...ys)];
  return { x: x0, y: y0, width: Math.max(...xs) - x0, height: Math.max(...ys) - y0 };
}
