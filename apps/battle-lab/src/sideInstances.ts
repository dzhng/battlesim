import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import {
  deploymentParts,
  proxyForUnit,
  SIDE_COLORS,
} from "@packages/battle-renderer/src/unitProxies";
import type { PointerPick } from "@web/battle/input/useUnitControl";
import type { Pose } from "@web/battle/present/interpolate";
import type { ObservationView } from "@web/battle/sim/observation";
import type { SideName } from "@web/battle/sim/protocol";
import type { LabPick } from "./LabViewport";
import { buildingUnderRay, groundUnderRay, type StaticWorld } from "./useStaticWorld";

export interface DrawnInstances {
  instances: SceneInstance[];
  /** Own unit id per instance, or null for an enemy. */
  owners: (number | null)[];
  /** Identified enemy handle per instance, or null for an own unit. */
  enemies: (number | null)[];
}

/** Everything one side may draw: its own units (interpolated poses) and the
 *  enemies it identifies, exactly as observed. Unidentified enemies do not
 *  exist here. */
export function sideInstances(
  side: SideName,
  poses: Pose[],
  observation: ObservationView,
  selected: readonly number[],
): DrawnInstances {
  const enemy: SideName = side === "blue" ? "red" : "blue";
  const kinds = new Map(observation.own.map((u) => [u.id, u.kind]));
  const instances: SceneInstance[] = [];
  const owners: (number | null)[] = [];
  const enemies: (number | null)[] = [];
  for (const pose of poses) {
    const kind = kinds.get(pose.id);
    if (!kind) continue;
    const highlight = selected.includes(pose.id);
    for (const p of pose.members.length ? pose.members : [pose.position]) {
      instances.push({
        kind: proxyForUnit(kind),
        x: p[0],
        y: p[1],
        z: p[2],
        yaw: pose.yaw,
        color: SIDE_COLORS[side],
        highlight,
      });
      owners.push(pose.id);
      enemies.push(null);
    }
    // The folded/unfolded pose follows the published progress, never the reverse.
    if (pose.deployment !== null) {
      for (const part of deploymentParts(pose.position, pose.yaw, pose.deployment)) {
        instances.push({ ...part, color: SIDE_COLORS[side], highlight });
        owners.push(pose.id);
        enemies.push(null);
      }
    }
  }
  for (const e of observation.identified) {
    for (const p of e.members.length ? e.members : [e.position]) {
      instances.push({
        kind: proxyForUnit(e.kind),
        x: p[0],
        y: p[1],
        z: p[2],
        yaw: e.yaw,
        color: SIDE_COLORS[enemy],
      });
      owners.push(null);
      enemies.push(e.id);
    }
  }
  return { instances, owners, enemies };
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
