// @vitest-environment node
// The Armory's own art residency: the yard and the one unit on show, built
// fresh per selection, with only the newest selection ever answered.
import { expect, test, vi } from "vitest";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { memoryFetch, type Fetch } from "@packages/scene-assets/src/loader.ts";
import { bundlePath, type RuntimeCatalog } from "@packages/scene-assets/src/schema.ts";
import { ArmoryAppearances } from "@apps/battle-lab/src/armory/armoryAppearances";
import { FACTIONS } from "@packages/scene-assets/src/units.ts";
import { AUTHORITY, syntheticUnits, testCatalog, testSources } from "./sceneAssets/synthetic";

vi.mock("@wasm/renderer_browser", () => ({ BrowserGpu: { new: vi.fn() } }));
vi.mock("@web/battle/sim/presentation", () => ({ loadPresentation: async () => {} }));
const { appResources } = await import("@apps/battle-lab/src/appResources");

const units = syntheticUnits();
/** The yard: every scenery appearance the catalog bakes, wrecks aside. */
const YARD: ReadonlySet<string> = new Set(
  Object.entries(testCatalog().appearances)
    .filter(([, e]) => e.unit === "scenery" && e.scenery !== "wreck")
    .map(([name]) => name),
);
const art = (kind: string) => [...units.appearancesFor([kind])];
const shown = (kind: string | null) => ({ kind, faction: FACTIONS[0], scenery: YARD });

/** The baked synthetic catalog served at the site root, each fetch counted
 *  by URL, and any URL `hold` names held until released. */
async function served() {
  const sources = testSources();
  const result = await bakeCatalog(testCatalog(), async (path) => sources[path], {
    authority: AUTHORITY,
  });
  const files = new Map<string, Uint8Array>([
    ["catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime))],
    ...result.files,
  ]);
  const inner = memoryFetch(files, "/");
  const fetched = new Map<string, number>();
  const held = new Map<string, Promise<void>>();
  const fetcher: Fetch = async (url) => {
    fetched.set(url, (fetched.get(url) ?? 0) + 1);
    await held.get(url);
    return inner(url);
  };
  const hold = (url: string) => {
    let release!: () => void;
    held.set(url, new Promise<void>((resolve) => (release = resolve)));
    return () => {
      held.delete(url);
      release();
    };
  };
  const runtime: RuntimeCatalog = result.runtime;
  /** The URL the bundle of appearance `name` is fetched from. */
  const bundleUrl = (name: string) =>
    `/${bundlePath(runtime.gzip![runtime.appearances[name].bundle].hash)}`;
  return { files, fetched, fetcher, hold, bundleUrl };
}

test("switching installs only the newest unit and the yard", async () => {
  const { fetcher } = await served();
  const armory = new ArmoryAppearances(units, fetcher);
  const [tank, truck] = [art("tank"), art("supply")];

  const first = await armory.show(shown("tank"));
  for (const name of [...YARD, ...tank]) expect(first.appearances.appearances.has(name)).toBe(true);
  for (const name of truck) expect(first.appearances.appearances.has(name)).toBe(false);

  const next = await armory.show(shown("supply"));
  for (const name of [...YARD, ...truck]) expect(next.appearances.appearances.has(name)).toBe(true);
  for (const name of tank) expect(next.appearances.appearances.has(name)).toBe(false);
  // The runtime files answer for the generation they installed.
  expect(next.runtime.catalog).not.toBeNull();

  const yard = await armory.show(shown(null));
  for (const name of [...tank, ...truck])
    expect(yard.appearances.appearances.has(name)).toBe(false);
  for (const name of YARD) expect(yard.appearances.appearances.has(name)).toBe(true);
});

test("a stale download never installs", async () => {
  const { fetched, fetcher, hold, bundleUrl } = await served();
  const armory = new ArmoryAppearances(units, fetcher);
  const tankBundle = bundleUrl(art("tank")[0]);
  const release = hold(tankBundle);

  const stale = armory.show(shown("tank"));
  const staleSettled = stale.then(
    () => "installed",
    (error: unknown) => (error instanceof Error ? error.name : String(error)),
  );
  // Let the tank's load reach its held bundle before the switch.
  await vi.waitFor(() => expect(fetched.get(tankBundle)).toBe(1));
  const newest = await armory.show(shown("supply"));
  release();

  expect(await staleSettled).toBe("AbortError");
  for (const name of art("tank")) expect(newest.appearances.appearances.has(name)).toBe(false);
  for (const name of art("supply")) expect(newest.appearances.appearances.has(name)).toBe(true);
});

test("a failing unit rejects and app resources stay healthy", async () => {
  const { files, fetcher, bundleUrl } = await served();
  const armory = new ArmoryAppearances(units, fetcher);
  const [truckArt] = art("supply");
  files.delete(bundleUrl(truckArt).slice(1));

  await expect(armory.show(shown("supply"))).rejects.toThrow(/HTTP 404/);
  expect(appResources.error()).toBeNull();
  // The next selection is unaffected.
  const tank = await armory.show(shown("tank"));
  for (const name of art("tank")) expect(tank.appearances.appearances.has(name)).toBe(true);
});

test("the yard is fetched once across switches", async () => {
  const { fetched, fetcher } = await served();
  const armory = new ArmoryAppearances(units, fetcher);
  await armory.show(shown(null));
  const yardFiles = [...fetched.keys()];
  expect(yardFiles).toContain("/catalog.json");

  for (const kind of ["tank", "supply", "rifle", "tank"]) await armory.show(shown(kind));
  for (const url of yardFiles) expect(fetched.get(url), url).toBe(1);
});
