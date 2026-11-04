import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { StaticBundle } from "@packages/scene-assets/src/schema";

/** The installed scenery appearances `names` needs, by name; throws naming
 *  any that is missing or is not of one of the scenery `kinds`. */
export function sceneryAppearances(
  installed: InstalledAppearances,
  names: readonly string[],
  kinds: readonly string[],
): Map<string, StaticBundle> {
  const out = new Map<string, StaticBundle>();
  for (const name of names) {
    const entry = installed.appearances.get(name);
    if (!entry) throw new Error(`scenery: appearance "${name}" is not in the catalog (asset bake)`);
    if (
      entry.bundle.kind !== "static" ||
      entry.unit !== "scenery" ||
      !kinds.includes(entry.scenery ?? "")
    )
      throw new Error(`scenery: appearance "${name}" is not a ${kinds.join(" or ")}`);
    out.set(name, entry.bundle);
  }
  return out;
}
