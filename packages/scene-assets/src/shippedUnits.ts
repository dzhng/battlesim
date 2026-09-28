// The shipped unit catalog, resolved by the simulation
// (`fixtures/unit-catalog.json`; a Rust test keeps it current).
import view from "@fixtures/unit-catalog.json";
import { UnitCatalog, type CatalogView } from "./units.ts";

export const UNITS = new UnitCatalog(view as unknown as CatalogView);
