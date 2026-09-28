/** Which of a mixed selection a command reaches (the user's call): the
 *  command bar shows the union of the selection's capabilities, a command
 *  is lit when any selected unit can carry it out, and its order goes to
 *  exactly the units that can. Orders every unit takes (moves, stop, the
 *  fire policy) are not listed: they reach the whole selection.
 *  The one owner of "can this unit do it", for the bar, the keys and the
 *  right-click alike. */
import type { OwnUnitView } from "../sim/observation";
import type { UnitCatalog } from "@packages/scene-assets/src/units";

export type ReachCommand = "attack" | "deploy" | "garrison" | "exit_building";

type Reach = Pick<OwnUnitView, "kind" | "garrison">;

const CAN: Record<ReachCommand, (u: Reach, units: UnitCatalog) => boolean> = {
  /** Its type has mounts. Every attack (at a unit, attack-move, attack
   *  ground) goes only to these: the simulation pursues a target through its
   *  weapons pass, which never runs for a unit with no mounts, so an unarmed
   *  unit given an attack would freeze. */
  attack: (u, units) => units.type(u.kind).mounts.length > 0,
  /** Its type has the deploy capability. */
  deploy: (u, units) => !!units.type(u.kind).capabilities.deploy,
  /** Infantry: garrisons are a fighting position for squads. */
  garrison: (u, units) => units.isInfantry(u.kind),
  /** It is inside a building. */
  exit_building: (u) => !!u.garrison,
};

/** The units of `selection` that `command` reaches, in selection order. */
export function reach<U extends Reach>(
  command: ReachCommand,
  selection: readonly U[],
  units: UnitCatalog,
): U[] {
  return selection.filter((u) => CAN[command](u, units));
}
