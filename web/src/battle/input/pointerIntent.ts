import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { OwnUnitView } from "../sim/observation";
import type { Order } from "../sim/protocol";
import { isAttackMoveClick } from "./commandBindings";
import { reach } from "./commandReach";
import { inReverseZone } from "./reverseZone";

export interface PointerPick {
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
  facingTo?: [number, number] | null;
}

export type CommandMode =
  | "move"
  | "attack_move"
  | "reverse_move"
  | "attack_ground"
  | "fast_move"
  | "garrison";

type PointerOrder = Extract<Order, { kind: "move" | "attack" | "attack_move" | "occupy_building" }>;
type WithoutGesture<O> = O extends unknown ? Omit<O, "gesture"> & { queued: boolean } : never;
export type PointerIntent =
  | WithoutGesture<PointerOrder>
  | { kind: "none" }
  | { kind: "blocked"; disarm: boolean };

/** A short tremor is a click, rather than an arrival-facing instruction. */
const MIN_FACING_DRAG_M = 1;

export function dragFacing(pick: Pick<PointerPick, "ground" | "facingTo">): number | undefined {
  if (!pick.ground || !pick.facingTo) return undefined;
  const dx = pick.facingTo[0] - pick.ground[0],
    dy = pick.facingTo[1] - pick.ground[1];
  return Math.hypot(dx, dy) < MIN_FACING_DRAG_M ? undefined : Math.atan2(dy, dx);
}

/** Hover and dispatch describe one intent; only dispatch mints its gesture. */
export function pointerIntent(
  pick: PointerPick,
  selected: readonly OwnUnitView[],
  mode: CommandMode,
  catalog: UnitCatalog,
): PointerIntent {
  if (selected.length === 0) return { kind: "none" };
  const queued = pick.shift;
  const armed = () => reach("attack", selected, catalog).map((u) => u.id);
  if (isAttackMoveClick(pick)) {
    const units = armed();
    return pick.ground && units.length
      ? { kind: "attack_move", units, goal: pick.ground, queued }
      : { kind: "blocked", disarm: !!pick.ground };
  }
  if (pick.enemy != null) {
    const units = armed();
    return units.length
      ? { kind: "attack", units, target: { kind: "identified", id: pick.enemy }, queued }
      : { kind: "blocked", disarm: true };
  }
  if (pick.contact != null) {
    const units = armed();
    if (units.length)
      return { kind: "attack", units, target: { kind: "contact", id: pick.contact }, queued };
  }
  const units = selected.map((u) => u.id);
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
  if (mode === "attack_move" || mode === "attack_ground") {
    const units = armed();
    if (!units.length) return { kind: "blocked", disarm: true };
    return mode === "attack_move"
      ? { kind: "attack_move", units, goal: pick.ground, queued }
      : { kind: "attack", units, target: { kind: "ground", point: [...pick.ground, 0] }, queued };
  }
  return {
    kind: "move",
    units,
    goal: pick.ground,
    queued,
    route: mode === "fast_move" ? "fastest" : "shortest",
    direction:
      mode === "reverse_move" || (mode === "move" && inReverseZone(selected, pick.ground))
        ? "reverse"
        : "forward",
    ...(facing === undefined ? {} : { facing }),
  };
}
