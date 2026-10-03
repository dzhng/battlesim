/** The one player command path: selection (double-click selects similar:
 * `selectSimilar.ts`), right-click moves (with the
 * double-click fast upgrade and Shift queueing), right-click on an identified
 * enemy or a contact's area to attack it, armed attack-move, reverse-move and attack-ground
 * (Ctrl+right-click attack-moves at once; a right-click behind a single
 * selected vehicle reverses), right-click on a building to garrison it (Shift
 * queues), leaving buildings, stop, the fire-policy toggle, deploy/pack, and
 * the acknowledgement log. Keys come from `CommandBindings`. Building clicks
 * send the complete selection for authoritative entry and outside gathering;
 * deploy and leaving a building reach only capable units (`commandReach.ts`). Labs and the
 * battle route share it; it sends only real commands. */
import { useCallback, useEffect, useMemo, useRef, useState, type SetStateAction } from "react";
import type { SimClient } from "../sim/client";
import type { ObservationView, OwnUnitView } from "../sim/observation";
import type { CommandAck, Order } from "../sim/protocol";
import { commandForKey, ShowOrdersBinding } from "./commandBindings";
import { useHeldKey } from "./heldKeys";
import { MoveGestures } from "./moveGestures";
import { pointerIntent, type PointerPick, type CommandMode } from "./pointerIntent";
import { reach } from "./commandReach";
import { SelectClicks, similarUnits } from "./selectSimilar";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";

export interface AckEntry {
  seq: number;
  label: string;
  order: Order;
  ack: CommandAck;
}

const LOG_LENGTH = 8;

export function useUnitControl(
  client: SimClient | null,
  observation: ObservationView | null,
  /** Hears every order as it is sent, queued or not (the order flash). */
  onIssue?: (order: Order, queued: boolean) => void,
) {
  const onIssueRef = useRef(onIssue);
  onIssueRef.current = onIssue;
  const [selected, setSelection] = useState<number[]>([]);
  const selectedUnits = useMemo(
    () => selected.flatMap((id) => observation?.own.find((u) => u.id === id) ?? []),
    [observation, selected],
  );
  // Reconcile before committing handlers: a casualty cannot poison the survivors' orders.
  if (selectedUnits.length !== selected.length) setSelection(selectedUnits.map((u) => u.id));
  const [acks, setAcks] = useState<AckEntry[]>([]);
  const [mode, setModeState] = useState<CommandMode>("move");
  // The armed mode as last set, ahead of the render: a key pressed right
  // after another reads what the first one did.
  const modeRef = useRef<CommandMode>("move");
  const setMode = useCallback((next: CommandMode) => {
    modeRef.current = next;
    setModeState(next);
  }, []);
  const selectedUnitsRef = useRef(selectedUnits);
  selectedUnitsRef.current = selectedUnits;
  const gestures = useRef(new MoveGestures());
  const clicks = useRef(new SelectClicks());
  const setSelected = useCallback((next: SetStateAction<number[]>) => {
    clicks.current.reset();
    setSelection(next);
  }, []);
  const observationRef = useRef(observation);
  observationRef.current = observation;
  /** Space held (D2+): the order overlay shows every own unit. */
  const showOrders = useHeldKey(ShowOrdersBinding.code);

  // A new client (reset) starts with no selection and an empty log.
  useEffect(() => {
    setSelection([]);
    setAcks([]);
    setMode("move");
    gestures.current = new MoveGestures();
    clicks.current = new SelectClicks();
  }, [client, setMode]);

  // An armed command applies to the selection it was armed for.
  useEffect(() => {
    if (selected.length === 0) setMode("move");
  }, [selected, setMode]);

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
        case "occupy_building":
          return `occupy building ${order.building} with ${order.units.map(unitName).join(", ")}${queued ? " (queued)" : ""}`;
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
      onIssueRef.current?.(order, queued);
      const label = describe(order, queued);
      const ack = await client.command(order, queued);
      setAcks((log) => [{ seq: ack.seq, label, order, ack }, ...log].slice(0, LOG_LENGTH));
      return ack;
    },
    [client, describe],
  );

  const onPointer = useCallback(
    (pick: PointerPick) => {
      if (pick.button === "left") {
        const unit = pick.unit;
        const own = observationRef.current?.own ?? [];
        const kind = own.find((u) => u.id === unit)?.kind ?? null;
        const similar = clicks.current.click({ ...pick, unit: kind ? unit : null, kind });
        if (similar && kind) {
          // Select similar: every own unit of its type, or of its role
          // (Shift adds them to the selection).
          const hits = similarUnits(own, kind, similar, UNITS);
          setSelection((current) => (pick.shift ? [...new Set([...current, ...hits])] : hits));
          return;
        }
        setSelection((current) =>
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
      const intent = pointerIntent(pick, selectedUnitsRef.current, modeRef.current, UNITS);
      if (intent.kind === "none") return;
      if (intent.kind === "blocked") {
        if (intent.disarm) setMode("move");
        return;
      }
      setMode("move");
      const { queued, ...command } = intent;
      if (command.kind === "move") {
        const order =
          command.route === "fastest"
            ? { ...command, gesture: gestures.current.token() }
            : gestures.current.rightClick(
                pick,
                command.units,
                command.goal,
                command.direction,
                command.facing,
              );
        void issue(order, order.kind === "move" && queued);
      } else if (command.kind === "attack") {
        void issue(command, queued);
      } else {
        void issue({ ...command, gesture: gestures.current.token() }, queued);
      }
    },
    [issue, setMode],
  );

  const intentAt = useCallback(
    (pick: PointerPick) => pointerIntent(pick, selectedUnitsRef.current, modeRef.current, UNITS),
    [],
  );

  /** Return fire only for the selection, or Fire at will if all hold. */
  const togglePolicy = useCallback(() => {
    const units = selectedUnitsRef.current;
    if (!units.length) return;
    const hold = units.every((u) => u.engagement === "return_fire_only");
    void issue({
      kind: "set_engagement",
      units: selected,
      policy: hold ? "fire_at_will" : "return_fire_only",
    });
  }, [selected, issue]);

  /** Select own units whose screen position falls in a dragged rectangle. */
  const selectInRect = useCallback(
    (inRect: (unit: OwnUnitView) => boolean, additive: boolean) => {
      const hits = (observationRef.current?.own ?? []).filter(inRect).map((u) => u.id);
      setSelected((current) => (additive ? [...new Set([...current, ...hits])] : hits));
    },
    [setSelected],
  );

  const stop = useCallback(() => {
    if (selected.length) void issue({ kind: "stop", units: selected });
  }, [selected, issue]);

  /** Deploy (set up in place) or pack the selection's units that deploy. */
  const setDeployment = useCallback(
    (deployed: boolean) => {
      const units = reach("deploy", selectedUnitsRef.current, UNITS).map((u) => u.id);
      if (units.length) void issue({ kind: "set_deployment", units, deployed });
    },
    [issue],
  );

  /** Deploy the selection's units that deploy, or pack them if all are
   *  already deployed or deploying. */
  const toggleDeployment = useCallback(() => {
    const units = reach("deploy", selectedUnitsRef.current, UNITS);
    if (!units.length) return;
    const deployed = units.every((u) => u.deployment?.target === "deployed");
    void issue({ kind: "set_deployment", units: units.map((u) => u.id), deployed: !deployed });
  }, [issue]);

  /** The selection's units inside a building leave it. */
  const exitBuilding = useCallback(() => {
    const units = reach("exit_building", selectedUnitsRef.current, UNITS).map((u) => u.id);
    if (units.length) void issue({ kind: "exit_building", units });
  }, [issue]);

  // Command keys, from the one binding table.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const command = commandForKey(e);
      // Esc is the unit control's only while a command is armed; otherwise
      // it goes on to the page (the battle's pause menu).
      if (!command || (command === "disarm" && modeRef.current === "move")) return;
      e.preventDefault();
      const any = selectedUnitsRef.current.length > 0;
      const armed = reach("attack", selectedUnitsRef.current, UNITS).length > 0;
      if (command === "stop") stop();
      else if (command === "toggle_fire_policy") togglePolicy();
      else if (command === "toggle_deployment") toggleDeployment();
      else if (command === "attack_move" && armed) setMode("attack_move");
      else if (command === "reverse_move" && any) setMode("reverse_move");
      else if (command === "attack_ground" && armed) setMode("attack_ground");
      else if (command === "disarm") setMode("move");
    };
    // Capture: ahead of the page's own Esc (the pause menu), whatever the
    // listeners' order.
    window.addEventListener("keydown", onKey, { capture: true });
    return () => window.removeEventListener("keydown", onKey, { capture: true });
  }, [stop, togglePolicy, toggleDeployment, setMode]);

  return {
    selected,
    setSelected,
    selectedUnits,
    acks,
    issue,
    onPointer,
    intentAt,
    selectInRect,
    stop,
    setDeployment,
    toggleDeployment,
    exitBuilding,
    togglePolicy,
    mode,
    setMode,
    unitName,
    showOrders,
  };
}
