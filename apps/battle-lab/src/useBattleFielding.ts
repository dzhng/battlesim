import { useEffect, useRef, useState } from "react";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { PresentedBattle } from "@web/battle/sim/presentation";
import { fieldedBattle } from "./presentedFrame";

export interface BattleArt {
  appearances: InstalledAppearances;
  models: InstalledAppearances;
}

/** Admit canonical topology and its complete art as one battle generation. */
export function useBattleFielding(
  scope: object,
  request: Parameters<typeof fieldedBattle>[0],
  art: BattleArt | null,
): (BattleArt & { battle: PresentedBattle }) | null {
  const [resolved, setResolved] = useState<{
    scope: object;
    request: typeof request;
    result: { battle: PresentedBattle } | { error: unknown };
  } | null>(null);
  const ready = art !== null;
  const admitted = useRef<{
    scope: object;
    generation: BattleArt & { battle: PresentedBattle };
    art: BattleArt;
  } | null>(null);
  if (admitted.current?.scope !== scope) admitted.current = null;
  useEffect(() => {
    if (!ready) return;
    let live = true;
    fieldedBattle(request).then(
      (battle) => live && setResolved({ scope, request, result: { battle } }),
      (error: unknown) => live && setResolved({ scope, request, result: { error } }),
    );
    return () => {
      live = false;
    };
  }, [scope, request, ready]);
  if (resolved?.scope === scope && resolved.request === request) {
    if ("error" in resolved.result) throw resolved.result.error;
    if (
      art &&
      (admitted.current?.art !== art ||
        admitted.current.generation.battle !== resolved.result.battle)
    )
      admitted.current = { scope, art, generation: { ...art, battle: resolved.result.battle } };
  }
  return admitted.current?.generation ?? null;
}
