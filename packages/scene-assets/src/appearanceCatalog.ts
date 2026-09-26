// Which appearance the battle draws for a unit kind on a side. Each kind has
// exactly one appearance; blue and red share its meshes and differ only by the
// side's tint, applied to the surfaces its materials mark (`Material.tint`).

import type { Vec3 } from "math";
import type { InstalledAppearances } from "./loader.ts";
import type { Side, UnitKind } from "./schema.ts";

/** Unit kinds drawn one appearance per kind; buildings and scenery are per placement. */
const UNIQUE: readonly UnitKind[] = ["rifle", "recon", "at", "tank", "supply"];

export interface ResolvedAppearance {
  /** The installed appearance's name. */
  appearance: string;
  /** Linear RGB multiplier on the appearance's tint-masked surfaces. */
  tint: Vec3;
}

export class AppearanceCatalog {
  private readonly byKind = new Map<UnitKind, string>();
  private readonly sides: InstalledAppearances["sides"];

  constructor(installed: InstalledAppearances) {
    this.sides = installed.sides;
    for (const [name, { unit }] of installed.appearances) {
      if (!UNIQUE.includes(unit)) continue;
      const other = this.byKind.get(unit);
      if (other)
        throw new Error(
          `unit kind ${unit} has two appearances, ${[other, name].sort().join(" and ")}; keep one`,
        );
      this.byKind.set(unit, name);
    }
  }

  resolve(kind: UnitKind, side: Side): ResolvedAppearance | null {
    const appearance = this.byKind.get(kind);
    return appearance ? { appearance, tint: this.sides[side] } : null;
  }
}
