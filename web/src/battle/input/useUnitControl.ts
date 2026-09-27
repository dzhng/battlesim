/** The one player command path: selection, right-click moves (with the
 * double-click fast upgrade and Shift queueing), right-click on an identified
 * enemy to attack it, armed attack-move, reverse-move and attack-ground
 * (Ctrl+right-click attack-moves at once; a right-click behind a single
 * selected vehicle reverses), right-click on a building to garrison it (Shift
 * queues), leaving buildings, stop, the fire-policy toggle, deploy/pack, and
 * the acknowledgement log. Keys come from `CommandBindings`. Labs and the
 * battle route share it; it sends only real commands. */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { SimClient } from "../sim/client";
import type { ObservationView, OwnUnitView } from "../sim/observation";
import type { CommandAck, Order } from "../sim/protocol";
import { commandForKey, isAttackMoveClick } from "./commandBindings";
import { MoveGestures } from "./moveGestures";
import { inReverseZone } from "./reverseZone";

export interface PointerPick {
  /** The own unit under the pointer, if any. */
  unit: number | null;
  button: "left" | "right";
  shift: boolean;
  ctrl: boolean;
  x: number;
  y: number;
  time: number;
  /** Ground point under the pointer on the known surface, if any. */
  ground: [number, number] | null;
  /** The building (static prop id) under the pointer, if any. */
  building?: number | null;
  /** The identified enemy (observed handle) under the pointer, if any. */
  enemy?: number | null;
}

/** What the next right-click does: move, or an armed command from the bar or
 *  keys (attack-move, reverse move, attack ground; fast move and garrison
 *  from the bar). */
export type CommandMode =
  | "move"
  | "attack_move"
  | "reverse_move"
  | "attack_ground"
  | "fast_move"
  | "garrison";

export interface AckEntry {
  seq: number;
  label: string;
  ack: CommandAck;
}

const LOG_LENGTH = 8;

export function useUnitControl(client: SimClient | null, observation: ObservationView | null) {
  const [selected, setSelected] = useState<number[]>([]);
  const [acks, setAcks] = useState<AckEntry[]>([]);
  const [mode, setMode] = useState<CommandMode>("move");
  const selectedRef = useRef(selected);
  selectedRef.current = selected;
  const gestures = useRef(new MoveGestures());
  const observationRef = useRef(observation);
  observationRef.current = observation;

  // A new client (reset) starts with no selection and an empty log.
  useEffect(() => {
    setSelected([]);
    setAcks([]);
    setMode("move");
    gestures.current = new MoveGestures();
  }, [client]);

  // An armed command applies to the selection it was armed for.
  useEffect(() => {
    if (selected.length === 0) setMode("move");
  }, [selected]);

  const unitName = useCallback((id: number) => {
    const unit = observationRef.current?.own.find((u) => u.id === id);
    return unit ? `${unit.kind} #${id}` : `unit #${id}`;
  }, []);

  const describe = useCallback(
    (order: Order, queued: boolean) => {
      switch (order.kind) {
        case "move":
          return `${order.route === "fastest" ? "fast " : ""}${order.direction === "reverse" ? "reverse move" : "move"} ${order.units.map(unitName).join(", ")} to (${order.goal
            .map((v) => v.toFixed(0))
            .join(", ")})${queued ? " (queued)" : ""}`;
        case "stop":
          return `stop ${order.units.map(unitName).join(", ")}`;
        case "attack": {
          const t = order.target;
          const what =
            t.kind === "ground"
              ? `ground (${t.point[0].toFixed(0)}, ${t.point[1].toFixed(0)})`
              : `${t.kind === "contact" ? "area" : "enemy"} ${t.id}`;
          return `attack ${what} with ${order.units.map(unitName).join(", ")}`;
        }
        case "attack_move":
          return `attack-move ${order.units.map(unitName).join(", ")} to (${order.goal
            .map((v) => v.toFixed(0))
            .join(", ")})`;
        case "set_engagement":
          return `${order.policy === "fire_at_will" ? "fire at will" : "return fire only"}: ${order.units.map(unitName).join(", ")}`;
        case "set_deployment":
          return `${order.deployed ? "deploy" : "pack"} ${order.units.map(unitName).join(", ")}`;
        case "garrison":
          return `garrison building ${order.building} with ${order.units.map(unitName).join(", ")}${queued ? " (queued)" : ""}`;
        case "exit_building":
          return `leave building: ${order.units.map(unitName).join(", ")}`;
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
      if (selected.length === 0) return;
      // Ctrl+right-click: attack-move to the ground there, whatever is armed.
      if (isAttackMoveClick(pick)) {
        if (!pick.ground) return;
        setMode("move");
        void issue(
          {
            kind: "attack_move",
            units: selected,
            gesture: gestures.current.token(),
            goal: pick.ground,
          },
          pick.shift,
        );
        return;
      }
      // Right-click an identified enemy: attack it (Shift queues).
      if (pick.enemy != null) {
        setMode("move");
        void issue(
          { kind: "attack", units: selected, target: { kind: "identified", id: pick.enemy } },
          pick.shift,
        );
        return;
      }
      if (pick.building != null && (mode === "move" || mode === "garrison")) {
        setMode("move");
        void issue({ kind: "garrison", units: selected, building: pick.building }, pick.shift);
        return;
      }
      if (!pick.ground || mode === "garrison") return;
      if (mode === "reverse_move") {
        setMode("move");
        const order = gestures.current.rightClick(pick, selected, pick.ground, "reverse");
        void issue(order, order.kind === "move" && pick.shift);
        return;
      }
      if (mode === "fast_move") {
        setMode("move");
        void issue(
          {
            kind: "move",
            units: selected,
            gesture: gestures.current.token(),
            goal: pick.ground,
            route: "fastest",
          },
          pick.shift,
        );
        return;
      }
      if (mode !== "move") {
        // An armed attack-move or attack-ground applies to one click, then movement is the default again.
        const [x, y] = pick.ground;
        const order: Order =
          mode === "attack_move"
            ? {
                kind: "attack_move",
                units: selected,
                gesture: gestures.current.token(),
                goal: [x, y],
              }
            : { kind: "attack", units: selected, target: { kind: "ground", point: [x, y, 0] } };
        setMode("move");
        void issue(order, pick.shift);
        return;
      }
      // Behind a single selected vehicle, a plain right-click reverses (Q31).
      const own = (observationRef.current?.own ?? []).filter((u) => selected.includes(u.id));
      const direction = inReverseZone(own, pick.ground) ? "reverse" : "forward";
      const order = gestures.current.rightClick(pick, selected, pick.ground, direction);
      void issue(order, order.kind === "move" && pick.shift);
    },
    [selected, issue, mode],
  );

  /** Return fire only for the selection, or Fire at will if all hold. */
  const togglePolicy = useCallback(() => {
    const units = (observationRef.current?.own ?? []).filter((u) => selected.includes(u.id));
    if (!units.length) return;
    const hold = units.every((u) => u.engagement === "return_fire_only");
    void issue({
      kind: "set_engagement",
      units: selected,
      policy: hold ? "fire_at_will" : "return_fire_only",
    });
  }, [selected, issue]);

  /** Select own units whose screen position falls in a dragged rectangle. */
  const selectInRect = useCallback((inRect: (unit: OwnUnitView) => boolean, additive: boolean) => {
    const hits = (observationRef.current?.own ?? []).filter(inRect).map((u) => u.id);
    setSelected((current) => (additive ? [...new Set([...current, ...hits])] : hits));
  }, []);

  const stop = useCallback(() => {
    if (selected.length) void issue({ kind: "stop", units: selected });
  }, [selected, issue]);

  /** Deploy (set up in place) or pack the selection. */
  const setDeployment = useCallback(
    (deployed: boolean) => {
      if (selected.length) void issue({ kind: "set_deployment", units: selected, deployed });
    },
    [selected, issue],
  );

  /** Deploy the selection's deploying units, or pack them if all are already
   *  deployed or deploying. */
  const toggleDeployment = useCallback(() => {
    const units = (observationRef.current?.own ?? []).filter(
      (u) => selected.includes(u.id) && u.deployment,
    );
    if (!units.length) return;
    const deployed = units.every((u) => u.deployment!.target === "deployed");
    void issue({ kind: "set_deployment", units: units.map((u) => u.id), deployed: !deployed });
  }, [selected, issue]);

  /** The selection leaves its buildings. */
  const exitBuilding = useCallback(() => {
    if (selected.length) void issue({ kind: "exit_building", units: selected });
  }, [selected, issue]);

  // Command keys, from the one binding table.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const command = commandForKey(e);
      if (!command) return;
      e.preventDefault();
      const any = selectedRef.current.length > 0;
      if (command === "stop") stop();
      else if (command === "toggle_fire_policy") togglePolicy();
      else if (command === "toggle_deployment") toggleDeployment();
      else if (command === "attack_move" && any) setMode("attack_move");
      else if (command === "reverse_move" && any) setMode("reverse_move");
      else if (command === "attack_ground" && any) setMode("attack_ground");
      else if (command === "disarm") setMode("move");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [stop, togglePolicy, toggleDeployment]);

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
    setDeployment,
    toggleDeployment,
    exitBuilding,
    togglePolicy,
    mode,
    setMode,
    unitName,
  };
}
