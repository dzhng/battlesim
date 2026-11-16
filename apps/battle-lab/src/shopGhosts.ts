// Purchases as ghosts: the one in hand on the cursor; a skirmish's pending
// purchases, each placed during preparation standing where it will go,
// turned as it was placed, until the battle starts; and in battle, each one
// just bought, fading where it was placed as the sign it went through. A
// squad bought aboard a carrier stands as its carrier.
import { DEFAULT_PLACEMENT_FACING, type PurchaseGhost } from "@web/battle/input/purchasePlacement";
import type { OwnUnitView, PendingPurchaseView } from "@web/battle/sim/observationView";
import {
  GHOST_BLOCKED,
  GHOST_OWN,
  GHOST_SELECTED,
  fieldedSoldiers,
  type UnitGhost,
} from "./unitGhosts";

/** The purchase in hand as its ghost: what will arrive (a squad's carrier,
 *  else the unit), where the authority would send it, blocked where it can't. */
export function heldGhost(placement: PurchaseGhost): UnitGhost {
  return {
    kind: placement.carrier ?? placement.variant,
    at: placement.destination,
    soldiers: fieldedSoldiers(placement.spots),
    yaw: placement.facing,
    look: placement.valid === false ? GHOST_BLOCKED : GHOST_SELECTED,
  };
}

/** A purchase made in battle: its ghost, when the authority accepted it
 *  (the page's performance clock, ms), and the newest own unit id before
 *  it, so the unit it bought is a newer one. */
export interface Bought {
  ghost: PurchaseGhost;
  at: number;
  after: number;
}

/** How long a bought unit's ghost lingers where it was placed, fading. */
export const BOUGHT_FADE_MS = 5000;
/** How near its spot the bought unit comes to take its ghost's place. */
const ARRIVED_M = 8;

/** The purchases made in the last moments, as ghosts fading where they
 *  were placed: each gone once faded, or once the unit it bought (an own
 *  unit of its kind newer than the purchase) reaches its spot. */
export function boughtGhosts(
  bought: readonly Bought[],
  now: number,
  own: readonly OwnUnitView[],
): UnitGhost[] {
  return bought.flatMap(({ ghost, at, after }) => {
    const age = (now - at) / BOUGHT_FADE_MS;
    if (age >= 1) return [];
    const kind = ghost.carrier ?? ghost.variant;
    const [x, y] = ghost.destination;
    const arrived = own.some(
      (unit) =>
        unit.id > after &&
        unit.kind === kind &&
        Math.hypot(unit.position[0] - x, unit.position[1] - y) <= ARRIVED_M,
    );
    if (arrived) return [];
    // A ghost draws every surface of its model, so a pixel stacks several
    // layers and reads near solid until its opacity is low: a steep curve
    // is what the eye sees as a steady fade.
    return [{ ...heldGhost(ghost), opacity: (1 - Math.max(0, age)) ** 1.5 }];
  });
}

/** How near a pending purchase's published destination is to the placed one. */
const PLACED_MATCH_M = 0.5;

/** `pending` as ghosts, each standing as the matching `placed` ghost was
 *  placed (its soldiers moved with the published destination). */
export function pendingGhosts(
  pending: readonly PendingPurchaseView[],
  placed: readonly PurchaseGhost[],
): UnitGhost[] {
  return pending.map((purchase) => {
    const [x, y] = purchase.destination;
    const match = placed.find(
      (g) =>
        g.variant === purchase.kind &&
        g.carrier === purchase.carrier &&
        Math.hypot(g.destination[0] - x, g.destination[1] - y) < PLACED_MATCH_M,
    );
    const [dx, dy] = match ? [x - match.destination[0], y - match.destination[1]] : [0, 0];
    return {
      kind: purchase.carrier ?? purchase.kind,
      at: purchase.destination,
      soldiers: fieldedSoldiers(
        (match?.spots ?? []).map(([sx, sy]) => [sx + dx, sy + dy] as const),
      ),
      yaw: match?.facing ?? DEFAULT_PLACEMENT_FACING,
      look: GHOST_OWN,
    };
  });
}

/** `derive` of the last input it was given, derived again only when the
 *  input's reference changes (the decoder keeps an unchanged list's), so a
 *  frame reads its publication without rebuilding what did not change. */
export function byReference<In, Out>(derive: (input: In) => Out, initial: Out) {
  let from: In | undefined;
  const memo = {
    value: initial,
    of(input: In): Out {
      if (input !== from) {
        from = input;
        memo.value = derive(input);
      }
      return memo.value;
    },
  };
  return memo;
}
