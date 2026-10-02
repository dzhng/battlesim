// The page's appearances, through the one loader (`AppearanceLibrary`), from
// what the bake wrote under `assets/runtime/` (Vite's publicDir, served at
// the site root). The catalog loads once per page: every appearance but the
// kits. A kit is fetched the first time the page asks for it, and a page asks
// for what it will draw: a map for the kits its buildings' templates place
// (`useMapAppearances`), a lab for the kit it shows. Every route that draws
// the world waits for its appearances, so its trees and buildings are there
// from the first frame. The workbench reloads the catalog after a re-bake.
import { useEffect, useMemo, useState } from "react";
import { AppearanceLibrary, type InstalledAppearances } from "@packages/scene-assets/src/loader";
import { PROTOTYPE_KIT } from "@packages/scene-assets/src/prototypeSet";
import { TemplateArtError, templateKits } from "@packages/scene-assets/src/templateLibrary";
import type { PlacedBuildings } from "@packages/battle-renderer/src/models/buildingReferences";
import type { MapProp, PropAppearances } from "@packages/battle-renderer/src/models/propAppearance";

const library = new AppearanceLibrary();
let loading: Promise<InstalledAppearances> | null = null;

/** The page's appearances with the kits `kits` among them: the catalog's one
 *  load, then whichever of those kits the page has not fetched yet. */
export function gameAppearances(kits: Iterable<string> = []): Promise<InstalledAppearances> {
  loading ??= library.load("/");
  return loading.then(() => library.withKits(kits));
}

/** Load the catalog afresh (it was re-baked): the page's appearances from now
 *  on. The new bake's kits are fetched when asked for again. */
export function reloadGameAppearances(): Promise<InstalledAppearances> {
  loading = null;
  return gameAppearances();
}

const NO_KITS: ReadonlySet<string> = new Set();

/** The installed appearances with `kits` among them, or null until they have
 *  loaded, and while `kits` is null (the page does not know yet what it
 *  draws). A failed load is an error on the console (scenes fail on it), and
 *  the world never draws. */
export function useGameAppearances(
  kits: ReadonlySet<string> | null = NO_KITS,
): InstalledAppearances | null {
  const [installed, setInstalled] = useState<InstalledAppearances | null>(null);
  useEffect(() => {
    if (!kits) return;
    let live = true;
    gameAppearances(kits).then(
      (next) => live && setInstalled(next),
      (error: unknown) => console.error("Appearances failed to load", error),
    );
    return () => {
      live = false;
    };
  }, [kits]);
  // What an earlier request installed answers this one only if it has its kits.
  const held = installed && kits && [...kits].every((kit) => installed.appearances.has(kit));
  return held ? installed : null;
}

/** The kits a map of `placed` buildings draws from: those its templates'
 *  rows place, in every state (`templateKits`). With `standIns`, also the
 *  prototype kit, whose box a prop with no art of its own is drawn as
 *  (`PropAppearances`). A map with buildings and a catalog
 *  with no template art for them is refused. */
export function mapKits(
  catalog: InstalledAppearances,
  placed: PlacedBuildings,
  standIns = false,
): Set<string> {
  const kits = new Set(standIns ? [PROTOTYPE_KIT] : []);
  if (placed.template.length === 0) return kits;
  if (!catalog.templates)
    throw new Error("the map has buildings and the catalog has no template art library; re-bake");
  for (const kit of templateKits(catalog.templates.library, placed.templates)) kits.add(kit);
  return kits;
}

/** The installed appearances with the kits of a map of `placed` buildings
 *  (`mapKits`), or null until they have loaded, and while `placed` is null
 *  (the map is not known yet). */
export function useMapAppearances(
  placed: PlacedBuildings | null,
  standIns = false,
): InstalledAppearances | null {
  const catalog = useGameAppearances();
  const kits = useMemo(
    () => catalog && placed && mapKits(catalog, placed, standIns),
    [catalog, placed, standIns],
  );
  return useGameAppearances(kits);
}

/**
 * What the models layer installs to draw a map: with `units`, every soldier
 * and vehicle; the appearance each of the map's props takes, and every wreck
 * and heap of rubble a battle can leave (`fit.drawnFor`); and, where the map
 * has buildings (`buildings`), the template art library and the kits those
 * buildings' templates draw from, no other. A kit of those that is not
 * installed refuses the map by name: no building is drawn without its art.
 * Trees, hedgerows and grass are the scenery layer's and the grass pass's,
 * which hold their own buffers.
 */
export function mapAppearances(
  appearances: InstalledAppearances,
  props: readonly MapProp[],
  fit: PropAppearances,
  buildings: PlacedBuildings,
  units: boolean,
): InstalledAppearances {
  const drawn = fit.drawnFor(props.filter((p) => !fit.drawsTree(p.kind)));
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
          (units && (a.unit === "soldier" || a.unit === "vehicle")) ||
          kits.has(name) ||
          drawn.has(name),
      ),
    ),
    templates: buildings.template.length > 0 ? appearances.templates : undefined,
  };
}
