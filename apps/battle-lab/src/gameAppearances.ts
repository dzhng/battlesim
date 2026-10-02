// The battle's appearances, through the one loader (`AppearanceLibrary`):
// every catalog entry the bake wrote under `assets/runtime/` (Vite's
// publicDir, served at the site root). Loaded once per page; every route
// that draws the world waits for it, so its trees are there from
// the first frame. The workbench reloads it after a re-bake.
import { useEffect, useState } from "react";
import { AppearanceLibrary, type InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { BuildingIndex } from "@packages/battle-renderer/src/models/buildingReferences";
import type { MapProp, PropAppearances } from "@packages/battle-renderer/src/models/propAppearance";

let loading: Promise<InstalledAppearances> | null = null;

export function gameAppearances(): Promise<InstalledAppearances> {
  loading ??= new AppearanceLibrary().load("/");
  return loading;
}

/** Load the catalog afresh (it was re-baked): the page's appearances from now on. */
export function reloadGameAppearances(): Promise<InstalledAppearances> {
  loading = null;
  return gameAppearances();
}

/** The installed appearances, or null until they load. A failed load is an
 *  error on the console (scenes fail on it), and the world never draws. */
export function useGameAppearances(): InstalledAppearances | null {
  const [installed, setInstalled] = useState<InstalledAppearances | null>(null);
  useEffect(() => {
    let live = true;
    gameAppearances().then(
      (next) => live && setInstalled(next),
      (error: unknown) => console.error("Appearances failed to load", error),
    );
    return () => {
      live = false;
    };
  }, []);
  return installed;
}

/**
 * What the models layer installs to draw a map: with `units`, every soldier
 * and vehicle; the appearance each of the map's props takes, and every wreck
 * and ruin a battle can leave (`fit.drawnFor`); and, where the map has
 * buildings drawn from template art (`buildings`), the art library and its
 * kits. Trees, hedgerows and grass are the scenery layer's and the grass
 * pass's, which hold their own buffers.
 */
export function mapAppearances(
  appearances: InstalledAppearances,
  props: readonly MapProp[],
  fit: PropAppearances,
  buildings: BuildingIndex,
  units: boolean,
): InstalledAppearances {
  const rowDrawn = buildings.partBuilding;
  const drawn = fit.drawnFor(props.filter((p) => !fit.drawsTree(p.kind) && !rowDrawn.has(p.id)));
  const kits = buildings.placed.template.length > 0;
  return {
    ...appearances,
    appearances: new Map(
      [...appearances.appearances].filter(
        ([name, a]) =>
          (units && (a.unit === "soldier" || a.unit === "vehicle")) ||
          (kits && a.unit === "kit") ||
          drawn.has(name),
      ),
    ),
    // The library is installed whole or not at all: its rows name every kit.
    templates: kits ? appearances.templates : undefined,
  };
}
