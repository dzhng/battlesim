import { cruise_agl_m } from "@wasm/game_wasm";
// Where a unit will stand, drawn as its own models in a ghost colour: the one
// look for a purchase being placed or already placed, a move's held facing
// preview, and every ordered destination with Space held. The UI says where
// each body stands; the presentation fields and poses them (`GhostBody`).
import type { SurfaceHeight } from "@packages/battle-renderer/src/orderOverlay";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { GhostReadout } from "@web/battle/present/readouts";
import type { OwnUnitView } from "@web/battle/sim/observationView";
import type { GhostBody } from "@web/battle/sim/presentation";

/** A ghost's look (`GhostBody.look`): the side's own colour, the
 *  selection's, or a purchase the simulation refused. */
export const GHOST_OWN = 0;
export const GHOST_SELECTED = 1;
export const GHOST_BLOCKED = 2;

/** The rules' air layers, each with its cruise height, and high air's
 *  floor (`contract::scenario::AirRules`). */
export type AirLayers = typeof import("../../../fixtures/rules.json").air;

/** How high over the ground a kind's ghost stands, flying where it will be:
 *  an aircraft's own cruise height, else its layer's, never under its
 *  layer's floor (`AirRules::cruise_agl_m`); anything else on the ground. */
export function ghostAloft(units: UnitCatalog, air: AirLayers) {
  const layers = JSON.stringify(air);
  const heights = new Map<string, number>();
  return (kind: string): number => {
    const known = heights.get(kind);
    if (known !== undefined) return known;
    const height = cruise_agl_m(layers, JSON.stringify(units.type(kind).mobility));
    heights.set(kind, height);
    return height;
  };
}

/** Cards accompany only revealed HighAir order ghosts while Space is held. */
export function aircraftOrderReadouts(
  own: readonly OwnUnitView[],
  showOrders: boolean,
  revealed: ReadonlyMap<number, number>,
  isHighAir: (kind: string) => boolean,
  ground: SurfaceHeight,
  altitude: (kind: string) => number,
): GhostReadout[] {
  if (!showOrders) return [];
  return own.flatMap((unit) => {
    if (!revealed.has(unit.id) || !isHighAir(unit.kind)) return [];
    const ghost = orderedGhost(unit, GHOST_OWN);
    if (!ghost) return [];
    const [x, y] = ghost.at;
    return [{ unit, position: [x, y, ground(x, y) + altitude(unit.kind)] }];
  });
}

export interface UnitGhost {
  kind: string;
  /** A vehicle's centre; a squad stands at `soldiers` instead. */
  at: readonly [number, number];
  /** A squad's soldiers: where each stands, his slot (which soldier he is)
   *  and id (which of that soldier's looks). Empty for a vehicle. */
  soldiers: readonly { at: readonly [number, number]; slot: number; id: number }[];
  yaw: number;
  look: number;
}

/** A squad's soldiers at `spots`, in slot order, as a purchase fields them. */
export function fieldedSoldiers(spots: readonly (readonly [number, number])[]) {
  return spots.map((at, slot) => ({ at, slot, id: slot }));
}

/** An own unit's living soldiers at `spots`, given in member order. */
export function ownSoldiers(unit: OwnUnitView, spots: readonly (readonly [number, number])[]) {
  return spots.map((at, i) => ({
    at,
    slot: unit.memberSlots[i] ?? i,
    id: unit.memberIds[i] ?? i,
  }));
}

/** Where `unit` ends its orders, for Space: its last queued point, else its
 *  goal, with its soldiers' published spots carried there; null while it holds. */
export function orderedGhost(unit: OwnUnitView, look: number): UnitGhost | null {
  if (!unit.goal) return null;
  const end = unit.queue.at(-1) ?? unit.goal;
  const [dx, dy] = [end[0] - unit.goal[0], end[1] - unit.goal[1]];
  return {
    kind: unit.kind,
    at: end,
    soldiers: ownSoldiers(
      unit,
      unit.memberOrders.map(({ spot }) => [spot[0] + dx, spot[1] + dy] as const),
    ),
    yaw: unit.finalFacing,
    look,
  };
}

/** Each ghost's bodies on the ground under them: one per soldier for a
 *  squad, one hull for a vehicle. A squad with no soldiers placed yet draws
 *  nothing. */
export function ghostBodies(
  ghosts: readonly UnitGhost[],
  hull: (kind: string) => boolean,
  z: SurfaceHeight,
  altitude: (kind: string) => number,
): GhostBody[] {
  return ghosts.flatMap((ghost) =>
    (hull(ghost.kind) ? [{ at: ghost.at, slot: 0, id: 0 }] : ghost.soldiers).map(
      ({ at, slot, id }) => ({
        kind: ghost.kind,
        slot,
        id,
        x: at[0],
        y: at[1],
        z: z(at[0], at[1]) + altitude(ghost.kind),
        yaw: ghost.yaw,
        look: ghost.look,
      }),
    ),
  );
}
