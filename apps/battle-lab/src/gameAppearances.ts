// The page's appearances, through the one loader (`AppearanceLibrary`), from
// what the bake wrote under `assets/runtime/` (Vite's publicDir, served at
// the site root). A battle installs the complete unit-art catalog with its
// scenery at map startup, so selecting a reinforcement never has to decode or
// upload a new model during play. The browser warms the remaining files into
// network/cache storage without uploading them. Kits and regional
// looks are fetched on request. One of those is fetched the
// first time the page asks for it, and a page asks for what it will draw: a
// map for the kits its buildings' templates place and its region's looks
// (`useMapAppearances`), a lab for the kit it shows. Every route that draws
// the world waits for its appearances, so its trees and buildings are there
// from the first frame. The workbench reloads the catalog after a re-bake.
import { countedFetch } from "@web/downloads";
import { useEffect, useMemo, useRef, useState } from "react";
import { AppearanceLibrary, type InstalledAppearances } from "@packages/scene-assets/src/loader";
import {
  bundlePath,
  familyLooks,
  templateLibraryPath,
  texturePath,
  type RuntimeCatalog,
} from "@packages/scene-assets/src/schema";
import { STAND_IN_KIT } from "@packages/scene-assets/src/standInKit";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { TemplateArtError, templateKits } from "@packages/scene-assets/src/templateLibrary";
import type { PlacedBuildings } from "@packages/battle-renderer/src/models/buildingReferences";
import { propArt } from "@packages/battle-renderer/src/models/propAppearance";
import type { WorldExports, WorldLayout } from "@packages/battle-renderer/src/worldMesh";
import { prop_appearances } from "@wasm/presentation/presentation_wasm.js";
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
async function prefetchRuntimeAssets(baseUrl: string): Promise<void> {
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
      void prefetchRuntimeAssets("/");
      return next;
    });
}

/** Load the catalog afresh (it was re-baked) for a session running `units`:
 *  the page's appearances from now on. The new bake's kits are fetched when
 *  asked for again. */
export function reloadGameAppearances(units: UnitCatalog): Promise<InstalledAppearances> {
  loading = null;
  loadingFactions = undefined;
  loadingScenery = undefined;
  prefetching = null;
  prefetchStatus = { state: "idle" };
  return gameAppearances(units);
}

const NOTHING: ReadonlySet<string> = new Set();

/** Unit art needed by the renderer for a set of worn base appearances,
 * including faction looks and vehicle wrecks that the catalog links from it. */
export function installedUnitNames(
  installed: InstalledAppearances,
  worn: ReadonlySet<string>,
): ReadonlySet<string> {
  const names = new Set(worn);
  for (const name of worn) {
    const entry = installed.appearances.get(name);
    if (!entry) continue;
    if (entry.wreck) names.add(entry.wreck);
    for (const look of Object.values(entry.factions ?? {})) names.add(look);
  }
  return names;
}

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
 * What the models layer installs to draw `world`: with `units`, every soldier
 * and vehicle; the appearance each of the map's props takes, and every wreck
 * and heap of rubble a battle can leave (the presentation's
 * `prop_appearances`); and, where the map has buildings (`buildings`), the
 * template art library and the kits those buildings' templates draw from, no
 * other. A kit of those that is not installed refuses the map by name: no
 * building is drawn without its art. Trees, hedgerows and grass are the
 * scenery layer's and the grass pass's, which hold their own buffers.
 */
export function mapAppearances(
  appearances: InstalledAppearances,
  world: { exports: WorldExports; layout: WorldLayout },
  buildings: PlacedBuildings,
  units: boolean,
  unitNames: ReadonlySet<string> | null = null,
  dynamicProps = true,
): InstalledAppearances {
  const art = propArt(appearances, world.exports.buildings.regionalFamily, dynamicProps);
  const drawn = new Set(
    JSON.parse(
      prop_appearances(JSON.stringify(world.layout), world.exports.props, art),
    ) as string[],
  );
  const kits = mapKits(appearances, buildings);
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
          (units &&
            (a.unit === "soldier" || a.unit === "vehicle") &&
            (!unitNames || unitNames.has(name))) ||
          kits.has(name) ||
          drawn.has(name),
      ),
    ),
    templates: buildings.template.length > 0 ? appearances.templates : undefined,
  };
}
