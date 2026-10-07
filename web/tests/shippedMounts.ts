// How each shipped type's model draws its mounts, from the authored
// appearance catalog, for tests that pose units without installing bundles.
import catalog from "../../assets/catalog.json";
import type { Catalog } from "@packages/scene-assets/src/schema";
import { UNITS } from "./catalog";
import { mountRoles, type MountRole } from "@packages/scene-assets/src/units";

const appearances = (catalog as unknown as Catalog).appearances;

export const shippedMounts = (kind: string): readonly MountRole[] => {
  const type = UNITS.type(kind);
  return mountRoles(type, appearances[type.appearance ?? ""]?.mounts);
};
