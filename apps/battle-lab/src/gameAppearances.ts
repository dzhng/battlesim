// The page's appearances, through the one loader (`AppearanceLibrary`), from
// what the bake wrote under `assets/runtime/` (Vite's publicDir, served at
// the site root). A battle admits its requested units, their wrecks and the
// map's scenery, kits and regional looks. The page cache can retain other art;
// that cache never determines the battle's GPU residency. Routes wait for a
// complete source generation before exposing its presentation to the viewport.
// The workbench reloads the catalog after a re-bake.
import type { MapBuildings, StaticWorld } from "./useStaticWorld";
import { countedFetch } from "@web/downloads";
import { useEffect, useMemo, useRef, useState } from "react";
import { AppearanceLibrary, type InstalledAppearances } from "@packages/scene-assets/src/loader";
import {
  bundlePath,
  familyLooks,
  templateLibraryPath,
  texturePath,
  type RuntimeCatalog,
  catalogLoadNames,
} from "@packages/scene-assets/src/schema";
import { STAND_IN_KIT } from "@packages/scene-assets/src/standInKit";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { ObservationView } from "@web/battle/sim/observationView";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { TemplateArtError, templateKits } from "@packages/scene-assets/src/templateLibrary";
import type { PlacedBuildings } from "@packages/battle-renderer/src/models/buildingReferences";
import { propArt, mapProps } from "@packages/battle-renderer/src/models/propAppearance";
import type { WorldExports, WorldLayout } from "@packages/battle-renderer/src/worldMesh";
import { prop_appearances } from "@wasm/renderer_browser";
import { diagnosticsSource } from "@web/diagnostics";
import { appResources } from "./appResources";
import type { RuntimeFiles } from "./presentedFrame";

const library = new AppearanceLibrary(countedFetch);
/** The runtime files the page's appearances were loaded from: what Rust
 *  resolves a battle's units and effects from (`fieldedBattle`). */
export const gameRuntime: RuntimeFiles = library;
let loading: Promise<InstalledAppearances> | null = null;
let loadingFactions: string | null | undefined;
let loadingScenery: string | null | undefined;
let appearanceStatus: Record<string, unknown> = { state: "idle" };
let prefetching: Promise<void> | null = null;
let prefetchTimer: ReturnType<typeof setTimeout> | null = null;
let prefetchAllowed = false;
let prefetchStatus: Record<string, unknown> = { state: "idle" };
diagnosticsSource("appearance loading", () => ({ ...appearanceStatus, prefetch: prefetchStatus }));

function selectionKey(
  selection: ReadonlySet<string> | null | undefined,
): string | null | undefined {
  return selection === null ? null : selection && [...selection].sort().join("|");
}

/** Warm the browser's network/cache storage without decoding or uploading art.
 * The page cancels it as it leaves (`appResources.signal`): that is no failure,
 * where a fetch the network fails while the page stays is. */
export async function prefetchRuntimeAssets(baseUrl: string): Promise<void> {
  if (prefetching) return prefetching;
  const { signal } = appResources;
  prefetching = (async () => {
    prefetchStatus = { state: "loading" };
    const response = await fetch(`${baseUrl}catalog.json`, { cache: "force-cache", signal });
    if (!response.ok) throw new Error(`appearance prefetch catalog: HTTP ${response.status}`);
    const catalog = (await response.json()) as RuntimeCatalog;
    const files = new Set<string>();
    const add = (hash: string | undefined, path: (hash: string) => string) => {
      if (!hash) return;
      const transport = catalog.gzip?.[hash];
      if (transport) files.add(`${baseUrl}${path(transport.hash)}`);
    };
    for (const entry of Object.values(catalog.appearances ?? {})) add(entry.bundle, bundlePath);
    for (const ids of Object.values(catalog.textures ?? {}))
      for (const id of ids) add(id, texturePath);
    for (const hash of Object.values(catalog.skeletons ?? {})) add(hash, bundlePath);
    if (catalog.templates) add(catalog.templates.library, templateLibraryPath);
    const urls = [...files.keys()];
    const cache =
      typeof caches === "undefined"
        ? null
        : await caches.open("battle-runtime-v1").catch(() => null);
    let cursor = 0;
    let failed = 0;
    const worker = async () => {
      for (;;) {
        const url = urls[cursor++];
        if (!url) return;
        try {
          if (cache && (await cache.match(url))) continue;
          const file = await fetch(url, { cache: "force-cache", signal });
          if (!file.ok) throw new Error(`HTTP ${file.status}`);
          if (cache) await cache.put(url, file.clone());
          await file.arrayBuffer();
        } catch (error) {
          if (signal.aborted) return;
          failed++;
          console.error("Background appearance prefetch failed.", { url, error });
        }
      }
    };
    await Promise.all(Array.from({ length: Math.min(6, urls.length) }, () => worker()));
    if (cache && failed === 0 && !signal.aborted) {
      const expected = new Set(
        urls.map((url) => new URL(url, globalThis.location?.href ?? "http://localhost/").href),
      );
      for (const request of await cache.keys()) {
        if (!expected.has(request.url)) await cache.delete(request);
      }
    }
    prefetchStatus = signal.aborted
      ? { state: "cancelled" }
      : { state: failed ? "partial" : "ready", files: urls.length, failed };
  })()
    .catch((error) => {
      if (signal.aborted) {
        prefetchStatus = { state: "cancelled" };
        return;
      }
      prefetchStatus = {
        state: "failed",
        error: error instanceof Error ? error.message : String(error),
      };
      console.error("Background appearance prefetch failed.", error);
    })
    .then(() => undefined);
  return prefetching;
}

function scheduleRuntimePrefetch(baseUrl: string): void {
  if (prefetching || prefetchTimer !== null) return;
  prefetchStatus = { state: "scheduled" };
  // Let the first page settle before broadening the network/cache work beyond
  // the current view's required art.
  prefetchTimer = setTimeout(() => {
    prefetchTimer = null;
    void prefetchRuntimeAssets(baseUrl);
  }, 3_000);
}

/** Start warming unused art only after the first page has a stable graphics tier. */
export function allowRuntimePrefetch(): void {
  prefetchAllowed = true;
  scheduleRuntimePrefetch("/");
}

/** The page's appearances for a session running `units`, with `asked`
 *  among them: the catalog's one load, then whichever of their art and of
 *  those the page has not fetched yet. */
export function gameAppearances(
  units: UnitCatalog,
  asked: Iterable<string> = [],
  wearing: ReadonlySet<string> = units.appearances,
  factions?: ReadonlySet<string> | null,
  scenery?: ReadonlySet<string> | null,
): Promise<InstalledAppearances> {
  const factionsKey = selectionKey(factions);
  const sceneryKey = selectionKey(scenery);
  if (loading && (loadingFactions !== factionsKey || loadingScenery !== sceneryKey)) loading = null;
  loadingFactions = factionsKey;
  loadingScenery = sceneryKey;
  loading ??= library.load("/", wearing, factions, scenery);
  return loading
    .then(() => library.withUnits(wearing))
    .then(() => library.withAppearances(asked))
    .then((next) => {
      if (prefetchAllowed) scheduleRuntimePrefetch("/");
      return next;
    });
}

/** Load the catalog afresh (it was re-baked) for a session running `units`:
 *  the page's appearances from now on. The new bake's kits are fetched when
 *  asked for again. */
export function reloadGameAppearances(
  units: UnitCatalog,
  wearing: ReadonlySet<string> = units.appearances,
  factions?: ReadonlySet<string> | null,
  scenery?: ReadonlySet<string> | null,
): Promise<InstalledAppearances> {
  loading = null;
  loadingFactions = undefined;
  loadingScenery = undefined;
  prefetching = null;
  if (prefetchTimer !== null) {
    clearTimeout(prefetchTimer);
    prefetchTimer = null;
  }
  prefetchStatus = { state: "idle" };
  return gameAppearances(units, [], wearing, factions, scenery);
}

const NOTHING: ReadonlySet<string> = new Set();

/** The installed appearances for the session's units (`useSessionCatalog`)
 *  with `asked` among them, or null until they have loaded, and while `asked`
 *  is null (the page does not know yet what it draws). Required failures
 *  refuse the active view through the app boundary. */
export function useGameAppearances(
  asked: ReadonlySet<string> | null = NOTHING,
  kinds: Iterable<string> | null = null,
  factions?: ReadonlySet<string> | null,
  scenery?: ReadonlySet<string> | null,
  retainOnError = false,
): InstalledAppearances | null {
  const { units } = useSessionCatalog();
  const wearing = useMemo(
    () => (kinds ? units.appearancesFor(kinds) : units.appearances),
    [kinds, units],
  );
  const [installed, setInstalled] = useState<InstalledAppearances | null>(null);
  const installedRef = useRef(installed);
  installedRef.current = installed;
  useEffect(() => {
    if (!asked) return;
    let live = true;
    let retry: ReturnType<typeof setTimeout> | undefined;
    const request = () => {
      appearanceStatus = {
        state: "loading",
        requestedKinds: kinds ? [...kinds] : null,
        requestedAppearances: [...asked],
      };
      gameAppearances(units, asked, wearing, factions, scenery).then(
        (next) => {
          if (!live) return;
          appearanceStatus = { state: "ready", generation: next.generation };
          setInstalled(next);
        },
        (error: unknown) => {
          if (!live) return;
          if (retainOnError && installedRef.current) {
            // Keep the last complete generation drawing while a newly seen
            // unit's art is unavailable. Retry without interrupting the sim.
            appearanceStatus = {
              state: "retrying",
              requestedKinds: kinds ? [...kinds] : null,
              requestedAppearances: [...asked],
              error: error instanceof Error ? error.message : String(error),
              retryMs: 1_000,
            };
            console.error("Unit appearance load failed; retrying in the background.", error);
            retry = setTimeout(request, 1_000);
          } else {
            appearanceStatus = {
              state: "failed",
              error: error instanceof Error ? error.message : String(error),
            };
            console.error("Appearance load failed.", error);
            appResources.refuse(error);
          }
        },
      );
    };
    request();
    return () => {
      live = false;
      if (retry !== undefined) clearTimeout(retry);
    };
  }, [asked, kinds, units, wearing, factions, scenery, retainOnError]);
  // What an earlier request installed answers this one only if it has what it asked.
  const held =
    installed &&
    asked &&
    [...asked].every((name) => installed.appearances.has(name)) &&
    [...wearing].every((name) => installed.appearances.has(name));
  return retainOnError ? installed : held ? installed : null;
}

/** The kits a map of `placed` buildings draws from: those its templates'
 *  rows place, in every state (`templateKits`). With `standIns`, also the
 *  stand-in kit, whose box a prop with no art of its own is drawn as. A map
 *  with buildings and a catalog
 *  with no template art for them is refused. */
export function mapKits(
  catalog: InstalledAppearances,
  placed: PlacedBuildings,
  standIns = false,
): Set<string> {
  const kits = new Set(standIns ? [STAND_IN_KIT] : []);
  if (placed.template.length === 0) return kits;
  if (!catalog.templates)
    throw new Error("the map has buildings and the catalog has no template art library; re-bake");
  for (const kit of templateKits(catalog.templates.library, placed.templates)) kits.add(kit);
  return kits;
}

/** What a map of `placed` buildings in regional family `family` (null for a
 *  map of none) fetches on request: its kits (`mapKits`) and its family's
 *  regional looks, no other family's. */
export function mapDownloads(
  catalog: InstalledAppearances,
  placed: PlacedBuildings,
  family: string | null,
  standIns = false,
): Set<string> {
  const asked = mapKits(catalog, placed, standIns);
  for (const name of familyLooks(catalog.onRequest, family)) asked.add(name);
  return asked;
}

/** The installed appearances with what a map of `placed` buildings in
 *  `family` fetches (`mapDownloads`), or null until they have loaded, and
 *  while `placed` is null (the map is not known yet). */
export function useMapAppearances(
  placed: PlacedBuildings | null,
  family: string | null,
  standIns = false,
  kinds: Iterable<string> | null = null,
  factions?: ReadonlySet<string> | null,
  scenery?: ReadonlySet<string> | null,
): InstalledAppearances | null {
  const catalog = useGameAppearances(NOTHING, kinds, factions, scenery, true);
  const asked = useMemo(
    () => catalog && placed && mapDownloads(catalog, placed, family, standIns),
    [catalog, placed, family, standIns],
  );
  // Keep the complete generation on screen while map-specific art is fetched;
  // the viewport must not unmount and reset the player's camera for a
  // background appearance update.
  return useGameAppearances(asked, kinds, factions, scenery, true);
}

/**
 * What the models layer installs to draw `world`: the selected source generation
 * and its unit wreck dependencies; the appearance each of the map's props takes, and every wreck
 * and heap of rubble a battle can leave (the presentation's
 * `prop_appearances`); and, where the map has buildings (`buildings`), the
 * template art library and the kits those buildings' templates draw from, no
 * other. A kit of those that is not installed refuses the map by name: no
 * building is drawn without its art. The shared renderer owns the buffers
 * for trees, hedgerows and grass.
 */
export function mapAppearances(
  appearances: InstalledAppearances,
  world: { exports: WorldExports; layout: WorldLayout },
  buildings: PlacedBuildings,
  sources: ReadonlySet<string> | null,
  dynamicProps = true,
): InstalledAppearances {
  // The page cache can contain art from an earlier battle. Scope it before
  // canonical prop selection, which otherwise admits every cached wreck.
  if (sources)
    appearances = {
      ...appearances,
      appearances: new Map(
        [...appearances.appearances].filter(
          ([name]) => sources.has(name) || appearances.onRequest.has(name),
        ),
      ),
    };
  const art = propArt(appearances, world.exports.buildings.regionalFamily, dynamicProps);
  const drawn = new Set(
    JSON.parse(
      prop_appearances(JSON.stringify(world.layout), world.exports.props, art),
    ) as string[],
  );
  const kits = mapKits(appearances, buildings);
  // Unit topology includes its possible wreck even on maps without dynamic props.
  const wrecks = new Set(
    sources ? [...appearances.appearances.values()].flatMap((entry) => entry.wreck ?? []) : [],
  );
  for (const kit of kits)
    if (!appearances.appearances.has(kit))
      throw new TemplateArtError(
        "kit.missing",
        `kit "${kit}" is not installed, and the map's buildings draw from it`,
      );
  return {
    ...appearances,
    appearances: new Map(
      [...appearances.appearances].filter(
        ([name, a]) =>
          (sources?.has(name) && (a.unit === "soldier" || a.unit === "vehicle")) ||
          wrecks.has(name) ||
          kits.has(name) ||
          drawn.has(name),
      ),
    ),
    templates: buildings.template.length > 0 ? appearances.templates : undefined,
  };
}

/** A battle keeps art it has learned, and preloads its player's purchase choices. */
export function useBattleKinds(input: {
  scope: unknown;
  side: string;
  units: Pick<UnitCatalog, "cards">;
  fielded: readonly string[];
  faction: string | null;
  observation: ObservationView | null;
  purchasing: string | null;
}): readonly string[] {
  const { scope, side, units, fielded, faction, observation, purchasing } = input;
  const seed = useMemo(
    () =>
      new Set([
        ...fielded,
        ...units.cards
          .filter(
            (card) => !card.disabled_reason && card.roster.factions.some((f) => f === faction),
          )
          .map((card) => card.id),
      ]),
    [units, fielded, faction],
  );
  const history = useRef<{
    scope: unknown;
    side: string;
    seed: Set<string>;
    kinds: Set<string>;
  } | null>(null);
  if (
    !history.current ||
    history.current.scope !== scope ||
    history.current.side !== side ||
    history.current.seed !== seed
  )
    history.current = { scope, side, seed, kinds: new Set(seed) };
  const retained = history.current.kinds;
  const key = useMemo(() => {
    for (const unit of [
      ...(observation?.own ?? []),
      ...(observation?.identified ?? []),
      ...(observation?.corpses ?? []),
      ...(observation?.crashes ?? []),
      ...(observation?.airTransits ?? []),
    ])
      retained.add(unit.kind);
    for (const prop of observation?.knownProps ?? []) if (prop.wreckOf) retained.add(prop.wreckOf);
    for (const contact of observation?.contacts ?? []) if (contact.kind) retained.add(contact.kind);
    for (const pending of observation?.skirmish?.players.flatMap((player) => player.pending) ?? [])
      retained.add(pending.kind);
    if (purchasing) retained.add(purchasing);
    return [...retained].sort().join("\0");
  }, [retained, observation, purchasing]);
  return useMemo(() => (key ? key.split("\0") : []), [key]);
}

/** Keep cached art separate from the complete source generation this battle admits. */
export function useBattleAppearances(input: {
  world: StaticWorld | null;
  buildings: MapBuildings | null;
  units: UnitCatalog;
  kinds: readonly string[];
  skirmish: boolean;
  factions: ReadonlySet<string> | null;
  scenery: ReadonlySet<string> | null | undefined;
}) {
  const { world, buildings, units, kinds, skirmish, factions, scenery } = input;
  const selectedKinds = useMemo(
    () => [
      ...new Set([
        ...kinds,
        ...(world
          ? mapProps(world.exports, world.layout).flatMap((prop) => prop.wreckOf ?? [])
          : []),
      ]),
    ],
    [kinds, world],
  );
  const appearances = useMapAppearances(
    buildings?.index.placed ?? null,
    world?.exports.buildings.regionalFamily ?? null,
    (world?.exports.props.length ?? 0) > 0 || (buildings?.index.placed.template.length ?? 0) > 0,
    selectedKinds,
    factions,
    scenery,
  );
  const sources = useMemo(() => {
    if (!appearances || !library.catalog) return null;
    return new Set(
      catalogLoadNames(library.catalog, units.appearancesFor(selectedKinds), factions, scenery),
    );
  }, [appearances, selectedKinds, units, factions, scenery]);
  const ready = !!(
    appearances &&
    sources &&
    world &&
    buildings &&
    [
      ...sources,
      ...mapDownloads(
        appearances,
        buildings.index.placed,
        world.exports.buildings.regionalFamily,
        world.exports.props.length > 0 || buildings.index.placed.template.length > 0,
      ),
    ].every((name) => appearances.appearances.has(name))
  );
  const models = useMemo(
    () =>
      ready && appearances && world && buildings
        ? mapAppearances(
            appearances,
            world,
            buildings.index.placed,
            sources,
            skirmish || world.exports.forests.length > 0,
          )
        : null,
    [ready, appearances, world, buildings, sources, skirmish],
  );
  return useMemo(
    () => (appearances && models ? { appearances, models } : null),
    [appearances, models],
  );
}
