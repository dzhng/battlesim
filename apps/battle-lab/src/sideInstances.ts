import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import {
  deploymentParts,
  proxyForVehicle,
  SIDE_COLORS,
} from "@packages/battle-renderer/src/unitProxies";
import type { PointerPick } from "@web/battle/input/useUnitControl";
import type { Pose } from "@web/battle/present/interpolate";
import type { ObservationView } from "@web/battle/sim/observation";
import type { SideName } from "@web/battle/sim/protocol";
import type { LabPick } from "./LabViewport";
import { buildingUnderRay, groundUnderRay, type StaticWorld } from "./useStaticWorld";

export interface DrawnInstances {
  /** Proxies drawn this frame: vehicles and their deployment parts (soldiers
   *  are the models layer's). */
  instances: SceneInstance[];
  /** What a click can pick: every drawn vehicle and soldier, as boxes. */
  picks: SceneInstance[];
  /** Own unit id per pick, or null for an enemy. */
  owners: (number | null)[];
  /** Identified enemy handle per pick, or null for an own unit. */
  enemies: (number | null)[];
}

/** Everything one side may draw as a box: its own units (interpolated
 *  poses) and the enemies it identifies, as blended for the same moment.
 *  Unidentified enemies do not exist here. A vehicle is a drawn proxy and
 *  its pick box; a soldier is a pick box only, drawn by the models layer. */
export function sideInstances(
  side: SideName,
  own: readonly Pose[],
  identified: readonly Pose[],
  observation: ObservationView,
  selected: readonly number[],
): DrawnInstances {
  const enemy: SideName = side === "blue" ? "red" : "blue";
  const kinds = new Map(observation.own.map((u) => [u.id, u.kind]));
  const enemyKinds = new Map(observation.identified.map((e) => [e.id, e.kind]));
  const instances: SceneInstance[] = [];
  const picks: SceneInstance[] = [];
  const owners: (number | null)[] = [];
  const enemies: (number | null)[] = [];
  const add = (
    pose: Pose,
    kind: string,
    color: readonly [number, number, number],
    owner: number | null,
    handle: number | null,
    highlight: boolean,
  ) => {
    const vehicle = proxyForVehicle(kind);
    for (const p of pose.members.length ? pose.members : [pose.position]) {
      const box: SceneInstance = {
        kind: vehicle ?? "infantry",
        x: p[0],
        y: p[1],
        z: p[2],
        yaw: pose.yaw,
        color,
        highlight,
      };
      if (vehicle) instances.push(box);
      picks.push(box);
      owners.push(owner);
      enemies.push(handle);
    }
    // The folded/unfolded pose follows the published progress, never the reverse.
    if (vehicle && pose.deployment !== null)
      for (const part of deploymentParts(pose.position, pose.yaw, pose.deployment))
        instances.push({ ...part, color, highlight });
  };
  for (const pose of own) {
    const kind = kinds.get(pose.id);
    if (kind) add(pose, kind, SIDE_COLORS[side], pose.id, null, selected.includes(pose.id));
  }
  for (const pose of identified) {
    const kind = enemyKinds.get(pose.id);
    if (kind) add(pose, kind, SIDE_COLORS[enemy], null, pose.id, false);
  }
  return { instances, picks, owners, enemies };
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
  const k = pick.instance;
  return {
    ...pick,
    unit: k >= 0 ? (drawn.owners[k] ?? null) : null,
    enemy: k >= 0 ? (drawn.enemies[k] ?? null) : null,
    building: right ? buildingUnderRay(world, pick.ray) : null,
    ground: ground && [ground[0], ground[1]],
  };
}
