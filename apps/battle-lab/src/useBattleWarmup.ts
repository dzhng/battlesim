/** Cache warming observes public art needs; it cannot broaden GPU residency. */
import { useCallback, useMemo, useRef } from "react";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { ObservationView } from "@web/battle/sim/observationView";
import { catalogLoadNames } from "@packages/scene-assets/src/schema";
import { gameRuntime, observedKinds } from "./gameAppearances";
import { prefetchRuntimeAssets } from "./runtimePrefetch";

export function useBattleWarmup(input: {
  scope: unknown;
  units: Pick<UnitCatalog, "cards" | "appearancesFor">;
  enemy: string | null;
  factions: ReadonlySet<string> | null;
  observation: ObservationView | null;
  onReady(): void;
}): () => void {
  const { scope, units, enemy, factions, observation, onReady } = input;
  const learned = useRef({ scope, kinds: new Set<string>() });
  if (learned.current.scope !== scope) learned.current = { scope, kinds: new Set() };
  for (const kind of observedKinds(observation)) learned.current.kinds.add(kind);
  const enemyKinds = useMemo(
    () =>
      units.cards
        .filter(
          (card) =>
            !card.disabled_reason && card.roster.factions.some((faction) => faction === enemy),
        )
        .map((card) => card.id),
    [units, enemy],
  );
  const learnedKey = [...learned.current.kinds].join("\0");
  const catalog = gameRuntime.catalog;
  const order = useMemo(() => {
    if (!catalog) return [];
    const kinds = new Set([...(learnedKey ? learnedKey.split("\0") : []), ...enemyKinds]);
    return [
      ...new Set(
        [...kinds].flatMap((kind) =>
          catalogLoadNames(catalog, units.appearancesFor([kind]), factions, null),
        ),
      ),
    ];
  }, [catalog, units, learnedKey, enemyKinds, factions]);
  const latest = useRef(order);
  latest.current = order;
  return useCallback(() => {
    onReady();
    if (enemy) void prefetchRuntimeAssets("/", () => latest.current);
  }, [onReady, enemy]);
}
