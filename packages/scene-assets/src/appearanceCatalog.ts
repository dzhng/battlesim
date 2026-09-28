// Which appearance the battle draws for a unit on a side. Blue and red share
// the meshes and differ only by the side's tint, applied to the surfaces its
// materials mark (`Material.tint`).
//
// The unit catalog names them: a hull type draws its one `appearance`; a
// squad's soldier draws from his slot's soldier kind's appearance set
// (head, kit, colours), the member his id picks, the same one alive and
// fallen, so no squad reads as copies of one man. A squad's appearances are
// all skinned to one skeleton, so the squad shares one clip set.

import type { Vec3 } from "math";
import type { InstalledAppearances } from "./loader.ts";
import type { Side } from "./schema.ts";
import { mountRoles, type MountRole, type UnitCatalog } from "./units.ts";

export interface ResolvedAppearance {
  /** The installed appearance's name. */
  appearance: string;
  /** Linear RGB multiplier on the appearance's tint-masked surfaces. */
  tint: Vec3;
}

export class AppearanceCatalog {
  private readonly sides: InstalledAppearances["sides"];
  private readonly installed: InstalledAppearances;
  private readonly units: UnitCatalog;
  private readonly roles = new Map<string, readonly MountRole[]>();

  constructor(installed: InstalledAppearances, units: UnitCatalog) {
    this.installed = installed;
    this.units = units;
    this.sides = installed.sides;
    const skeletonOf = (names: readonly string[]) =>
      new Set(
        names.flatMap((n) => {
          const bundle = installed.appearances.get(n)?.bundle;
          return bundle?.kind === "skinned" ? [bundle.skeleton] : [];
        }),
      );
    const refuse = (who: string, skeletons: Set<string>) => {
      if (skeletons.size > 1)
        throw new Error(
          `${who} use skeletons ${[...skeletons].sort().join(" and ")}; a squad's soldiers share one skeleton and clip set`,
        );
    };
    for (const [kind, soldier] of Object.entries(units.view.soldiers))
      refuse(`soldier kind ${kind}'s appearances`, skeletonOf(soldier.appearance));
    for (const id of units.ids) {
      const slots = units.slots(id);
      if (slots.length)
        refuse(
          `unit type ${id}'s soldiers`,
          skeletonOf(slots.flatMap((k) => units.soldier(k).appearance)),
        );
    }
  }

  /** How type `kind`'s model draws each of its mounts, in mount order: the
   *  rig its installed appearance declares (`mountRoles`), by hand for a
   *  squad, and by hand for every mount when nothing is installed. */
  mountRoles(kind: string): readonly MountRole[] {
    let roles = this.roles.get(kind);
    if (!roles) {
      const type = this.units.type(kind);
      roles = mountRoles(type, this.installed.appearances.get(type.appearance ?? "")?.mounts);
      this.roles.set(kind, roles);
    }
    return roles;
  }

  /** The appearance a unit of type `kind` draws on `side`: a hull's model,
   *  or for soldier `id` in slot `slot` the member of his soldier kind's set
   *  his id picks (`id` modulo the installed members: consecutive soldiers
   *  never share one). Null when nothing is installed for it: nothing is
   *  drawn. */
  resolve(kind: string, side: Side, id = 0, slot = 0): ResolvedAppearance | null {
    if (!this.units.has(kind)) return null;
    const soldier = this.units.slots(kind)[slot];
    const set = this.units.hull(kind)
      ? [this.units.type(kind).appearance ?? ""]
      : soldier
        ? this.units.soldier(soldier).appearance
        : [];
    const names = set.filter((n) => this.installed.appearances.has(n));
    if (!names.length) return null;
    const pick = ((id % names.length) + names.length) % names.length;
    return { appearance: names[pick], tint: this.sides[side] };
  }
}
