// The template line-up (`/lab/city-lineup`): where each building template
// stands, the references the frame draws them from, and the cameras a
// template is judged from. Pure: the route draws it
// through the frame's own buildings path, as a generated town is drawn.
//
// The line-up is rows of templates on flat ground at their real size, a
// category a row, each row's fronts (the descriptor's street side, -y) on a
// line and the camera to the south. Rows run south to north by their tallest
// building, so no row hides the one behind it.
import { vec3 } from "math";
import {
  createGpuMat4,
  createProjectedPoint,
  eyePosition,
  projectPoint,
  viewProjMatrix,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d";
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import type {
  FallenBuilding,
  PlacedBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import type { PropBox } from "@packages/battle-renderer/src/models/propAppearance";
import {
  templateArt,
  type TemplateArtLibrary,
  type TemplateState,
} from "@packages/scene-assets/src/templateLibrary";

/** A template as the line-up needs it: the catalogue's physical parts (the
 *  contract's descriptor), and the source set that dresses it. */
export interface LineupTemplate {
  id: string;
  category: string;
  set: string;
  parts: {
    center: [number, number];
    yaw: number;
    half_extents: [number, number, number];
    base_z: number;
  }[];
}

export interface LineupSpacing {
  /** Clear ground between neighbours in a row, metres. */
  gap_m: number;
  /** Clear ground between rows, metres. */
  row_gap_m: number;
  /** Clear ground round the whole line-up, metres. */
  margin_m: number;
  /** More ground behind it (north), in heights of its tallest building: a
   *  picture taken from the south sees that far past the roof, and should
   *  see ground there, not the map's edge. */
  backdrop: number;
  /** The map's size is a whole number of these, metres. */
  grid_m: number;
}

/** A template where it stands. */
export interface LineupEntry {
  id: string;
  category: string;
  set: string;
  /** Its frame on the map: x, y, z, then yaw (radians about +Z). */
  frame: [number, number, number, number];
  /** Its physical parts in the world. */
  parts: PropBox[];
  /** The box round its parts, in the world. */
  min: [number, number, number];
  max: [number, number, number];
}

export interface Lineup {
  /** The flat map that holds it, metres. */
  size: [number, number];
  entries: LineupEntry[];
  /** The rows, south to north: a category and the box round its buildings. */
  rows: { category: string; min: [number, number, number]; max: [number, number, number] }[];
}

/** The box round a template's parts, in its own frame. */
function planBox(template: LineupTemplate) {
  const min = [Infinity, Infinity, Infinity];
  const max = [-Infinity, -Infinity, -Infinity];
  for (const part of template.parts) {
    const [cos, sin] = [Math.abs(Math.cos(part.yaw)), Math.abs(Math.sin(part.yaw))];
    const [hx, hy, hz] = part.half_extents;
    const reach = [cos * hx + sin * hy, sin * hx + cos * hy];
    for (const axis of [0, 1]) {
      min[axis] = Math.min(min[axis], part.center[axis] - reach[axis]);
      max[axis] = Math.max(max[axis], part.center[axis] + reach[axis]);
    }
    min[2] = Math.min(min[2], part.base_z);
    max[2] = Math.max(max[2], part.base_z + 2 * hz);
  }
  return { min, max };
}

/** Stand `templates` in rows by category. */
export function lineUp(templates: readonly LineupTemplate[], spacing: LineupSpacing): Lineup {
  const boxes = new Map(templates.map((t) => [t.id, planBox(t)]));
  const byCategory = new Map<string, LineupTemplate[]>();
  for (const t of [...templates].sort((a, b) => a.id.localeCompare(b.id)))
    byCategory.set(t.category, [...(byCategory.get(t.category) ?? []), t]);
  const tallest = (list: LineupTemplate[]) => Math.max(...list.map((t) => boxes.get(t.id)!.max[2]));
  const categories = [...byCategory].sort(
    ([a, listA], [b, listB]) => tallest(listA) - tallest(listB) || a.localeCompare(b),
  );

  const entries: LineupEntry[] = [];
  const rows: Lineup["rows"] = [];
  let front = spacing.margin_m;
  let width = 0;
  let height = 0;
  for (const [category, list] of categories) {
    let left = spacing.margin_m;
    const row = { category, min: [left, front, 0], max: [left, front, 0] } as Lineup["rows"][0];
    for (const t of list) {
      const box = boxes.get(t.id)!;
      const [x, y] = [left - box.min[0], front - box.min[1]];
      const entry: LineupEntry = {
        id: t.id,
        category,
        set: t.set,
        frame: [x, y, 0, 0],
        parts: t.parts.map((p) => ({
          kind: "building",
          center: [x + p.center[0], y + p.center[1]],
          yaw: p.yaw,
          half: p.half_extents,
          baseZ: p.base_z,
        })),
        min: [x + box.min[0], y + box.min[1], box.min[2]],
        max: [x + box.max[0], y + box.max[1], box.max[2]],
      };
      entries.push(entry);
      for (const axis of [0, 1, 2]) row.max[axis] = Math.max(row.max[axis], entry.max[axis]);
      left = entry.max[0] + spacing.gap_m;
    }
    rows.push(row);
    width = Math.max(width, row.max[0]);
    height = Math.max(height, row.max[2]);
    front = row.max[1] + spacing.row_gap_m;
  }
  const whole = (m: number) => Math.max(1, Math.ceil(m / spacing.grid_m)) * spacing.grid_m;
  return {
    size: [
      whole(width + spacing.margin_m),
      whole(front - spacing.row_gap_m + spacing.margin_m + spacing.backdrop * height),
    ],
    entries,
    rows,
  };
}

/** The references the frame draws `entries` from. A building's owner is its
 *  template's place in id order, so its tint is the same whichever others
 *  stand with it. */
export function lineupBuildings(entries: readonly LineupEntry[]): PlacedBuildings {
  const templates = [...new Set(entries.map((e) => e.id))].sort();
  return {
    templates,
    template: Uint16Array.from(entries, (e) => templates.indexOf(e.id)),
    frames: Float64Array.from(entries.flatMap((e) => e.frame)),
    owners: Uint32Array.from(entries, (e) => ownerOf(e.id)),
  };
}

/** A stable number for a template id (FNV-1a, 32 bits). */
function ownerOf(id: string): number {
  let hash = 0x811c9dc5;
  for (let i = 0; i < id.length; i++) hash = Math.imul(hash ^ id.charCodeAt(i), 0x01000193);
  return hash >>> 0;
}

/** The `entries` whose templates `library` has rows for in `state`: what a
 *  line-up in that state holds. A template is destroyed into one damage state
 *  only, so a line-up of ruins leaves out what stands gutted, and the other
 *  way round. */
export function lineupIn(
  entries: readonly LineupEntry[],
  state: TemplateState,
  library: TemplateArtLibrary,
): LineupEntry[] {
  return entries.filter((e) => templateArt(library, e.id).states[state] !== undefined);
}

/** What the side knows of `entries` (as `lineupBuildings` lists them) when
 *  every one is in `state`: nothing for intact, else each destroyed into it. */
export function lineupFallen(
  entries: readonly LineupEntry[],
  state: TemplateState,
): FallenBuilding[] {
  if (state === "intact") return [];
  return entries.map((_, building) => ({ building, state }));
}

const _pose_eye = vec3.create();

/** The eye's distance to the nearest point of `entry`'s box when the camera
 *  orbits the middle of its footprint from `distance`. */
function rangeFrom(entry: LineupEntry, distance: number, yaw: number, pitch: number): number {
  eyePosition(_pose_eye, {
    target: [(entry.min[0] + entry.max[0]) / 2, (entry.min[1] + entry.max[1]) / 2, entry.min[2]],
    distance,
    yaw,
    pitch,
    fovY: 1,
    aspect: 1,
    near: 1,
  });
  let squared = 0;
  for (const axis of [0, 1, 2]) {
    const nearest = Math.min(entry.max[axis], Math.max(entry.min[axis], _pose_eye[axis]));
    squared += (_pose_eye[axis] - nearest) ** 2;
  }
  return Math.sqrt(squared);
}

/**
 * The camera on the middle of `entry`'s footprint, looking from `yaw` and
 * `pitch`, with its eye `range` metres from the nearest point of the
 * building. The renderer chooses a tier by that distance (to the building's
 * chunk), not by the orbit distance, so this is the pose a tier's boundary
 * is judged from.
 */
export function poseAtRange(
  entry: LineupEntry,
  range: number,
  yaw: number,
  pitch: number,
): CameraPose {
  // The range grows with the orbit distance and is never more than the box's
  // reach short of it.
  const reach = Math.hypot(
    entry.max[0] - entry.min[0],
    entry.max[1] - entry.min[1],
    entry.max[2] - entry.min[2],
  );
  let [near, far] = [0, range + reach];
  for (let step = 0; step < 60; step++) {
    const middle = (near + far) / 2;
    if (rangeFrom(entry, middle, yaw, pitch) < range) near = middle;
    else far = middle;
  }
  return {
    target: [(entry.min[0] + entry.max[0]) / 2, (entry.min[1] + entry.max[1]) / 2],
    distance: (near + far) / 2,
    yaw,
    pitch,
  };
}

const _fit_viewProj = createGpuMat4();
const _fit_point = createProjectedPoint();
const _fit_corner = vec3.create();

/**
 * The nearest camera on the middle of the box `min`..`max`, looking from
 * `yaw` and `pitch` through `lens`, that holds every corner of the box
 * within `fill` of the picture's half width and half height.
 */
export function cameraToFit(
  min: readonly number[],
  max: readonly number[],
  yaw: number,
  pitch: number,
  lens: Pick<Camera3DParams, "fovY" | "aspect" | "near">,
  fill: number,
): Camera3DParams {
  const camera: Camera3DParams = {
    ...lens,
    target: [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2],
    distance: 1,
    yaw,
    pitch,
  };
  /** How far the box reaches across the picture from `distance`; past 1 it
   *  leaves it, and a corner behind the eye is as far out as can be. */
  const reach = (distance: number) => {
    camera.distance = distance;
    viewProjMatrix(_fit_viewProj, camera);
    let most = 0;
    for (let k = 0; k < 8; k++) {
      vec3.set(
        _fit_corner,
        k & 1 ? max[0] : min[0],
        k & 2 ? max[1] : min[1],
        k & 4 ? max[2] : min[2],
      );
      const { ndc, clipW } = projectPoint(_fit_point, _fit_viewProj, _fit_corner);
      if (clipW <= 0) return Infinity;
      most = Math.max(most, Math.abs(ndc[0]), Math.abs(ndc[1]));
    }
    return most;
  };
  // The box shrinks in the picture as the camera backs off: double the
  // distance until it fits, then close in on where it just does.
  let far = Math.hypot(max[0] - min[0], max[1] - min[1], max[2] - min[2]) + lens.near;
  while (reach(far) > fill) far *= 2;
  let near = 0;
  for (let step = 0; step < 50; step++) {
    const middle = (near + far) / 2;
    if (reach(middle) > fill) near = middle;
    else far = middle;
  }
  camera.distance = far;
  return camera;
}
