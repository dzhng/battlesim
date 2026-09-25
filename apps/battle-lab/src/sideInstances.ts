import type { SceneInstance } from "@packages/battle-renderer/src/scene";
import {
  deploymentParts,
  proxyForUnit,
  SIDE_COLORS,
} from "@packages/battle-renderer/src/unitProxies";
import type { Pose } from "@web/battle/present/interpolate";
import type { ObservationView } from "@web/battle/sim/observation";
import type { SideName } from "@web/battle/sim/protocol";

export interface DrawnInstances {
  instances: SceneInstance[];
  /** Own unit id per instance, or null for an enemy. */
  owners: (number | null)[];
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
    }
    // The folded/unfolded pose follows the published progress, never the reverse.
    if (pose.deployment !== null) {
      for (const part of deploymentParts(pose.position, pose.yaw, pose.deployment)) {
        instances.push({ ...part, color: SIDE_COLORS[side], highlight });
        owners.push(pose.id);
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
    }
  }
  return { instances, owners };
}
