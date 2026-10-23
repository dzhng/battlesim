// A skirmish's pending purchases as ghosts: each unit placed during
// preparation stands where it will go, turned as it was placed, until the
// battle starts.
import { DEFAULT_PLACEMENT_FACING, type PurchaseGhost } from "@web/battle/input/purchasePlacement";
import type { PendingPurchaseView } from "@web/battle/sim/observationView";
import { GHOST_OWN, fieldedSoldiers, type UnitGhost } from "./unitGhosts";

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
        Math.hypot(g.destination[0] - x, g.destination[1] - y) < PLACED_MATCH_M,
    );
    const [dx, dy] = match ? [x - match.destination[0], y - match.destination[1]] : [0, 0];
    return {
      kind: purchase.kind,
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
