import type { Vec3 } from "math";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { pickBox } from "@packages/battle-renderer/src/picking";
import type { Project, ReadoutLayerHandle } from "@web/battle/present/readouts";
import type { ViewportPointer } from "./LabViewport";
import { pickedUnit, type DrawnInstances } from "./sideInstances";

/** What the last drawn frame drew, as the session's refs hold it: which unit
 *  or enemy each pick box is, each unit's drawn (interpolated) position, and
 *  the frame's presentation clock. Read at call time, never captured. */
export interface LastDrawnFrame {
  readonly drawn: { readonly current: DrawnInstances };
  readonly drawnAt: { readonly current: Map<number, Readonly<Vec3>> };
  readonly drawnEnemyAt: { readonly current: Map<number, Readonly<Vec3>> };
  readonly drawnClock: { readonly current: number | null };
}

/** Places the info panels' layer (`ReadoutLayer`'s handle) off what the last
 *  frame drew, at its presentation clock; call the result once per animation
 *  frame. The panels open as Space opens them. */
export const panelPlacer =
  (
    readouts: { readonly current: ReadoutLayerHandle | null },
    frame: LastDrawnFrame,
    ground: (x: number, y: number) => number,
    control: { readonly showOrders: boolean },
  ) =>
  (project: Project, camera: Camera3DParams, pointer: ViewportPointer) =>
    readouts.current?.place(
      project,
      camera,
      { own: frame.drawnAt.current, enemies: frame.drawnEnemyAt.current, ground },
      frame.drawnClock.current,
      control.showOrders,
      pointer.position
        ? {
            ...pointer.position,
            ...pickedUnit(
              frame.drawn.current,
              pointer.ray ? pickBox(pointer.ray, frame.drawn.current.picks) : -1,
            ),
          }
        : null,
    );
