/** The one player command path: selection (double-click selects similar:
 * `selectSimilar.ts`), right-click moves (with the
 * double-click fast upgrade and Shift queueing), right-click on an identified
 * enemy or a contact's area to attack it, armed attack-move and reverse-move.
 * Attack-ground confirms with left-click and cancels with right-click
 * (Ctrl+right-click attack-moves at once; a right-click behind a single
 * selected vehicle reverses), right-click on a building to garrison it (Shift
 * queues), leaving buildings, stop, the fire-policy toggle, deploy/pack, and
 * the acknowledgement log. Keys come from `CommandBindings`. Building clicks
 * send the complete selection for authoritative entry and outside gathering;
 * deploy and leaving a building reach only capable units (`commandReach.ts`). Labs and the
 * battle route share it; it sends only real commands. Its events (`UnitControlEvents`)
 * let the battle hear orders, their acknowledgements and the selection's growth
 * (the order flash, and the units' spoken answers). */
import { useCallback, useEffect, useMemo, useRef, useState, type SetStateAction } from "react";
import type { SimClient } from "../sim/client";
import { aboardCarrier, type ObservationView, type OwnUnitView } from "../sim/observationView";
import type { CommandAck, Order } from "../sim/protocol";
import { commandForKey, ShowOrdersBinding } from "./commandBindings";
import { useHeldKey } from "./heldKeys";
import { MoveGestures } from "./moveGestures";
import {
  CONFIRMED_BY_LEFT_CLICK,
  pointerIntent,
  reconcilePointerIntent,
  type PointerPick,
  type CommandMode,
  type PointerIntent,
} from "./pointerIntent";
import { reach } from "./commandReach";
import { SelectClicks, similarUnits } from "./selectSimilar";
import {
  afterOrder,
  NO_UNLOAD_POINTS,
  retire,
  unloadMarks,
  type UnloadPoints,
} from "./unloadPoints";
import { useSessionCatalog } from "../catalog/context";

export interface AckEntry {
  seq: number;
  label: string;
  order: Order;
  ack: CommandAck;
}

const LOG_LENGTH = 8;

/** What the player's commands tell the rest of the battle. */
export interface UnitControlEvents {
  /** Every order as it is sent, queued or not (the order flash). */
  onIssue?(order: Order, queued: boolean): void;
  /** Every order's acknowledgement from the current client: accepted, or refused and why. */
  onAck?(order: Order, ack: CommandAck, queued: boolean): void;
  /** The selection gained `added`; `clicked` is the unit clicked to gain them, if one was. */
  onSelect?(added: readonly number[], clicked: number | null): void;
}

export function useUnitControl(
  client: SimClient | null,
  observation: ObservationView | null,
  events?: UnitControlEvents,
  enabled = true,
) {
  const { units: catalog } = useSessionCatalog();
  const enabledRef = useRef(enabled);
  enabledRef.current = enabled;
  const eventsRef = useRef(events);
  eventsRef.current = events;
  const [selected, setSelection] = useState<number[]>([]);
  // The selection as last heard, and the unit a left-click picked to change it.
  const heardSelection = useRef<readonly number[]>([]);
  const clickedRef = useRef<number | null>(null);
  useEffect(() => {
    const before = new Set(heardSelection.current);
    heardSelection.current = selected;
    const added = selected.filter((id) => !before.has(id));
    const clicked = clickedRef.current;
    clickedRef.current = null;
    if (added.length)
      eventsRef.current?.onSelect?.(
        added,
        clicked !== null && added.includes(clicked) ? clicked : null,
      );
  }, [selected]);
  // Only units on the field are selectable: a squad a carrier holds drops
  // out however it got in (a click, a box, a double-click, or boarding).
  const selectedUnits = useMemo(
    () =>
      selected.flatMap(
        (id) => observation?.own.find((u) => u.id === id && !aboardCarrier(u)) ?? [],
      ),
    [observation, selected],
  );
  // Reconcile before committing handlers: a casualty or a boarding squad
  // cannot poison the rest's orders.
  if (selectedUnits.length !== selected.length) setSelection(selectedUnits.map((u) => u.id));
  const [acks, setAcks] = useState<AckEntry[]>([]);
  const [unloadPoints, setUnloadPoints] = useState<UnloadPoints>(NO_UNLOAD_POINTS);
  // Points passed or replaced retire as the observation shows it, before
  // anything reads them (as the selection reconciles above).
  const liveUnloadPoints = observation ? retire(unloadPoints, observation) : unloadPoints;
  if (liveUnloadPoints !== unloadPoints) setUnloadPoints(liveUnloadPoints);
  /** Where each carrier's orders let its passengers out: the order line's
   *  unload mark. */
  const unloads = useMemo(() => unloadMarks(liveUnloadPoints), [liveUnloadPoints]);
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
  /** Every own unit as last observed: passengers are among them. */
  const ownUnits = useCallback(() => observationRef.current?.own ?? [], []);
  const clientRef = useRef(client);
  clientRef.current = client;
  /** Space held (D2+): the order overlay shows every own unit. */
  const showOrders = useHeldKey(ShowOrdersBinding.code, enabled ? window : null);

  // A new client (reset) starts with no selection and an empty log.
  useEffect(() => {
    clientRef.current = client;
    setSelection([]);
    setAcks([]);
    setUnloadPoints(NO_UNLOAD_POINTS);
    setMode("move");
    gestures.current = new MoveGestures();
    clicks.current = new SelectClicks();
    return () => {
      clientRef.current = null;
    };
  }, [client, setMode]);

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
        case "set_afterburner":
          return `${order.enabled ? "afterburners on" : "afterburners off"}: ${order.units.map(unitName).join(", ")}`;
        case "set_deployment":
          return `${order.deployed ? "deploy" : "pack"} ${order.units.map(unitName).join(", ")}`;
        case "garrison":
          return `garrison building ${order.building} with ${order.units.map(unitName).join(", ")}${queued ? " (queued)" : ""}`;
        case "occupy_building":
          return `occupy building ${order.building} with ${order.units.map(unitName).join(", ")}${queued ? " (queued)" : ""}`;
        case "exit_building":
          return `leave building: ${order.units.map(unitName).join(", ")}`;
        case "board":
          return `board carrier ${unitName(order.carrier)} with ${order.units.map(unitName).join(", ")}${queued ? " (queued)" : ""}`;
        case "collect":
          return `collect ${order.squads.map(unitName).join(", ")} into ${order.units.map(unitName).join(", ")}${queued ? " (queued)" : ""}`;
        case "unload":
          return order.goal
            ? `unload ${order.units.map(unitName).join(", ")} at (${order.goal
                .map((v) => v.toFixed(0))
                .join(", ")})${queued ? " (queued)" : ""}`
            : `unload ${order.units.map(unitName).join(", ")} here`;
        case "refund":
          return `return ${order.units.map(unitName).join(", ")} to base`;
        case "ready":
          return "ready";
        case "confirm_purchase":
          return `purchase ${order.variant}`;
        case "cancel_pending":
          return `cancel pending purchase ${order.purchase}`;
        case "upgrade_move":
          return `upgrade gesture ${order.gesture} to fast route`;
      }
    },
    [unitName],
  );

  const issue = useCallback(
    async (order: Order, queued = false) => {
      if (!client || !enabledRef.current) return null;
      eventsRef.current?.onIssue?.(order, queued);
      const label = describe(order, queued);
      const ack = await client.command(order, queued);
      if (clientRef.current === client) {
        setUnloadPoints((points) => afterOrder(points, order, ack, queued));
        setAcks((log) => [{ seq: ack.seq, label, order, ack }, ...log].slice(0, LOG_LENGTH));
        eventsRef.current?.onAck?.(order, ack, queued);
      }
      return ack;
    },
    [client, describe],
  );

  /** What a pointer at `pick` would order, by the selection and armed mode. */
  const intentAt = useCallback(
    (pick: PointerPick) =>
      pointerIntent(pick, selectedUnitsRef.current, modeRef.current, catalog, ownUnits()),
    [catalog, ownUnits],
  );

  const onPointer = useCallback(
    (pick: PointerPick, captured?: PointerIntent) => {
      if (!enabledRef.current) return;
      // A right-click cancels a mode a left-click confirms, and a press
      // captured in one.
      if (
        pick.button === "right" &&
        (CONFIRMED_BY_LEFT_CLICK.has(modeRef.current) ||
          captured?.kind === "unload" ||
          (captured?.kind === "attack" && captured.target.kind === "ground"))
      ) {
        setMode("move");
        return;
      }
      if (pick.button === "left" && !CONFIRMED_BY_LEFT_CLICK.has(modeRef.current)) {
        const unit = pick.unit;
        const own = observationRef.current?.own ?? [];
        const kind = own.find((u) => u.id === unit)?.kind ?? null;
        clickedRef.current = kind ? unit : null;
        const similar = clicks.current.click({ ...pick, unit: kind ? unit : null, kind });
        if (similar && kind) {
          // Select similar: every own unit of its type, or of its role
          // (Shift adds them to the selection).
          const hits = similarUnits(own, kind, similar, catalog);
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
      const intent = captured
        ? reconcilePointerIntent(captured, observationRef.current)
        : intentAt(pick);
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
    [issue, setMode, intentAt, catalog],
  );

  /** Return fire only for the selection, or Fire at will if all hold. */
  const togglePolicy = useCallback(() => {
    const units = selectedUnitsRef.current;
    if (!units.length) return;
    const hold = units.every((u) => u.engagement === "return_fire_only");
    // The policy and its units come from one read of the live selection.
    void issue({
      kind: "set_engagement",
      units: units.map((u) => u.id),
      policy: hold ? "fire_at_will" : "return_fire_only",
    });
  }, [issue]);

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
      const units = reach("deploy", selectedUnitsRef.current, catalog, ownUnits()).map((u) => u.id);
      if (units.length) void issue({ kind: "set_deployment", units, deployed });
    },
    [issue, catalog, ownUnits],
  );

  /** Deploy the selection's units that deploy, or pack them if all are
   *  already deployed or deploying. */
  const toggleDeployment = useCallback(() => {
    const units = reach("deploy", selectedUnitsRef.current, catalog, ownUnits());
    if (!units.length) return;
    const deployed = units.every((u) => u.deployment?.target === "deployed");
    void issue({ kind: "set_deployment", units: units.map((u) => u.id), deployed: !deployed });
  }, [issue, catalog, ownUnits]);

  /** Toggle the persistent afterburner on the selected aircraft. */
  const toggleAfterburner = useCallback(() => {
    const aircraft = selectedUnitsRef.current.filter((u) => u.fuelS != null);
    if (!aircraft.length) return;
    const enabled = !aircraft.every((u) => u.afterburner);
    void issue({ kind: "set_afterburner", units: aircraft.map((u) => u.id), enabled });
  }, [issue]);

  /** The selection's units inside a building leave it. */
  const exitBuilding = useCallback(() => {
    const units = reach("exit_building", selectedUnitsRef.current, catalog, ownUnits()).map(
      (u) => u.id,
    );
    if (units.length) void issue({ kind: "exit_building", units });
  }, [issue, catalog, ownUnits]);

  /** Unload (D8): the first press arms it, for a click on the ground; a
   *  press while armed unloads where the carriers stand, replacing their
   *  orders. Only carriers with passengers take it. */
  const pressUnload = useCallback(() => {
    const units = reach("unload", selectedUnitsRef.current, catalog, ownUnits()).map((u) => u.id);
    if (!units.length) return;
    if (modeRef.current !== "unload") {
      setMode("unload");
      return;
    }
    setMode("move");
    const gesture = gestures.current.token();
    void issue({ kind: "unload", units, goal: null, gesture, route: "shortest" });
  }, [issue, catalog, setMode, ownUnits]);

  const refund = useCallback(() => {
    const units = selectedUnitsRef.current.map((unit) => unit.id);
    if (units.length) void issue({ kind: "refund", units });
  }, [issue]);

  // Command keys, from the one binding table.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!enabledRef.current) return;
      const command = commandForKey(e);
      // Esc is the unit control's only while a command is armed; otherwise
      // it goes on to the page (the battle's pause menu).
      if (!command || (command === "disarm" && modeRef.current === "move")) return;
      e.preventDefault();
      const any = selectedUnitsRef.current.length > 0;
      const armed = reach("attack", selectedUnitsRef.current, catalog, ownUnits()).length > 0;
      if (command === "stop") stop();
      else if (command === "toggle_fire_policy") togglePolicy();
      else if (command === "toggle_deployment") toggleDeployment();
      else if (command === "toggle_afterburner") toggleAfterburner();
      else if (command === "unload") pressUnload();
      else if (command === "attack_move" && any) setMode("attack_move");
      else if (command === "reverse_move" && any) setMode("reverse_move");
      else if (command === "attack_ground" && armed) setMode("attack_ground");
      else if (command === "disarm") setMode("move");
    };
    // Capture: ahead of the page's own Esc (the pause menu), whatever the
    // listeners' order.
    window.addEventListener("keydown", onKey, { capture: true });
    return () => window.removeEventListener("keydown", onKey, { capture: true });
  }, [
    stop,
    togglePolicy,
    toggleDeployment,
    toggleAfterburner,
    pressUnload,
    setMode,
    catalog,
    ownUnits,
  ]);

  return {
    refund: observation?.skirmish ? refund : undefined,
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
    toggleAfterburner,
    exitBuilding,
    pressUnload,
    unloads,
    togglePolicy,
    mode,
    setMode,
    unitName,
    showOrders,
  };
}
