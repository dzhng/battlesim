/** Grass appearance inputs for the shared renderer. */
import type { StaticBundle } from "@packages/scene-assets/src/schema.ts";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader.ts";

/** Grass appearances (scenery kind "grass") by catalog name. */
export type GrassAppearances = ReadonlyMap<string, StaticBundle>;

/** The grass kinds in an installed catalog generation. */
export function grassAppearancesOf(installed: InstalledAppearances | null): GrassAppearances {
  const out = new Map<string, StaticBundle>();
  for (const [name, { scenery, bundle }] of installed?.appearances ?? [])
    if (scenery === "grass" && bundle.kind === "static") out.set(name, bundle);
  return out;
}
