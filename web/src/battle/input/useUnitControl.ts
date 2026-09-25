/** The one player command path: selection, right-click moves (with the
 * double-click fast upgrade and Shift queueing), Stop, and the acknowledgement
 * log. Labs and the battle route share it; it sends only real commands. */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { SimClient } from "../sim/client";
import type { ObservationView, OwnUnitView } from "../sim/observation";
import type { CommandAck, Order } from "../sim/protocol";
import { MoveGestures } from "./moveGestures";

export interface PointerPick {
  /** The own unit under the pointer, if any. */
  unit: number | null;
  button: "left" | "right";
  shift: boolean;
  x: number;
  y: number;
  time: number;
  /** Ground point under the pointer on the known surface, if any. */
  ground: [number, number] | null;
}

export interface AckEntry {
  seq: number;
  label: string;
  ack: CommandAck;
}

const LOG_LENGTH = 8;

export function useUnitControl(client: SimClient | null, observation: ObservationView | null) {
  const [selected, setSelected] = useState<number[]>([]);
  const [acks, setAcks] = useState<AckEntry[]>([]);
  const gestures = useRef(new MoveGestures());
  const observationRef = useRef(observation);
  observationRef.current = observation;

  // A new client (reset) starts with no selection and an empty log.
  useEffect(() => {
    setSelected([]);
    setAcks([]);
    gestures.current = new MoveGestures();
  }, [client]);

  const unitName = useCallback((id: number) => {
    const unit = observationRef.current?.own.find((u) => u.id === id);
    return unit ? `${unit.kind} #${id}` : `unit #${id}`;
  }, []);

  const describe = useCallback(
    (order: Order, queued: boolean) => {
      switch (order.kind) {
        case "move":
          return `${order.route === "fastest" ? "fast move" : "move"} ${order.units.map(unitName).join(", ")} to (${order.goal
            .map((v) => v.toFixed(0))
            .join(", ")})${queued ? " (queued)" : ""}`;
        case "stop":
          return `stop ${order.units.map(unitName).join(", ")}`;
        case "upgrade_move":
          return `upgrade gesture ${order.gesture} to fast route`;
      }
    },
    [unitName],
  );

  const issue = useCallback(
    async (order: Order, queued = false) => {
      if (!client) return null;
      const label = describe(order, queued);
      const ack = await client.command(order, queued);
      setAcks((log) => [{ seq: ack.seq, label, ack }, ...log].slice(0, LOG_LENGTH));
      return ack;
    },
    [client, describe],
  );

  const onPointer = useCallback(
    (pick: PointerPick) => {
      if (pick.button === "left") {
        const unit = pick.unit;
        setSelected((current) =>
          unit === null
            ? pick.shift
              ? current
              : []
            : pick.shift
              ? current.includes(unit)
                ? current.filter((u) => u !== unit)
                : [...current, unit]
              : [unit],
        );
        return;
      }
      if (!pick.ground || selected.length === 0) return;
      const order = gestures.current.rightClick(pick, selected, pick.ground);
      void issue(order, order.kind === "move" && pick.shift);
    },
    [selected, issue],
  );

  /** Select own units whose screen position falls in a dragged rectangle. */
  const selectInRect = useCallback((inRect: (unit: OwnUnitView) => boolean, additive: boolean) => {
    const hits = (observationRef.current?.own ?? []).filter(inRect).map((u) => u.id);
    setSelected((current) => (additive ? [...new Set([...current, ...hits])] : hits));
  }, []);

  const stop = useCallback(() => {
    if (selected.length) void issue({ kind: "stop", units: selected });
  }, [selected, issue]);

  // S stops the selection; never while typing in a control.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (e.key.toLowerCase() === "s" && !e.metaKey && !e.ctrlKey) stop();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [stop]);

  const selectedUnits = useMemo(
    () => (observation?.own ?? []).filter((u) => selected.includes(u.id)),
    [observation, selected],
  );

  return {
    selected,
    setSelected,
    selectedUnits,
    acks,
    issue,
    onPointer,
    selectInRect,
    stop,
    unitName,
  };
}
