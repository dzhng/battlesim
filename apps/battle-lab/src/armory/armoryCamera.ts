// The Armory's camera: an orbit round the one unit on show, on the battle's
// rig (`CameraController`) with the Armory's own config
// (`presentation.camera_armory`). Left-drag orbits, the wheel zooms between
// framings sized to the unit's bounds, and the pitch may dip below the
// horizon to show the underside against the sky. There is no edge pan and no
// key, and input comes from the stage's canvas alone. The eye keeps clear of
// the unit, the yard's props and the ground through renderer-core's
// clearance.
//
// The stage draws the whole screen behind the glass panels, and the unit
// stands in the centre cell between the list and the sheet, so the drawn
// target slides sideways until the orbit's centre falls in that cell.
import { useEffect, type RefObject } from "react";
import { clamp } from "math";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import {
  createClearanceState,
  type ClearanceState,
} from "@packages/renderer-core/src/cameraClearance";
import {
  CameraController,
  type CameraPresentation,
} from "@packages/renderer-core/src/cameraController";
import {
  createCameraObstacles,
  type CameraObstacles,
  type ObstacleBox,
} from "@packages/renderer-core/src/cameraObstacles";
import type { Bounds } from "@packages/scene-assets/src/schema";
import type { PresentedCamera } from "@web/battle/sim/presentation";
import { presentationSettings } from "@web/presentationSettings";
import { presentedCamera } from "../presentedFrame";
import type { CameraPose } from "./yard";

/** `presentation.camera_armory`. */
const ARMORY = presentationSettings.camera_armory as {
  fov_y: number;
  near_m: number;
  pitch_limits: [number, number];
  zoom_speed: number;
  orbit_speed: number;
  /** The fraction of the cell's narrower side the unit's bounding sphere
   *  spans at the opening, the nearest and the farthest zoom. */
  framing: { open: number; near: number; far: number };
  clearance: CameraPresentation["clearance"];
};

/** A box on the page, in CSS pixels. */
export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** What the camera frames: the unit's bounds at rest, the angle its spot
 *  looks from, and what the eye keeps clear of: boxes, and the ground's
 *  height under it. Without bounds (the yard alone) the camera holds the
 *  spot's own pose. */
export interface Framing {
  bounds: Bounds | null;
  pose: CameraPose;
  boxes: readonly ObstacleBox[];
  ground: (x: number, y: number) => number;
}

/** The orbit distance at which a box of half extents `half` (axis-aligned,
 *  about the orbit's target), seen from `yaw` and `pitch`, spans at most
 *  `fill` of `cell` across and up, drawn into `view` through a lens of
 *  `fovY`. Each corner needs the eye far enough back that its offset across
 *  and up fits from its own depth along the view; the farthest of them wins. */
function fitDistance(
  half: readonly number[],
  yaw: number,
  pitch: number,
  fill: number,
  fovY: number,
  view: Box,
  cell: Box,
) {
  const [cy, sy, cp, sp] = [Math.cos(yaw), Math.sin(yaw), Math.cos(pitch), Math.sin(pitch)];
  // Toward the eye, the view's right, and its up.
  const axes = [
    [cp * cy, cp * sy, sp],
    [-sy, cy, 0],
    [-sp * cy, -sp * sy, cp],
  ];
  const tan = Math.tan(fovY / 2) / Math.max(1, view.height);
  const [across, up] = [fill * tan * cell.width, fill * tan * cell.height];
  let distance = 0;
  for (const x of [-half[0], half[0]])
    for (const y of [-half[1], half[1]])
      for (const z of [-half[2], half[2]]) {
        const [toward, right, rise] = axes.map((a) => a[0] * x + a[1] * y + a[2] * z);
        distance = Math.max(
          distance,
          toward + Math.max(Math.abs(right) / across, Math.abs(rise) / up),
        );
      }
  return distance;
}

/** `orbit` with its target where the middle of `cell` falls in `view`. */
export function framedOrbit(orbit: Camera3DParams, view: Box, cell: Box): Camera3DParams {
  if (!(view.height > 0)) return orbit;
  // Metres per pixel at the target's distance.
  const perPx = (2 * orbit.distance * Math.tan(orbit.fovY / 2)) / view.height;
  const right = (cell.left + cell.width / 2 - (view.left + view.width / 2)) * perPx;
  const down = (cell.top + cell.height / 2 - (view.top + view.height / 2)) * perPx;
  // The view's right on the ground (+Y at yaw 0: the eye looks down -X) and
  // its up, tilted back by the pitch.
  const [rx, ry] = [-Math.sin(orbit.yaw), Math.cos(orbit.yaw)];
  const s = Math.sin(orbit.pitch);
  const up = [-s * Math.cos(orbit.yaw), -s * Math.sin(orbit.yaw), Math.cos(orbit.pitch)];
  const [x, y, z] = orbit.target;
  // The target moves against the cell's offset, so the unit lands in it.
  return {
    ...orbit,
    target: [x - rx * right + up[0] * down, y - ry * right + up[1] * down, z + up[2] * down],
  };
}

/** The Armory's orbit, framing, input and clearance for one stage. */
export class ArmoryCamera {
  private framing: Framing | null = null;
  private obstacles: CameraObstacles = createCameraObstacles([], () => 0);
  private clearance: ClearanceState = createClearanceState();
  /** The orbit asked for, round the unit's centre; null until the first
   *  frame after a new framing, which opens it. */
  private orbit: Camera3DParams | null = null;
  private drawnOrbit: Camera3DParams | null = null;
  private wheel = 0;
  private drag: [number, number] = [0, 0];

  /** Frame `framing` afresh: the next frame opens on it. */
  frame(framing: Framing) {
    this.framing = framing;
    this.obstacles = createCameraObstacles(framing.boxes, framing.ground);
    this.clearance = createClearanceState();
    this.orbit = null;
    [this.wheel, this.drag] = [0, [0, 0]];
  }

  /** A drag of `[right, down]` viewport heights. */
  dragBy(right: number, down: number) {
    this.drag = [this.drag[0] + right, this.drag[1] + down];
  }

  /** Wheel `deltaY` pixels: positive zooms out. */
  wheelBy(deltaY: number) {
    this.wheel += deltaY;
  }

  /** The camera to draw `dt` seconds after the last, into `view` (the whole
   *  canvas) of `width` × `height` device pixels with the unit in `cell`;
   *  null before a framing. */
  presented(view: Box, cell: Box, width: number, height: number, dt: number) {
    const framing = this.framing;
    if (!framing) return null;
    const lens = { fovY: ARMORY.fov_y, aspect: width / height, near: ARMORY.near_m };
    const camera = framing.bounds
      ? this.orbitFor(framing.bounds, framing.pose, lens, view, cell, dt)
      : { ...lens, ...framing.pose, aspect: width / height };
    this.drawnOrbit = camera;
    return presentedCamera({ camera3d: camera, width, height }) as PresentedCamera;
  }

  /** The orbit drawn last; null before the first frame. */
  drawn(): Camera3DParams | null {
    return this.drawnOrbit;
  }

  /** What it frames now; null before a framing. */
  framed(): Framing | null {
    return this.framing;
  }

  /** The rig for `orbit` round a box of half extents `half`, in this view:
   *  the wheel reaches between the nearest and the farthest framing from
   *  where it looks now. */
  private rig(orbit: Camera3DParams, half: readonly number[], view: Box, cell: Box) {
    const fit = (fill: number) =>
      fitDistance(half, orbit.yaw, orbit.pitch, fill, ARMORY.fov_y, view, cell);
    const [near, far] = [fit(ARMORY.framing.near), fit(ARMORY.framing.far)];
    const config: CameraPresentation = {
      fov_y: ARMORY.fov_y,
      near_m: ARMORY.near_m,
      zoom_min: near,
      zoom_max: far,
      // Flat: the wheel never carries the pitch along.
      pitch_curve: [
        [near, 0],
        [far, 0],
      ],
      pan_speed: 0,
      rotate_speed: 0,
      zoom_speed: ARMORY.zoom_speed,
      orbit_speed: ARMORY.orbit_speed,
      pitch_limits: ARMORY.pitch_limits,
      clearance: ARMORY.clearance,
    };
    return { controller: new CameraController(config), near, far };
  }

  private orbitFor(
    bounds: Bounds,
    pose: CameraPose,
    lens: Pick<Camera3DParams, "fovY" | "aspect" | "near">,
    view: Box,
    cell: Box,
    dt: number,
  ): Camera3DParams {
    const center: [number, number, number] = [0, 1, 2].map(
      (i) => (bounds.min[i] + bounds.max[i]) / 2,
    ) as [number, number, number];
    const half = [0, 1, 2].map((i) => (bounds.max[i] - bounds.min[i]) / 2);
    const [low, high] = ARMORY.pitch_limits;
    let from = this.orbit;
    if (!from) {
      const angle = { yaw: pose.yaw, pitch: clamp(pose.pitch, low, high) };
      const { framing } = ARMORY;
      const open = fitDistance(half, angle.yaw, angle.pitch, framing.open, lens.fovY, view, cell);
      from = { ...lens, ...angle, target: center, distance: open };
    }
    const { controller } = this.rig(from, half, view, cell);
    const stepped = controller.step(
      { ...from, ...lens },
      { wheel: this.wheel, drag: this.drag },
      dt,
    );
    [this.wheel, this.drag] = [0, [0, 0]];
    // From where it looks now, and in this view, the orbit stays between the
    // nearest and farthest framings.
    const { near, far } = this.rig(stepped, half, view, cell);
    this.orbit = { ...stepped, distance: clamp(stepped.distance, near, far) };
    return controller.resolve(
      this.clearance,
      framedOrbit(this.orbit, view, cell),
      dt,
      this.obstacles,
    );
  }
}

/** The obstacle box round `bounds`, axis-aligned. */
export function boundsBox(bounds: Bounds): ObstacleBox {
  const [x0, y0, z0] = bounds.min;
  const [x1, y1, z1] = bounds.max;
  return {
    center: [(x0 + x1) / 2, (y0 + y1) / 2],
    yaw: 0,
    half: [(x1 - x0) / 2, (y1 - y0) / 2, (z1 - z0) / 2],
    baseZ: z0,
  };
}

/** Left-drag and wheel on `canvas` steer `camera`; nothing else does. A drag
 *  begun anywhere else (the list, the sheet) never reaches it. */
export function useArmoryCamera(canvas: RefObject<HTMLCanvasElement | null>, camera: ArmoryCamera) {
  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    let last: [number, number] | null = null;
    const down = (e: PointerEvent) => {
      if (e.button !== 0) return;
      last = [e.clientX, e.clientY];
      el.setPointerCapture(e.pointerId);
      el.dataset.dragging = "";
    };
    const move = (e: PointerEvent) => {
      if (!last) return;
      const h = Math.max(1, el.clientHeight);
      camera.dragBy((e.clientX - last[0]) / h, (e.clientY - last[1]) / h);
      last = [e.clientX, e.clientY];
    };
    const up = () => {
      last = null;
      delete el.dataset.dragging;
    };
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      camera.wheelBy(e.deltaY);
    };
    el.addEventListener("pointerdown", down);
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
    el.addEventListener("wheel", wheel, { passive: false });
    return () => {
      el.removeEventListener("pointerdown", down);
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
      el.removeEventListener("wheel", wheel);
    };
  }, [canvas, camera]);
}
