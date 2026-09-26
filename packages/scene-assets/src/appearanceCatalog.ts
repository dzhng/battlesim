// Which appearance the battle draws for a unit kind on a side. Blue and red
// share the meshes and differ only by the side's tint, applied to the surfaces
// its materials mark (`Material.tint`).
//
// A vehicle kind has exactly one appearance. An infantry kind may have several
// variants (head, kit, colours), all skinned to one skeleton so they share one
// clip set; each soldier draws the variant his id picks, the same one alive and
// fallen, so no squad reads as copies of one man.

import type { Vec3 } from "math";
import type { InstalledAppearances } from "./loader.ts";
import type { Side, UnitKind } from "./schema.ts";

/** Unit kinds drawn from the catalog by kind; buildings and scenery are per placement. */
const BY_KIND: readonly UnitKind[] = ["rifle", "recon", "at", "tank", "supply"];

export interface ResolvedAppearance {
  /** The installed appearance's name. */
  appearance: string;
  /** Linear RGB multiplier on the appearance's tint-masked surfaces. */
  tint: Vec3;
}

export class AppearanceCatalog {
  /** Each kind's appearances, sorted by name: the variant order. */
  private readonly byKind = new Map<UnitKind, string[]>();
  private readonly sides: InstalledAppearances["sides"];

  constructor(installed: InstalledAppearances) {
    this.sides = installed.sides;
    const skeletonOf = new Map<UnitKind, string>();
    for (const [name, { unit, bundle }] of installed.appearances) {
      if (!BY_KIND.includes(unit)) continue;
      const names = this.byKind.get(unit) ?? [];
      if (names.length && bundle.kind !== "skinned")
        throw new Error(
          `unit kind ${unit} has two appearances, ${[names[0], name].sort().join(" and ")}; a vehicle keeps one`,
        );
      if (bundle.kind === "skinned") {
        const skeleton = skeletonOf.get(unit);
        if (skeleton !== undefined && skeleton !== bundle.skeleton)
          throw new Error(
            `unit kind ${unit}'s variants use skeletons ${skeleton} and ${bundle.skeleton}; variants share one skeleton and clip set`,
          );
        skeletonOf.set(unit, bundle.skeleton);
      }
      names.push(name);
      this.byKind.set(unit, names);
    }
    for (const names of this.byKind.values()) names.sort();
  }

  /** The appearance `kind` draws on `side`; for infantry, the variant soldier
   *  `id` wears (`id` modulo the variant count: consecutive soldiers, a
   *  squad's, never share one). */
  resolve(kind: UnitKind, side: Side, id = 0): ResolvedAppearance | null {
    const names = this.byKind.get(kind);
    if (!names) return null;
    const pick = ((id % names.length) + names.length) % names.length;
    return { appearance: names[pick], tint: this.sides[side] };
  }
}
