// When a pointer preview's answer goes stale: the identity of everything a
// move or garrison query depends on, and a revision that counts its changes.
import { useMemo, useRef } from "react";
import type { AckEntry } from "@web/battle/input/useUnitControl";
import type { ObservationView, OrderStepView } from "@web/battle/sim/observationView";
import type { SideName } from "@web/battle/sim/protocol";

/** A preview answer stands until the side's knowledge, its units, or its
 *  accepted orders not yet applied change. */
export function previewContextIdentity(
  side: SideName,
  knownKey: string,
  eligibilityIdentity: string,
  pendingClaims: string,
  clearedCount: number,
  clearingEpoch: number,
): string {
  return JSON.stringify([
    side,
    knownKey,
    eligibilityIdentity,
    pendingClaims,
    clearedCount,
    clearingEpoch,
  ]);
}

/** What a step was ordered at: its target or host, else its point; never
 *  where a target, carrier or squad it follows stands now. */
function stepIntent(step: OrderStepView) {
  switch (step.kind) {
    case "attack":
      return [step.kind, step.target ?? null];
    case "board":
      return [step.kind, step.host?.id ?? null];
    case "collect":
      return [step.kind];
    default:
      return [step.kind, step.point, step.host?.id ?? null];
  }
}

/** The preview context's revision: its `version` steps whenever the
 *  identity (`previewContextIdentity`) of `side`'s view changes. `knownKey`
 *  is the side's known props, and `clearedCount` and `clearingEpoch` its
 *  cleared ground. */
export function usePreviewRevision(
  side: SideName,
  observation: ObservationView | null,
  acks: readonly AckEntry[],
  knownKey: string,
  clearedCount: number,
  clearingEpoch: number,
) {
  const eligibilityIdentity = JSON.stringify(
    (observation?.own ?? []).map((u) => [
      u.id,
      u.kind,
      u.members.length,
      u.hold?.host.kind,
      u.hold?.host.id,
      u.hold?.phase,
      u.steps.map(stepIntent),
    ]),
  );
  const pendingClaims = JSON.stringify(
    acks
      .filter(({ ack }) => !ack.error && ack.applied_tick > (observation?.tick ?? 0))
      .map(({ seq, order }) => [seq, order]),
  );
  const identity = useMemo(
    () =>
      previewContextIdentity(
        side,
        knownKey,
        eligibilityIdentity,
        pendingClaims,
        clearedCount,
        clearingEpoch,
      ),
    [side, knownKey, eligibilityIdentity, pendingClaims, clearedCount, clearingEpoch],
  );
  const revision = useRef({ identity: "", version: 0 });
  if (revision.current.identity !== identity) {
    revision.current = { identity, version: revision.current.version + 1 };
  }
  return revision;
}
