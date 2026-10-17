/** Which of a mixed selection a command reaches (the user's call): the
 *  command is available when any selected unit can carry it out, and its
 *  order goes to exactly those units. Orders every unit takes (moves, stop, the
 *  fire policy) are not listed: they reach the whole selection.
 *  The one owner of "can this unit do it", for the bar, the keys and the
 *  right-click alike. */
import { inBuilding, passengersOf, type OwnUnitView } from "../sim/observationView";
import type { UnitCatalog } from "@packages/scene-assets/src/units";

export type ReachCommand =
  | "attack"
  | "deploy"
  | "garrison"
  | "exit_building"
  | "board"
  | "collect"
  | "unload";

type Reach = Pick<OwnUnitView, "id" | "kind" | "hold" | "deployment">;

const CAN: Record<
  ReachCommand,
  (u: Reach, units: UnitCatalog, own: readonly Pick<OwnUnitView, "hold">[]) => boolean
> = {
  /** Its type has mounts. Targeted attacks and attack-ground go only
   *  to these: pursuit runs through the weapons pass, so an unarmed targeted
   *  attack would freeze. Attack-move reaches the whole selection. */
  attack: (u, units) => units.type(u.kind).mounts.length > 0,
  /** Its type has the deploy capability. */
  deploy: (u, units) => !!units.type(u.kind).capabilities.deploy,
  /** Infantry: garrisons are a fighting position for squads. */
  garrison: (u, units) => units.isInfantry(u.kind),
  /** It is inside a building. */
  exit_building: inBuilding,
  /** Infantry ride carriers, as they garrison buildings. */
  board: (u, units) => units.isInfantry(u.kind),
  /** Its type has seats. */
  collect: (u, units) => !!units.type(u.kind).capabilities.carriage,
  /** It has seats and someone in them. */
  unload: (u, units, own) =>
    !!units.type(u.kind).capabilities.carriage && passengersOf(own, u.id).length > 0,
};

/** The units of `selection` that `command` reaches, in selection order.
 *  `own` is every own unit, the passengers among them. */
export function reach<U extends Reach>(
  command: ReachCommand,
  selection: readonly U[],
  units: UnitCatalog,
  own: readonly Pick<OwnUnitView, "hold">[],
): U[] {
  return selection.filter((u) => CAN[command](u, units, own));
}
