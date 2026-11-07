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

export type GestureButton = "left" | "middle" | "right";

export interface GestureDown {
  button: GestureButton;
  x: number;
  y: number;
  shift: boolean;
  ray?: WorldRay;
  pick?: LabPick;
}

export interface GestureMove {
  x: number;
  y: number;
}

export interface GestureUp extends GestureMove {
  button: GestureButton;
  pick?: LabPick;
  release?: WorldRay;
}

export interface ViewportGestureOwnerOptions {
  leftDragAction: () => boolean;
  onPick?: (pick: LabPick) => void;
  onRightPress?: (pick: LabPick | null) => void;
  onBox?: (box: LabBox) => void;
  onBoxPreview?: (box: Omit<LabBox, "shift" | "project"> | null) => void;
  onOrbit?: (delta: [number, number]) => void;
  project: LabBox["project"];
}

interface LeftPress {
  x: number;
  y: number;
  shift: boolean;
  ray: WorldRay;
  dragged: boolean;
}

interface RightPress {
  x: number;
  y: number;
  pick: LabPick;
  ray: WorldRay;
}

interface Orbit {
  x: number;
  y: number;
}

/** Owns viewport gesture transitions while the browser adapter owns the DOM.
 * Each button has its own lane so a second press never erases another button's
 * captured semantic target. */
export class ViewportGestureOwner {
  private left: LeftPress | null = null;
  private right: RightPress | null = null;
  private orbit: Orbit | null = null;

  constructor(private readonly options: ViewportGestureOwnerOptions) {}

  down(event: GestureDown) {
    if (event.button === "left") {
      if (!event.ray) return;
      this.left = {
        x: event.x,
        y: event.y,
        shift: event.shift,
        ray: event.ray,
        dragged: false,
      };
      return;
    }
    if (event.button === "right") {
      if (!event.pick || !event.ray) return;
      this.right = { x: event.x, y: event.y, pick: event.pick, ray: event.ray };
      this.options.onRightPress?.(event.pick);
      return;
    }
    this.orbit = { x: event.x, y: event.y };
  }

  move(event: GestureMove) {
    if (this.left) {
      if (Math.hypot(event.x - this.left.x, event.y - this.left.y) > CLICK_SLOP_PX) {
        this.left.dragged = true;
        if (!this.options.leftDragAction()) {
          this.options.onBoxPreview?.(this.boxRect(this.left.x, this.left.y, event.x, event.y));
        }
      }
    }
    if (!this.orbit) return;
    const delta: [number, number] = [event.x - this.orbit.x, event.y - this.orbit.y];
    this.orbit.x = event.x;
    this.orbit.y = event.y;
    this.options.onOrbit?.(delta);
  }

  up(event: GestureUp) {
    // Keep the established capture policy: any pointerup ends an orbit, while
    // the left and right lanes finish only their own press records.
    this.orbit = null;
    if (event.button === "middle") {
      return;
    }
    if (event.button === "right") {
      const right = this.right;
      if (!right) return;
      this.right = null;
      const dragged = Math.hypot(event.x - right.x, event.y - right.y) > CLICK_SLOP_PX;
      this.options.onPick?.({
        ...right.pick,
        release: dragged ? event.release : undefined,
      });
      return;
    }
    const left = this.left;
    if (!left) return;
    this.left = null;
    this.options.onBoxPreview?.(null);
    const click =
      this.options.leftDragAction() ||
      Math.hypot(event.x - left.x, event.y - left.y) <= CLICK_SLOP_PX;
    if (click) {
      if (event.pick) this.options.onPick?.(event.pick);
      return;
    }
    this.options.onBox?.({
      ...this.boxRect(left.x, left.y, event.x, event.y),
      shift: left.shift,
      project: this.options.project,
    });
  }

  cancel() {
    this.left = null;
    this.right = null;
    this.orbit = null;
    this.options.onBoxPreview?.(null);
    this.options.onRightPress?.(null);
  }

  get active() {
    return !!(this.left || this.right || this.orbit);
  }

  get cameraDragging() {
    return !!this.orbit;
  }

  get leftPressActive() {
    return !!this.left;
  }

  get rightPressRay() {
    return this.right?.ray ?? null;
  }

  get leftPressRay() {
    return this.left?.dragged && this.options.leftDragAction() ? this.left.ray : null;
  }

  rightDraggingAt(point: { x: number; y: number }) {
    return !!(
      this.right && Math.hypot(point.x - this.right.x, point.y - this.right.y) > CLICK_SLOP_PX
    );
  }

  private boxRect(x0: number, y0: number, x1: number, y1: number) {
    return {
      x0: Math.min(x0, x1),
      y0: Math.min(y0, y1),
      x1: Math.max(x0, x1),
      y1: Math.max(y0, y1),
    };
  }
}

const CLICK_SLOP_PX = 5;
