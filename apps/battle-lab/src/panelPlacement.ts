import { useCallback, useRef, useState } from "react";
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
 *  frame. The panels open as Space opens them, or when `open` says so (a
 *  capture filming them open). The pointer hovers the unit under it, or
 *  the one whose army deck card it is over (`deckHover`); `onHover` hears the unit
 *  hovered, by its card, its model or its deck card, each frame. */
const panelPlacer =
  (
    readouts: { readonly current: ReadoutLayerHandle | null },
    frame: LastDrawnFrame,
    ground: (x: number, y: number) => number,
    control: { readonly showOrders: boolean },
    deckHover: { readonly current: number | null },
    onHover: (unit: number | null) => void,
  ) =>
  (project: Project, camera: Camera3DParams, pointer: ViewportPointer, open = control.showOrders) =>
    onHover(
      readouts.current?.place(
        project,
        camera,
        { own: frame.drawnAt.current, enemies: frame.drawnEnemyAt.current, ground },
        frame.drawnClock.current,
        open,
        // Over a deck card the pointer is on no card or model: it hovers
        // that card's unit.
        deckHover.current !== null
          ? { x: -Infinity, y: -Infinity, unit: deckHover.current, enemy: null }
          : pointer.position
            ? {
                ...pointer.position,
                ...pickedUnit(
                  frame.drawn.current,
                  pointer.ray ? pickBox(pointer.ray, frame.drawn.current.picks) : -1,
                ),
              }
            : null,
      ) ?? null,
    );

/** The info panels' placing (`panelPlacer`), and the hover it finds: the
 *  side's unit under the pointer, by its card, its model or its army deck
 *  card (`hoverFromDeck`, `ArmyDeck`'s `onHover`), whose marker shows on
 *  the ground (`hovered`). */
export function usePanels(
  readouts: { readonly current: ReadoutLayerHandle | null },
  frame: LastDrawnFrame,
  ground: (x: number, y: number) => number,
  control: { readonly showOrders: boolean },
) {
  const [hovered, setHovered] = useState<number | null>(null);
  const hoveredRef = useRef(hovered);
  const deckHover = useRef<number | null>(null);
  const hoverFromDeck = useCallback((unit: number | null) => {
    deckHover.current = unit;
  }, []);
  const placePanels = panelPlacer(readouts, frame, ground, control, deckHover, (unit) => {
    if (unit === hoveredRef.current) return;
    hoveredRef.current = unit;
    setHovered(unit);
  });
  return { placePanels, hovered, hoverFromDeck };
}
