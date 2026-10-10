import { vec2, type Vec2 } from "math";
import { bodyBox, type PickBox, type SoldierBody } from "@packages/battle-renderer/src/picking";
import {
  aircraftCircle,
  circleContains,
  unitCircle,
  type UnitCircle,
} from "@packages/battle-renderer/src/orderOverlay";
import { airborne, type UnitCatalog } from "@packages/scene-assets/src/units";
import type { PointerPick } from "@web/battle/input/pointerIntent";
import type { Pose } from "@web/battle/present/interpolate";
import { contactUnder } from "@web/battle/input/contactPick";
import type { ContactView, ObservationView } from "@web/battle/sim/observation";
import type { LabPick } from "./LabViewport";
import { buildingUnderRay, groundUnderRay, type StaticWorld } from "./useStaticWorld";
import { aircraftView, orderView } from "./battleOverlay";
import { gameOrderStyle } from "./gameOverlay";

export interface DrawnInstances {
  /** What a click can pick: every drawn soldier and vehicle, by the
   *  simulation's box for its body (the models layer draws them). */
  picks: PickBox[];
  /** Own unit id per pick, or null for an enemy. */
  owners: (number | null)[];
  /** Identified enemy handle per pick, or null for an own unit. */
  enemies: (number | null)[];
  /** Each unit's marker circle on the ground, where a click that meets no
   *  body still picks it: every own unit's as its selection draws it, and
   *  every identified enemy aircraft's, which is always drawn. */
  rings: PickRing[];
}

/** A unit's marker circle on the ground, and whose it is. */
export interface PickRing {
  circle: UnitCircle;
  unit: number | null;
  enemy: number | null;
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
  const drawn: DrawnInstances = { picks: [], owners: [], enemies: [], rings: [] };
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
  // The rings, where this frame draws each unit: an own unit's as if it
  // were selected (the ring a click there would show).
  const poses = new Map(own.map((p) => [p.id, p]));
  for (const u of observation.own) {
    const pose = poses.get(u.id);
    if (!pose) continue;
    const view = orderView(units, u, true);
    const circle = unitCircle(
      { ...view, position: pose.position, members: pose.members },
      gameOrderStyle,
    );
    if (circle) drawn.rings.push({ circle, unit: u.id, enemy: null });
  }
  const enemyPoses = new Map(identified.map((p) => [p.id, p]));
  for (const e of observation.identified) {
    const pose = enemyPoses.get(e.id);
    if (!pose || !airborne(units.type(e.kind))) continue;
    const circle = aircraftCircle(aircraftView(units, { ...e, ...pose }), gameOrderStyle);
    drawn.rings.push({ circle, unit: null, enemy: e.id });
  }
  return drawn;
}

/** The unit whose ring `point` lies in (the nearest centre where rings
 *  overlap): an own unit's only with `includeOwn` (a left click selects it;
 *  a right click there still moves), an enemy aircraft's always. */
export function ringUnder(
  rings: readonly PickRing[],
  point: readonly [number, number],
  includeOwn: boolean,
): { unit: number | null; enemy: number | null } | null {
  let best: PickRing | null = null;
  let bestD = Infinity;
  for (const ring of rings) {
    if ((ring.unit !== null && !includeOwn) || !circleContains(ring.circle, point)) continue;
    const d = vec2.squaredDistance(ring.circle.c as Vec2, point as Vec2);
    if (d < bestD) [best, bestD] = [ring, d];
  }
  return best && { unit: best.unit, enemy: best.enemy };
}

/** A drawn body belongs to one own unit or one identified enemy. */
export function pickedUnit(drawn: Pick<DrawnInstances, "owners" | "enemies">, index: number) {
  return { unit: drawn.owners[index] ?? null, enemy: drawn.enemies[index] ?? null };
}

/** A viewport pick in player terms: the own unit or identified enemy drawn
 *  under the pointer (its body, else its ring on the ground: `ringUnder`),
 *  the ground it meets and its contact area, and for a right-click the
 *  building it meets. */
export function pickToPointer(
  world: StaticWorld,
  drawn: Pick<DrawnInstances, "owners" | "enemies" | "rings">,
  pick: LabPick,
  contacts: readonly ContactView[] = [],
): PointerPick {
  const right = pick.button === "right";
  const ground = groundUnderRay(world.view, pick.ray);
  const faced = right && pick.release ? groundUnderRay(world.view, pick.release) : null;
  const picked = (contact: number | null) => ({
    contact,
    contactAirborne: contacts.find((c) => c.id === contact)?.layer !== "ground" && contact != null,
  });
  const ringed =
    pick.instance < 0 && ground ? ringUnder(drawn.rings, [ground[0], ground[1]], !right) : null;
  return {
    ...pick,
    ...(ringed ?? pickedUnit(drawn, pick.instance)),
    building: right ? buildingUnderRay(world, pick.ray) : null,
    ground: ground && [ground[0], ground[1]],
    ...picked(contactUnder(contacts, ground && [ground[0], ground[1]], pick.ray)),
    facingTo: faced && [faced[0], faced[1]],
  };
}
