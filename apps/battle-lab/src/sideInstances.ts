import { bodyBox, type BodyRules, type PickBox } from "@packages/battle-renderer/src/picking";
import type { PointerPick } from "@web/battle/input/useUnitControl";
import type { Pose } from "@web/battle/present/interpolate";
import type { ObservationView } from "@web/battle/sim/observation";
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
  bodies: BodyRules,
): DrawnInstances {
  const kinds = new Map(observation.own.map((u) => [u.id, u.kind]));
  const enemyKinds = new Map(observation.identified.map((e) => [e.id, e.kind]));
  const drawn: DrawnInstances = { picks: [], owners: [], enemies: [] };
  const add = (pose: Pose, kind: string, owner: number | null, handle: number | null) => {
    const body = bodyBox(bodies, kind);
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

/** A viewport pick in player terms: the own unit or identified enemy drawn
 *  under the pointer, and for a right-click the building and ground it meets. */
export function pickToPointer(
  world: StaticWorld,
  drawn: Pick<DrawnInstances, "owners" | "enemies">,
  pick: LabPick,
): PointerPick {
  const right = pick.button === "right";
  const ground = right ? groundUnderRay(world.view, pick.ray) : null;
  const faced = right && pick.release ? groundUnderRay(world.view, pick.release) : null;
  const k = pick.instance;
  return {
    ...pick,
    unit: k >= 0 ? (drawn.owners[k] ?? null) : null,
    enemy: k >= 0 ? (drawn.enemies[k] ?? null) : null,
    building: right ? buildingUnderRay(world, pick.ray) : null,
    ground: ground && [ground[0], ground[1]],
    facingTo: faced && [faced[0], faced[1]],
  };
}
