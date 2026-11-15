import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import type { RuntimeCatalog } from "@packages/scene-assets/src/schema";
import type { UnitCard } from "@packages/scene-assets/src/units";
import type { ObservationView } from "@web/battle/sim/observationView";

const card = (id: string, faction: UnitCard["roster"]["factions"][number]): UnitCard => ({
  id,
  name: id,
  description: "",
  family: "test",
  cost: 1,
  roster: { factions: [faction], family_name: "Test", category: "veh", variant: id },
  disabled_reason: null,
  planned_weapons: [],
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.resetModules();
});

test("a direct battle warms learned art, the public enemy and the rest only after a presented frame, without wanting it", async () => {
  const asked: string[] = [];
  const names = ["rest", "enemy", "learned"];
  const catalog: RuntimeCatalog = {
    appearances: Object.fromEntries(
      names.map((name) => [
        name,
        { kind: "articulated" as const, unit: "vehicle" as const, bundle: name },
      ]),
    ),
    skeletons: {},
    sides: { blue: [0, 0, 1], red: [1, 0, 0] },
    gzip: Object.fromEntries(names.map((hash) => [hash, { hash, bytes: 1, raw_bytes: 1 }])),
  };
  const fetch = vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url.endsWith("catalog.json")) return Response.json(catalog);
    asked.push(url);
    return new Response(new Uint8Array([1]));
  });
  vi.stubGlobal("fetch", fetch);
  const { useBattleWarmup } = await import("@apps/battle-lab/src/useBattleWarmup");
  const { gameRuntime, useBattleKinds } = await import("@apps/battle-lab/src/gameAppearances");
  vi.spyOn(gameRuntime, "catalog", "get").mockReturnValue(catalog);
  const units = {
    ids: names,
    cards: [card("enemy", "eastern")],
    appearancesFor: (kinds: Iterable<string>) => new Set(kinds),
  };
  const scope = {};
  const observation = { identified: [{ kind: "learned" }] } as ObservationView;
  let frames = 0;
  const hook = renderHook(() => {
    const presented = useBattleWarmup({
      scope,
      units,
      enemy: "eastern",
      factions: null,
      observation,
      onReady: () => {
        frames++;
      },
    });
    const wanted = useBattleKinds({
      scope,
      side: "blue",
      units,
      fielded: [],
      faction: null,
      observation,
      purchasing: null,
    });
    return { presented, wanted };
  });
  expect(fetch).not.toHaveBeenCalled();
  expect(asked).toEqual([]);
  expect(hook.result.current.wanted).toEqual(["learned"]);
  act(() => hook.result.current.presented());
  await waitFor(() =>
    expect(asked).toEqual(["/learned/bundle.bin", "/enemy/bundle.bin", "/rest/bundle.bin"]),
  );
  expect(frames).toBe(1);
  expect(hook.result.current.wanted).toEqual(["learned"]);
});

test("a newly learned type takes the next unstarted download, even after its contact expires", async () => {
  const names = Array.from({ length: 10 }, (_, i) => `test-${i}`);
  const catalog: RuntimeCatalog = {
    appearances: Object.fromEntries(
      names.map((name) => [
        name,
        { unit: "vehicle" as const, kind: "articulated" as const, bundle: name },
      ]),
    ),
    skeletons: {},
    sides: { blue: [0, 0, 1], red: [1, 0, 0] },
    gzip: Object.fromEntries(names.map((hash) => [hash, { hash, bytes: 1, raw_bytes: 1 }])),
  };
  const asked: string[] = [],
    finish: (() => void)[] = [];
  vi.stubGlobal("fetch", async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url.endsWith("catalog.json")) return Response.json(catalog);
    asked.push(url);
    return new Promise<Response>((resolve) =>
      finish.push(() => resolve(new Response(new Uint8Array([1])))),
    );
  });
  const { useBattleWarmup } = await import("@apps/battle-lab/src/useBattleWarmup");
  const { gameRuntime } = await import("@apps/battle-lab/src/gameAppearances");
  vi.spyOn(gameRuntime, "catalog", "get").mockReturnValue(catalog);
  const units = { cards: [], appearancesFor: (kinds: Iterable<string>) => new Set(kinds) };
  const scope = {};
  const hook = renderHook(
    ({ observation }: { observation: ObservationView | null }) =>
      useBattleWarmup({ scope, units, enemy: "public", factions: null, observation, onReady() {} }),
    { initialProps: { observation: null as ObservationView | null } },
  );
  act(() => hook.result.current());
  await waitFor(() => expect(asked).toHaveLength(6));
  const learned = names.at(-1)!;
  hook.rerender({ observation: { identified: [{ kind: learned }] } as ObservationView });
  hook.rerender({ observation: null });
  await act(async () => finish.shift()!());
  await waitFor(() => expect(asked[6]).toBe(`/${learned}/bundle.bin`));
  // Drain the real downloader without releasing a production-owned cache.
  while (asked.length < names.length || finish.length) {
    await act(async () => {
      for (const done of finish.splice(0)) done();
    });
  }
  expect([...new Set(asked)].sort()).toEqual(names.map((name) => `/${name}/bundle.bin`).sort());
  expect(asked.length).toBe(names.length);
});

test("an encounter without a public enemy still reports readiness without warming the game catalog", async () => {
  const fetch = vi.fn();
  vi.stubGlobal("fetch", fetch);
  const { useBattleWarmup } = await import("@apps/battle-lab/src/useBattleWarmup");
  let ready = false;
  const hook = renderHook(() =>
    useBattleWarmup({
      scope: {},
      units: { cards: [], appearancesFor: () => new Set() },
      enemy: null,
      factions: null,
      observation: null,
      onReady() {
        ready = true;
      },
    }),
  );
  act(() => hook.result.current());
  expect(ready).toBe(true);
  expect(fetch).not.toHaveBeenCalled();
});
