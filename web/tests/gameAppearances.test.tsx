import { renderInRouter as render } from "./support/router";
import { cleanup, fireEvent, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import type { UnitCard } from "@packages/scene-assets/src/units";
import type { ObservationView } from "@web/battle/sim/observationView";
import { afterEach, expect, test, vi } from "vitest";
import { TEST_CATALOG } from "./catalog";

/** A view under the test set, through the context module the freshly
 *  imported page modules read (`vi.resetModules` gives each test its own). */
async function underTestCatalog() {
  const { SessionCatalogProvider } = await import("@web/battle/catalog/context");
  return ({ children }: { children: ReactNode }) => (
    <SessionCatalogProvider catalog={TEST_CATALOG}>{children}</SessionCatalogProvider>
  );
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  vi.resetModules();
});

test("the player's enabled purchases are ready before any unit enters the battle", async () => {
  const { useBattleKinds } = await import("@apps/battle-lab/src/gameAppearances");
  const card = (
    id: string,
    faction: "us" | "eastern",
    disabled_reason: string | null,
  ): UnitCard => ({
    id,
    name: id,
    description: "",
    family: "test",
    cost: 1,
    roster: { factions: [faction], family_name: "Test", category: "veh", variant: id },
    disabled_reason,
    planned_weapons: [],
  });
  const input = {
    scope: {},
    side: "blue",
    units: {
      cards: [
        card("buyable", "us", null),
        card("unavailable", "us", "planned"),
        card("enemy", "eastern", null),
      ],
    },
    fielded: ["scenario-unit"],
    faction: "us",
    observation: null,
    purchasing: null,
  };
  const { result } = renderHook(() => useBattleKinds(input));
  expect(result.current).toEqual(["buyable", "scenario-unit"]);
});

test("learned and fallen unit art stays until the battle or observing side changes", async () => {
  const { useBattleKinds } = await import("@apps/battle-lab/src/gameAppearances");
  const observation: ObservationView = {
    tick: 1,
    own: [],
    identified: [],
    contacts: [
      {
        id: 1,
        source: "last_seen",
        center: [0, 0, 0],
        layer: "ground",
        aloft: false,
        radius: 1,
        evidenceTick: 1,
        expiresTick: 2,
        kind: "remembered",
        heard: [],
      },
    ],
    corpses: [{ kind: "fallen", position: [0, 0, 0], own: false, soldier: 1, slot: 0, yaw: 0 }],
    audible: [],
    refusals: [],
    knownProps: [],
    projectiles: [],
    blasts: [],
    airTransits: [],
    fallenBodies: [],
    propDisplayPoses: [],
    grinds: [],
    crashes: [],
    shouts: [],
    guided: [],
    encounter: null,
    skirmish: null,
    fog: { cellM: 1, nx: 1, ny: 1, bits: new Uint32Array(1) },
    groundPatch: { epoch: 1, side: "blue", baseRevision: 0, revision: 1, full: true },
  };
  const input = {
    scope: {},
    side: "blue",
    units: { cards: [] },
    fielded: ["initial"],
    faction: null,
    observation: observation as ObservationView | null,
    purchasing: null,
  };
  const hook = renderHook((props) => useBattleKinds(props), { initialProps: input });
  expect(hook.result.current).toEqual(["fallen", "initial", "remembered"]);
  hook.rerender({ ...input, observation: null });
  expect(hook.result.current).toEqual(["fallen", "initial", "remembered"]);
  hook.rerender({ ...input, side: "red", observation: null });
  expect(hook.result.current).toEqual(["initial"]);
  hook.rerender({ ...input, scope: {}, observation: null });
  expect(hook.result.current).toEqual(["initial"]);
});

test("required appearance rejection remains fatal on the menu", async () => {
  vi.stubGlobal("fetch", async () => {
    throw new Error("Required appearance unavailable");
  });
  const { AppResourceBoundary } = await import("@apps/battle-lab/src/AppResourceBoundary");
  const { useGameAppearances } = await import("@apps/battle-lab/src/gameAppearances");
  const Scope = await underTestCatalog();
  function World() {
    const appearances = useGameAppearances();
    return <div>{appearances ? "Battlefield ready" : "Loading appearance"}</div>;
  }
  const view = render(
    <AppResourceBoundary menu={false}>
      <Scope>
        <World />
      </Scope>
    </AppResourceBoundary>,
  );
  await waitFor(() => expect(view.getByRole("heading", { name: "Aborted" })).toBeTruthy());
  expect(view.queryByText("Loading appearance")).toBeNull();
  expect(view.getByRole("link", { name: "Back to the menu" }).getAttribute("href")).toBe("/");
  view.rerender(
    <AppResourceBoundary menu={true}>
      <div>Main menu remains available</div>
    </AppResourceBoundary>,
  );
  expect(view.queryByText("Main menu remains available")).toBeNull();
  expect(view.getByRole("heading", { name: "Aborted" })).toBeTruthy();
});

test("a demanded building kit refuses play even after the shared catalog loaded", async () => {
  vi.stubGlobal("fetch", async () =>
    Response.json({ appearances: {}, skeletons: {}, sides: { blue: [0, 0, 1], red: [1, 0, 0] } }),
  );
  const { AppResourceBoundary } = await import("@apps/battle-lab/src/AppResourceBoundary");
  const { gameAppearances, useGameAppearances } =
    await import("@apps/battle-lab/src/gameAppearances");
  const base = await gameAppearances(TEST_CATALOG.units);
  expect(base.appearances.size).toBe(0);
  const kits = new Set(["required-building-kit"]);
  const Scope = await underTestCatalog();
  function World() {
    useGameAppearances(kits);
    return <div>Loading required kit</div>;
  }
  const view = render(
    <AppResourceBoundary menu={false}>
      <Scope>
        <World />
      </Scope>
    </AppResourceBoundary>,
  );
  await waitFor(() => expect(view.getByRole("heading", { name: "Aborted" })).toBeTruthy());
  expect(view.queryByText("Loading required kit")).toBeNull();
});

/** A catalog whose only files are two textures the background prefetch warms,
 *  and those files' fetch: `file` answers each one. */
function prefetchServer(file: (url: string, init?: RequestInit) => Promise<Response>) {
  const asked: string[] = [];
  vi.stubGlobal("fetch", async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    if (url.endsWith("catalog.json"))
      return Response.json({
        appearances: {},
        skeletons: {},
        sides: { blue: [0, 0, 1], red: [1, 0, 0] },
        textures: { "test-look": ["texture-a", "texture-b"] },
        gzip: {
          "texture-a": { hash: "gzip-a", bytes: 1, raw_bytes: 1 },
          "texture-b": { hash: "gzip-b", bytes: 1, raw_bytes: 1 },
        },
      });
    asked.push(url);
    return file(url, init);
  });
  return asked;
}

const PREFETCH_FAILED = "Background appearance prefetch failed.";
const prefetchErrors = (spy: { mock: { calls: unknown[][] } }) =>
  spy.mock.calls.filter(([message]) => message === PREFETCH_FAILED);

test("a prefetch the page cuts off as it leaves is not an error", async () => {
  const errors = vi.spyOn(console, "error").mockImplementation(() => {});
  // The browser ends a leaving document's fetches; one the page has not
  // cancelled itself fails as a network error.
  const asked = prefetchServer(
    (_url, init) =>
      new Promise((_, reject) => {
        const cut = () => reject(new TypeError("Failed to fetch"));
        init?.signal?.addEventListener("abort", () => reject(init.signal!.reason));
        window.addEventListener("pagehide", cut, { once: true });
      }),
  );
  const { AppResourceBoundary } = await import("@apps/battle-lab/src/AppResourceBoundary");
  const { gameAppearances, prefetchRuntimeAssets } =
    await import("@apps/battle-lab/src/gameAppearances");
  render(
    <AppResourceBoundary menu={false}>
      <div>Main menu</div>
    </AppResourceBoundary>,
  );
  await gameAppearances(TEST_CATALOG.units);
  void prefetchRuntimeAssets("/");
  await waitFor(() => expect(asked.length).toBe(2));
  fireEvent(window, new Event("pagehide"));
  await new Promise((resolve) => setTimeout(resolve, 10));
  expect(prefetchErrors(errors)).toEqual([]);
});

test("a prefetch the network fails while the page stays is still an error", async () => {
  const errors = vi.spyOn(console, "error").mockImplementation(() => {});
  prefetchServer(async () => {
    throw new TypeError("Failed to fetch");
  });
  const { AppResourceBoundary } = await import("@apps/battle-lab/src/AppResourceBoundary");
  const { gameAppearances, prefetchRuntimeAssets } =
    await import("@apps/battle-lab/src/gameAppearances");
  render(
    <AppResourceBoundary menu={false}>
      <div>Main menu</div>
    </AppResourceBoundary>,
  );
  await gameAppearances(TEST_CATALOG.units);
  await prefetchRuntimeAssets("/");
  await waitFor(() =>
    expect(prefetchErrors(errors).map(([, detail]) => (detail as { url: string }).url)).toEqual([
      "/gzip-a/texture.bin",
      "/gzip-b/texture.bin",
    ]),
  );
});

test("a successful prefetch removes obsolete runtime cache entries", async () => {
  const asked = prefetchServer(async () => new Response(new Uint8Array([1])));
  const entries = new Map<string, Response>([
    [new URL("/old/texture.bin", window.location.href).href, new Response("old")],
  ]);
  const key = (request: RequestInfo | URL) =>
    new URL(request instanceof Request ? request.url : String(request), window.location.href).href;
  const cache = {
    match: async (request: RequestInfo | URL) => entries.get(key(request)) ?? undefined,
    put: async (request: RequestInfo | URL, response: Response) => {
      entries.set(key(request), response);
    },
    keys: async () => [...entries.keys()].map((url) => new Request(url)),
    delete: async (request: RequestInfo | URL) => entries.delete(key(request)),
  };
  vi.stubGlobal("caches", { open: async () => cache });
  const { prefetchRuntimeAssets } = await import("@apps/battle-lab/src/gameAppearances");
  await prefetchRuntimeAssets("/");
  expect(asked).toEqual(["/gzip-a/texture.bin", "/gzip-b/texture.bin"]);
  expect([...entries.keys()].sort()).toEqual([
    new URL("/gzip-a/texture.bin", window.location.href).href,
    new URL("/gzip-b/texture.bin", window.location.href).href,
  ]);
});

test("document departure releases the active view and restored documents require manual reload", async () => {
  const { AppResourceBoundary } = await import("@apps/battle-lab/src/AppResourceBoundary");
  const view = render(
    <AppResourceBoundary menu={false}>
      <div>Active battle</div>
    </AppResourceBoundary>,
  );
  fireEvent(window, new Event("pagehide"));
  fireEvent(window, new Event("pageshow"));
  expect(view.queryByText("Active battle")).toBeNull();
  expect(view.getByRole("heading", { name: "Aborted" })).toBeTruthy();
  expect(view.getByRole("button", { name: "Reload" }).closest("main")).toBe(
    view.getByTestId("loading"),
  );
});

test("app boundary effect refresh leaves page resources usable until pagehide", async () => {
  const { AppResourceBoundary } = await import("@apps/battle-lab/src/AppResourceBoundary");
  const first = render(
    <AppResourceBoundary menu={false}>
      <div>First view</div>
    </AppResourceBoundary>,
  );
  first.unmount();
  const next = render(
    <AppResourceBoundary menu={false}>
      <div>Fresh view</div>
    </AppResourceBoundary>,
  );
  expect(next.getByText("Fresh view")).toBeTruthy();
  fireEvent(window, new Event("pagehide"));
  expect(next.queryByText("Fresh view")).toBeNull();
  expect(next.getByRole("heading", { name: "Aborted" })).toBeTruthy();
});
