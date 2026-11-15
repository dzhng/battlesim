// The reinforcement in hand: the unit chosen from the shop, its ghost on the
// cursor, and the placements the authority accepted, from choice to confirm
// or cancel. A new battle drops all of it.
import { useCallback, useEffect, useRef, useState } from "react";
import { PurchasePlacementControl, type PurchaseGhost } from "@web/battle/input/purchasePlacement";
import type { SimClient } from "@web/battle/sim/client";
import type { WorldRay } from "@packages/renderer-core/src/camera3d";
import type { LabPick } from "./LabViewport";

/** Where the reinforcement in hand stands for the pointer, and which way it
 *  faces: at the point `under` finds beneath the pointer's ray, or, while a
 *  left drag is held, pinned where it was pressed and turned toward the
 *  pointer. */
export function placementPose(
  pointer: { ray: WorldRay | null; leftPress: WorldRay | null },
  under: (ray: WorldRay) => readonly number[] | null,
): { at: [number, number] | null; facing: number | undefined } {
  const ray = pointer.leftPress ?? pointer.ray;
  const at = ray && under(ray);
  const to = pointer.leftPress && pointer.ray ? under(pointer.ray) : null;
  return {
    at: at ? [at[0], at[1]] : null,
    facing: at && to ? Math.atan2(to[1] - at[1], to[0] - at[0]) : undefined,
  };
}

type Send = Parameters<PurchasePlacementControl["confirm"]>[0];

export function usePurchasePlacement(client: SimClient | null) {
  const [placement] = useState(() => new PurchasePlacementControl());
  /** The chosen reinforcement must be requested before the cursor asks the
   *  presentation to draw its ghost.  Keep this in React state so the art
   *  request advances on the same render as the picker choice. */
  const [placing, setPlacing] = useState<string | null>(null);
  const ghost = useRef<PurchaseGhost | null>(null);
  /** The ghosts the authority accepted, for their facing: a pending purchase
   *  carries only its kind and destination. */
  const placed = useRef<PurchaseGhost[]>([]);
  const cancel = useCallback(() => {
    placement.cancel();
    ghost.current = null;
    setPlacing(null);
  }, [placement]);
  const choose = useCallback(
    (variant: string) => {
      placement.choose(variant);
      setPlacing(variant);
    },
    [placement],
  );
  /** A click with a unit in hand: the left button places it (with Shift
   *  held, keeping it in hand for another), the right cancels. */
  const click = useCallback(
    (pick: LabPick, send: Send) => {
      if (pick.button === "right") return cancel();
      void placement.confirm(send, pick.shift).then((accepted) => {
        if (!accepted) return;
        placed.current.push(accepted);
        if (!placement.variant) {
          ghost.current = null;
          setPlacing(null);
        }
      });
    },
    [placement, cancel],
  );
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || !placement.variant) return;
      event.preventDefault();
      cancel();
    };
    window.addEventListener("keydown", onKey, { capture: true });
    return () => window.removeEventListener("keydown", onKey, { capture: true });
  }, [placement, cancel]);
  useEffect(() => {
    cancel();
    placed.current = [];
  }, [client, cancel]);
  return { placement, placing, ghost, placed, choose, click };
}
