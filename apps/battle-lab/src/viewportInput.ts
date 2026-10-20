import type { WorldRay } from "@packages/renderer-core/src/camera3d";

/** One snapshot shared by hover, the ruler and the held facing preview. */
export interface ViewportPointer {
  position: { x: number; y: number } | null;
  ray: WorldRay | null;
  rightPress: WorldRay | null;
  /** Where a held left drag began, while it is an action (`leftDragAction`). */
  leftPress: WorldRay | null;
  ctrl: boolean;
  shift: boolean;
  cameraDragging: boolean;
  rightDragging: boolean;
}

export interface LabPick {
  instance: number;
  ray: WorldRay;
  button: "left" | "right";
  shift: boolean;
  ctrl: boolean;
  /** CSS pixel position and event time, for gesture recognition. */
  x: number;
  y: number;
  time: number;
  /** A right-drag's release ray (Q9 facing): the press is the pick, the
   *  release the point the units face. Absent for a plain right-click. */
  release?: WorldRay;
}

export interface LabBox {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  shift: boolean;
  project: (x: number, y: number, z: number) => [number, number] | null;
}
