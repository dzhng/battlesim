import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { ObservationView, OwnUnitView } from "../sim/observationView";
import type { Order } from "../sim/protocol";
import { isAttackMoveClick } from "./commandBindings";
import { reach } from "./commandReach";
import { inReverseZone } from "./reverseZone";
import { controlled } from "./controlled";

export interface PointerPick {
  /** The own unit under the pointer. */
  unit: number | null;
  button: "left" | "right";
  shift: boolean;
  ctrl: boolean;
  x: number;
  y: number;
  time: number;
  ground: [number, number] | null;
  building?: number | null;
  enemy?: number | null;
  contact?: number | null;
  /** The picked contact is in the air: area fire cannot reach it (D22). */
  contactAirborne?: boolean;
  facingTo?: [number, number] | null;
}

export type CommandMode =
  | "move"
  | "attack_move"
  | "reverse_move"
  | "attack_ground"
  | "fast_move"
  | "garrison"
  | "unload";

/** Armed modes a left-click confirms and a right-click cancels; in any
 *  other a left-click selects. */
export const CONFIRMED_BY_LEFT_CLICK: ReadonlySet<CommandMode> = new Set([
  "attack_ground",
  "unload",
]);

type PointerOrder = Extract<
  Order,
  {
    kind: "move" | "attack" | "attack_move" | "occupy_building" | "board" | "collect" | "unload";
  }
>;
type WithoutGesture<O> = O extends unknown ? Omit<O, "gesture"> & { queued: boolean } : never;
export type PointerIntent =
  | WithoutGesture<PointerOrder>
  | { kind: "none" }
  | { kind: "blocked"; disarm: boolean };

/** A held intent keeps its target and selection, while observed losses still
 *  apply; only units `player` orders stay in it (`controlled`). */
export function reconcilePointerIntent(
  intent: PointerIntent,
  observation: ObservationView | null,
  player: string | undefined,
): PointerIntent {
  if (!("units" in intent)) return intent;
  const yours = controlled(observation, player);
  const ownIds = (ids: readonly number[]) =>
    ids.filter((id) => yours.some((unit) => unit.id === id));
  const units = ownIds(intent.units);
  if (!units.length) return { kind: "none" };
  if (intent.kind === "board" && !ownIds([intent.carrier]).length) return { kind: "none" };
  if (intent.kind === "collect") {
    const squads = ownIds(intent.squads);
    return squads.length ? { ...intent, units, squads } : { kind: "none" };
  }
  if (intent.kind === "attack" && intent.target.kind === "contact") {
    const target = intent.target.id;
    if (!observation?.contacts.some((contact) => contact.id === target))
      return { kind: "blocked", disarm: true };
  }
  return { ...intent, units };
}

/** A short tremor is a click, rather than an arrival-facing instruction. */
const MIN_FACING_DRAG_M = 1;

export function dragFacing(pick: Pick<PointerPick, "ground" | "facingTo">): number | undefined {
  if (!pick.ground || !pick.facingTo) return undefined;
  const dx = pick.facingTo[0] - pick.ground[0],
    dy = pick.facingTo[1] - pick.ground[1];
  return Math.hypot(dx, dy) < MIN_FACING_DRAG_M ? undefined : Math.atan2(dy, dx);
}

/** Right-clicking a unit the player orders (D7): squads selected board the
 *  carrier under the pointer; carriers selected collect the squad under it.
 *  Null when neither applies (an ally's unit included), and the click falls
 *  through to the ground. */
function carriageIntent(
  pick: PointerPick,
  selected: readonly OwnUnitView[],
  own: readonly OwnUnitView[],
  yours: readonly OwnUnitView[],
  catalog: UnitCatalog,
): PointerIntent | null {
  const target = pick.unit === null ? undefined : yours.find((u) => u.id === pick.unit);
  if (!target) return null;
  const queued = pick.shift;
  if (reach("collect", [target], catalog, own).length) {
    const units = reach("board", selected, catalog, own).map((u) => u.id);
    if (units.length) return { kind: "board", units, carrier: target.id, queued };
  }
  if (reach("board", [target], catalog, own).length) {
    const units = reach("collect", selected, catalog, own).map((u) => u.id);
    if (units.length) return { kind: "collect", units, squads: [target.id], queued };
  }
  return null;
}

/** Hover and dispatch describe one intent; only dispatch mints its gesture.
 *  `own` is every unit on the side, the passengers among them; `yours` the
 *  ones the player orders (`controlled`), the only ones it boards or collects. */
export function pointerIntent(
  pick: PointerPick,
  selected: readonly OwnUnitView[],
  mode: CommandMode,
  catalog: UnitCatalog,
  own: readonly OwnUnitView[],
  yours: readonly OwnUnitView[],
): PointerIntent {
  if (selected.length === 0) return { kind: "none" };
  const queued = pick.shift;
  const units = selected.map((u) => u.id);
  const armed = () => reach("attack", selected, catalog, own).map((u) => u.id);
  if (mode === "unload") {
    const carriers = reach("unload", selected, catalog, own).map((u) => u.id);
    if (!carriers.length) return { kind: "blocked", disarm: true };
    if (!pick.ground) return { kind: "blocked", disarm: false };
    return { kind: "unload", units: carriers, goal: pick.ground, route: "shortest", queued };
  }
  if (mode === "attack_ground") {
    if (!pick.ground) return { kind: "blocked", disarm: false };
    const units = armed();
    return units.length
      ? { kind: "attack", units, target: { kind: "ground", point: [...pick.ground, 0] }, queued }
      : { kind: "blocked", disarm: true };
  }
  if (isAttackMoveClick(pick)) {
    return pick.ground
      ? { kind: "attack_move", units, goal: pick.ground, queued }
      : { kind: "blocked", disarm: !!pick.ground };
  }
  if (pick.enemy != null) {
    const units = armed();
    return units.length
      ? { kind: "attack", units, target: { kind: "identified", id: pick.enemy }, queued }
      : { kind: "blocked", disarm: true };
  }
  if (pick.contact != null && !pick.contactAirborne) {
    const units = armed();
    if (units.length)
      return { kind: "attack", units, target: { kind: "contact", id: pick.contact }, queued };
  }
  if (mode === "move") {
    const carriage = carriageIntent(pick, selected, own, yours, catalog);
    if (carriage) return carriage;
  }
  const facing = dragFacing(pick);
  if (pick.building != null && (mode === "move" || mode === "garrison")) {
    return {
      kind: "occupy_building",
      units,
      building: pick.building,
      queued,
      ...(facing === undefined ? {} : { facing }),
    };
  }
  if (!pick.ground || mode === "garrison") return { kind: "blocked", disarm: false };
  if (mode === "attack_move") return { kind: "attack_move", units, goal: pick.ground, queued };

  return {
    kind: "move",
    units,
    goal: pick.ground,
    queued,
    route: mode === "fast_move" ? "fastest" : "shortest",
    direction:
      mode === "reverse_move" || (mode === "move" && inReverseZone(catalog, selected, pick.ground))
        ? "reverse"
        : "forward",
    ...(facing === undefined ? {} : { facing }),
  };
}
