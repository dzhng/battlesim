// The shipped unit catalog, resolved by the simulation
// (`fixtures/catalog.json`; a Rust test keeps it current).
import view from "@fixtures/catalog.json";
import { UnitCatalog, type CatalogView, type WeaponRow } from "./units.ts";

export const UNITS = new UnitCatalog(view as unknown as CatalogView);

/** `village.json`'s weapon rows as the simulation resolved them (`extends`
 *  applied), by the names mounts give them. Read these, never the raw rows. */
export const WEAPONS = (view as unknown as { weapons: Record<string, WeaponRow> }).weapons;
