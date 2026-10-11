import { vec2, type Vec2, type Vec3 } from "math";
import type { PickBox } from "@packages/battle-renderer/src/picking";
import { NONE, PICK_WORDS, Pick } from "@packages/battle-renderer/src/records.gen";
import {
  aircraftCircle,
  circleContains,
  unitCircle,
  type UnitCircle,
} from "@packages/battle-renderer/src/orderOverlay";
import { airborne, type UnitCatalog } from "@packages/scene-assets/src/units";
import type { PointerPick } from "@web/battle/input/pointerIntent";
import type { PresentedFrame } from "@web/battle/sim/presentation";
import { contactUnder } from "@web/battle/input/contactPick";
import type { ContactView, ObservationView } from "@web/battle/sim/observationView";
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

/** Where the frame drew one unit: its position, and its soldiers'. */
export interface DrawnUnit {
  position: Vec3;
  yaw: number;
  members: Vec3[];
}

/** Each drawn own unit and identified enemy, by id, from the frame's picks:
 *  a unit's own pick (`member` none) and its soldiers' after it. */
export function drawnUnits(frame: PresentedFrame): {
  own: Map<number, DrawnUnit>;
  enemies: Map<number, DrawnUnit>;
} {
  const own = new Map<number, DrawnUnit>();
  const enemies = new Map<number, DrawnUnit>();
  const { f32, u32 } = frame.picks;
  let unit: DrawnUnit | null = null;
  for (let i = 0; i < frame.picks.count; i++) {
    const o = i * PICK_WORDS;
    const half = f32[o + Pick.half + 2];
    const foot: Vec3 = [
      f32[o + Pick.centre],
      f32[o + Pick.centre + 1],
      f32[o + Pick.centre + 2] - half,
    ];
    if (u32[o + Pick.member] === NONE) {
      unit = { position: foot, yaw: f32[o + Pick.centre + 3], members: [] };
      (u32[o + Pick.flags] & OWN ? own : enemies).set(u32[o + Pick.unit], unit);
    } else unit?.members.push(foot);
  }
  return { own, enemies };
}

/** Everything one side may pick, as the frame drew it: its own units and
 *  the enemies it identifies, each soldier's body or a hull's
 *  (`Frame.picks`), and the rings round them. Unidentified enemies do not
 *  exist here. */
export function sideInstances(
  frame: PresentedFrame,
  drawnOwn: ReadonlyMap<number, DrawnUnit>,
  drawnEnemies: ReadonlyMap<number, DrawnUnit>,
  observation: ObservationView,
  units: UnitCatalog,
): DrawnInstances {
  const drawn: DrawnInstances = { picks: [], owners: [], enemies: [], rings: [] };
  const { f32, u32 } = frame.picks;
  for (let i = 0; i < frame.picks.count; i++) {
    const o = i * PICK_WORDS;
    const half: [number, number, number] = [
      f32[o + Pick.half],
      f32[o + Pick.half + 1],
      f32[o + Pick.half + 2],
    ];
    // A squad's own pick says only where it is drawn; its soldiers are hit.
    if (half[2] === 0) continue;
    const [x, y, z] = [f32[o + Pick.centre], f32[o + Pick.centre + 1], f32[o + Pick.centre + 2]];
    drawn.picks.push({
      x,
      y,
      z: z - half[2],
      yaw: f32[o + Pick.centre + 3],
      center: [0, 0, half[2]],
      half,
    });
    const id = u32[o + Pick.unit];
    const own = (u32[o + Pick.flags] & OWN) !== 0;
    drawn.owners.push(own ? id : null);
    drawn.enemies.push(own ? null : id);
  }
  // The rings, where this frame draws each unit: an own unit's as if it
  // were selected (the ring a click there would show).
  for (const u of observation.own) {
    const pose = drawnOwn.get(u.id);
    if (!pose) continue;
    const view = orderView(units, u, true);
    const circle = unitCircle(
      { ...view, position: pose.position, members: pose.members },
      gameOrderStyle,
    );
    if (circle) drawn.rings.push({ circle, unit: u.id, enemy: null });
  }
  for (const e of observation.identified) {
    const pose = drawnEnemies.get(e.id);
    if (!pose || !airborne(units.type(e.kind))) continue;
    const circle = aircraftCircle(aircraftView(units, { ...e, ...pose }), gameOrderStyle);
    drawn.rings.push({ circle, unit: null, enemy: e.id });
  }
  return drawn;
}

/** `Pick.flags`: an own unit's. */
const OWN = 1;

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
