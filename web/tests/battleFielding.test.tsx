import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { useBattleFielding, type BattleArt } from "@apps/battle-lab/src/useBattleFielding";
import type { fieldedBattle } from "@apps/battle-lab/src/presentedFrame";
import type { PresentedBattle } from "@web/battle/sim/presentation";

const fielding = vi.hoisted(() => ({
  resolve: null as null | ((battle: PresentedBattle) => void),
}));
// Canonical resolution crosses the asynchronous WASM/asset boundary.
vi.mock("@apps/battle-lab/src/presentedFrame", () => ({
  fieldedBattle: () => new Promise<PresentedBattle>((resolve) => (fielding.resolve = resolve)),
}));

afterEach(cleanup);

test("new art keeps the last complete battle drawing until matching topology and art are ready", async () => {
  const art = (generation: number): BattleArt => {
    const appearances: BattleArt["appearances"] = {
      generation,
      appearances: new Map(),
      onRequest: new Map(),
      skeletons: new Map(),
      sides: { blue: [1, 1, 1], red: [1, 1, 1] },
    };
    return { appearances, models: { ...appearances } };
  };
  const firstArt = art(1),
    secondArt = art(2);
  const firstBattle = { world: { first: true }, tables: new Map() } as unknown as PresentedBattle;
  const nextBattle = { world: { next: true }, tables: new Map() } as unknown as PresentedBattle;
  type Props = {
    scope: object;
    request: Parameters<typeof fieldedBattle>[0];
    art: BattleArt | null;
  };
  const props: Props = { scope: {}, request: {} as Props["request"], art: firstArt };
  const { result, rerender } = renderHook(
    (p: Props) => useBattleFielding(p.scope, p.request, p.art),
    { initialProps: props },
  );
  expect(result.current).toBeNull();
  await act(async () => fielding.resolve!(firstBattle));
  expect(result.current).toEqual({ ...firstArt, battle: firstBattle });
  const first = result.current;

  const next = { ...props, request: {} as Props["request"], art: null };
  rerender(next);
  expect(result.current).toBe(first);
  rerender({ ...next, art: secondArt });
  expect(result.current).toBe(first);
  await act(async () => fielding.resolve!(nextBattle));
  expect(result.current).toEqual({ ...secondArt, battle: nextBattle });

  // A map/observer/battle change retires the old generation immediately,
  // even if resolution from the departed scope finishes later.
  const lateRequest = { ...next, art: secondArt, request: {} as Props["request"] };
  rerender(lateRequest);
  const late = fielding.resolve!;
  const changedScope = { ...lateRequest, scope: {}, art: null };
  rerender(changedScope);
  expect(result.current).toBeNull();
  await act(async () => late(firstBattle));
  expect(result.current).toBeNull();
  rerender({ ...changedScope, art: secondArt });
  await act(async () => fielding.resolve!(nextBattle));
  expect(result.current).toEqual({ ...secondArt, battle: nextBattle });

  // Art-only refreshes retain the canonical battle object: no world reload.
  const refreshed = art(3);
  rerender({ ...changedScope, art: refreshed });
  expect(result.current).toEqual({ ...refreshed, battle: nextBattle });
  expect(result.current?.battle).toBe(nextBattle);
});
