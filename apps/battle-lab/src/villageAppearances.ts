// The battle's appearances, through the one loader (`AppearanceLibrary`):
// every catalog entry the bake wrote under `assets/runtime/` (Vite's
// publicDir, served at the site root). Loaded once per page; every route
// that draws the village's world waits for it, so its trees are there from
// the first frame.
import { useEffect, useState } from "react";
import { AppearanceLibrary, type InstalledAppearances } from "@packages/scene-assets/src/loader";

let loading: Promise<InstalledAppearances> | null = null;

export function villageAppearances(): Promise<InstalledAppearances> {
  loading ??= new AppearanceLibrary().load("/");
  return loading;
}

/** The installed appearances, or null until they load. A failed load is an
 *  error on the console (scenes fail on it), and the world never draws. */
export function useVillageAppearances(): InstalledAppearances | null {
  const [installed, setInstalled] = useState<InstalledAppearances | null>(null);
  useEffect(() => {
    let live = true;
    villageAppearances().then(
      (next) => live && setInstalled(next),
      (error: unknown) => console.error("Appearances failed to load", error),
    );
    return () => {
      live = false;
    };
  }, []);
  return installed;
}
