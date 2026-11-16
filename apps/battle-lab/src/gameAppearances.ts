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
import { familyLooks, catalogLoadNames } from "@packages/scene-assets/src/schema";
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
import { prefetchStatus, scheduleRuntimePrefetch, resetRuntimePrefetch } from "./runtimePrefetch";

const library = new AppearanceLibrary(countedFetch);
/** The appearance library is also the runtime source fielding reads. */
export const gameRuntime = library;
let loading: Promise<InstalledAppearances> | null = null;
let loadingFactions: string | null | undefined;
let loadingScenery: string | null | undefined;
let appearanceStatus: Record<string, unknown> = { state: "idle" };
diagnosticsSource("appearance loading", () => ({ ...appearanceStatus, prefetch: prefetchStatus }));

function selectionKey(
  selection: ReadonlySet<string> | null | undefined,
): string | null | undefined {
  return selection === null ? null : selection && [...selection].sort().join("|");
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
    .then(() => library.withUnits(wearing, asked))
    .then((next) => {
      scheduleRuntimePrefetch("/");
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
  resetRuntimePrefetch();
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

/** The kinds an observation shows its side: its units, what it has identified
 *  or holds as a contact, and the wrecks and bodies it knows. */
export function* observedKinds(observation: ObservationView | null): Generator<string> {
  for (const unit of [
    ...(observation?.own ?? []),
    ...(observation?.identified ?? []),
    ...(observation?.corpses ?? []),
    ...(observation?.crashes ?? []),
    ...(observation?.airTransits ?? []),
  ])
    yield unit.kind;
  for (const prop of observation?.knownProps ?? []) if (prop.wreckOf) yield prop.wreckOf;
  for (const contact of observation?.contacts ?? []) if (contact.kind) yield contact.kind;
}

/** A battle keeps art it has learned, and preloads its player's purchase choices.
 * Kinds wearing only that art are fielded together: learning another description
 * for an already wanted look must not stage the same catalog again. */
export function useBattleKinds(input: {
  scope: unknown;
  side: string;
  units: Pick<UnitCatalog, "cards" | "ids" | "appearancesFor">;
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
  const learned = useMemo(() => {
    for (const kind of observedKinds(observation)) retained.add(kind);
    for (const pending of observation?.skirmish?.players.flatMap((player) => player.pending) ?? [])
      for (const kind of [pending.kind, pending.carrier]) if (kind) retained.add(kind);
    if (purchasing) retained.add(purchasing);
    return [...retained].sort().join("\0");
  }, [retained, observation, purchasing]);
  const requirements = useMemo(
    () => units.ids.map((kind) => ({ kind, names: units.appearancesFor([kind]) })),
    [units],
  );
  const key = useMemo(() => {
    const wanted = units.appearancesFor(learned ? learned.split("\0") : []);
    return requirements
      .filter(({ names }) => [...names].every((name) => wanted.has(name)))
      .map(({ kind }) => kind)
      .sort()
      .join("\0");
  }, [learned, requirements, units]);
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
