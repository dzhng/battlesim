// Purchases as ghosts: the one in hand on the cursor, and a skirmish's
// pending purchases, each placed during preparation standing where it will
// go, turned as it was placed, until the battle starts. A squad bought aboard
// a carrier stands as its carrier.
import { DEFAULT_PLACEMENT_FACING, type PurchaseGhost } from "@web/battle/input/purchasePlacement";
import type { PendingPurchaseView } from "@web/battle/sim/observationView";
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
