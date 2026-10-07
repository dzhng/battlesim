import type { Replacement } from "../../fixture-publication/publication";

export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export type JsonObject = { [key: string]: Json };
export type Section = "units" | "weapons" | "soldiers";

export interface SourceDocument {
  path: string;
  value: JsonObject;
}

export interface MechanicsSnapshot {
  revision: string;
  documents: SourceDocument[];
  /** Every editable unit's resolved view, the test units included. */
  catalog: JsonObject;
  /** The game's resolved view, as `fixtures/catalog.json` holds it: what a
   *  page installs as its game catalog. */
  game: JsonObject;
}

/** What the editor's draft reads: the sources and every editable unit. */
export type EditableSources = Pick<MechanicsSnapshot, "documents" | "catalog">;

export interface MechanicsChange {
  section: Section;
  id: string;
  path: string[];
  value?: Json;
  restore?: boolean;
  /** Soldier edits are rebound only into this concrete unit's slots. */
  unit?: string;
}

export const targetKey = (target: Pick<MechanicsChange, "section" | "id" | "unit">) =>
  JSON.stringify([target.section, target.id, target.unit ?? null]);

export interface MechanicsDraft {
  revision: string;
  changes: MechanicsChange[];
}

export interface MechanicsPreview {
  revision: string;
  /** Scoped soldier target keys map to their accepted identity after rebinding or cleanup. */
  soldierIds: Record<string, string>;
  catalog: JsonObject;
  files: Replacement[];
  affectedUnits: string[];
  warnings: string[];
}

export const MECHANICS_API = "/__mechanics";
