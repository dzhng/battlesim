export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export type JsonObject = { [key: string]: Json };
export type Section = 'units' | 'weapons' | 'soldiers';

export interface SourceDocument {
  path: string;
  value: JsonObject;
}

export interface MechanicsSnapshot {
  revision: string;
  documents: SourceDocument[];
  catalog: JsonObject;
}

export interface MechanicsChange {
  section: Section;
  id: string;
  path: string[];
  value?: Json;
  restore?: boolean;
  /** Soldier edits are rebound only into this concrete unit's slots. */
  unit?: string;
}

export interface MechanicsDraft {
  revision: string;
  changes: MechanicsChange[];
}

export interface MechanicsPreview {
  revision: string;
  catalog: JsonObject;
  files: { path: string; before: string; after: string }[];
  affectedUnits: string[];
  warnings: string[];
}

export const MECHANICS_API = '/__mechanics';
