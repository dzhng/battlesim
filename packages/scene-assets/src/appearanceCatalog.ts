// Which appearance the battle draws for a unit on a side. Blue and red share
// the meshes and differ only by the side's tint, applied to the surfaces its
// materials mark (`Material.tint`).
//
// The unit catalog names them: a hull type draws its one `appearance`; a
// squad's soldier draws from his slot's soldier kind's appearance set
// (head, kit, colours), the member his id picks, the same one alive and
// fallen, so no squad reads as copies of one man. A published weapon operator
// may wear its mount's equipment instead. Each appearance supplies its own clips.
// A soldier appearance may have a look per faction (catalog `factions`): a
// squad every faction fields wears each army's uniform, by the faction the
// side fights for, not by the side.

import type { Vec3 } from "math";
import type { InstalledAppearances } from "./loader.ts";
import type { Side } from "./schema.ts";
import { mountRoles, type Faction, type MountRole, type UnitCatalog } from "./units.ts";

/** The faction each side fights for; absent for a battle of no factions (a lab). */
export type SideFactions = Record<Side, Faction>;

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
  private readonly factions: SideFactions | undefined;

  constructor(installed: InstalledAppearances, units: UnitCatalog, factions?: SideFactions) {
    this.installed = installed;
    this.units = units;
    this.factions = factions;
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
          `${who} use skeletons ${[...skeletons].sort().join(" and ")}; variants in one appearance set share one skeleton and clip set`,
        );
    };
    for (const [kind, soldier] of Object.entries(units.view.soldiers))
      refuse(`soldier kind ${kind}'s appearances`, skeletonOf(soldier.appearance));
    for (const id of units.ids)
      for (const mount of units.type(id).mounts) {
        refuse(
          `unit type ${id}'s ${mount.name} operator appearances`,
          skeletonOf(mount.operator_appearance?.active ?? []),
        );
        refuse(
          `unit type ${id}'s ${mount.name} carried appearances`,
          skeletonOf(mount.operator_appearance?.carried ?? []),
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
   *  never share one), in his side's faction's look of it where installed.
   *  Null when nothing is installed for it: nothing is drawn. */
  resolve(
    kind: string,
    side: Side,
    id = 0,
    slot = 0,
    operatorMount: number | null = null,
    activeMount: number | null = null,
  ): ResolvedAppearance | null {
    if (!this.units.has(kind)) return null;
    const soldier = this.units.slots(kind)[slot];
    const equipment =
      operatorMount === null
        ? undefined
        : this.units.type(kind).mounts[operatorMount]?.operator_appearance;
    const operated = equipment
      ? activeMount === operatorMount
        ? equipment.active
        : equipment.carried
      : [];
    const set = this.units.hull(kind)
      ? [this.units.type(kind).appearance ?? ""]
      : operated.length
        ? operated
        : soldier
          ? this.units.soldier(soldier).appearance
          : [];
    const names = set.filter((n) => this.installed.appearances.has(n));
    if (!names.length) return null;
    const picked = names[((id % names.length) + names.length) % names.length];
    const look =
      this.factions && this.installed.appearances.get(picked)?.factions?.[this.factions[side]];
    const appearance = look && this.installed.appearances.has(look) ? look : picked;
    return { appearance, tint: this.sides[side] };
  }
}
