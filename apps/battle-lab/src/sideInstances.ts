import { bodyBox, type PickBox, type SoldierBody } from "@packages/battle-renderer/src/picking";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { PointerPick } from "@web/battle/input/pointerIntent";
import type { Pose } from "@web/battle/present/interpolate";
import { contactUnder } from "@web/battle/input/contactPick";
import type { ContactView, ObservationView } from "@web/battle/sim/observation";
import type { LabPick } from "./LabViewport";
import { buildingUnderRay, groundUnderRay, type StaticWorld } from "./useStaticWorld";

export interface DrawnInstances {
  /** What a click can pick: every drawn soldier and vehicle, by the
   *  simulation's box for its body (the models layer draws them). */
  picks: PickBox[];
  /** Own unit id per pick, or null for an enemy. */
  owners: (number | null)[];
  /** Identified enemy handle per pick, or null for an own unit. */
  enemies: (number | null)[];
}

/** Everything one side may pick: its own units (interpolated poses) and the
 *  enemies it identifies, as blended for the same moment. Unidentified
 *  enemies do not exist here. */
export function sideInstances(
  own: readonly Pose[],
  identified: readonly Pose[],
  observation: ObservationView,
  soldier: SoldierBody,
  units: UnitCatalog,
): DrawnInstances {
  const kinds = new Map(observation.own.map((u) => [u.id, u.kind]));
  const enemyKinds = new Map(observation.identified.map((e) => [e.id, e.kind]));
  const drawn: DrawnInstances = { picks: [], owners: [], enemies: [] };
  const add = (pose: Pose, kind: string, owner: number | null, handle: number | null) => {
    const body = bodyBox(soldier, units.hull(kind));
    for (const p of pose.members.length ? pose.members : [pose.position]) {
      drawn.picks.push({ x: p[0], y: p[1], z: p[2], yaw: pose.yaw, ...body });
      drawn.owners.push(owner);
      drawn.enemies.push(handle);
    }
  };
  for (const pose of own) {
    const kind = kinds.get(pose.id);
    if (kind) add(pose, kind, pose.id, null);
  }
  for (const pose of identified) {
    const kind = enemyKinds.get(pose.id);
    if (kind) add(pose, kind, null, pose.id);
  }
  return drawn;
}

/** A drawn body belongs to one own unit or one identified enemy. */
export function pickedUnit(drawn: Pick<DrawnInstances, "owners" | "enemies">, index: number) {
  return { unit: drawn.owners[index] ?? null, enemy: drawn.enemies[index] ?? null };
}

/** A viewport pick in player terms: the own unit or identified enemy drawn
 *  under the pointer, and for a right-click the building and ground it meets
 *  and the contact whose area holds that ground (of the side's `contacts`). */
export function pickToPointer(
  world: StaticWorld,
  drawn: Pick<DrawnInstances, "owners" | "enemies">,
  pick: LabPick,
  contacts: readonly ContactView[] = [],
): PointerPick {
  const right = pick.button === "right";
  const ground = right ? groundUnderRay(world.view, pick.ray) : null;
  const faced = right && pick.release ? groundUnderRay(world.view, pick.release) : null;
  return {
    ...pick,
    ...pickedUnit(drawn, pick.instance),
    building: right ? buildingUnderRay(world, pick.ray) : null,
    ground: ground && [ground[0], ground[1]],
    contact: ground ? contactUnder(contacts, [ground[0], ground[1]]) : null,
    facingTo: faced && [faced[0], faced[1]],
  };
}
