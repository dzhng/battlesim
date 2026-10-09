import type { MapChoice, MapDiagnostic } from "../../../web/src/maps/source";
import type { MapIdentity } from "../../../web/src/maps/resolve";
import type { Replacement } from "../../fixture-publication/publication";

export type Json = null | boolean | number | string | Json[] | JsonObject;
export interface JsonObject {
  [key: string]: Json;
}
export interface Documents {
  presets: JsonObject;
  defaults: JsonObject;
}
export interface Draft {
  revision: string;
  documents: Documents;
}
export interface Field {
  id: string;
  document: "presets" | "defaults";
  path: string[];
  group: string;
  label: string;
  description: string;
  unit: string;
  role: "construction" | "validation" | "work" | "analysis" | "identity" | "physical";
  editable: boolean;
}
export interface Snapshot extends Draft {
  fields: Field[];
  receipts: { [owner: string]: string };
}
export interface Feature {
  id: string;
  group: string;
  label: string;
  crop?: string;
}
export interface RunRequest {
  purpose: "preview" | "sample";
  retainedArtifactIds: string[];
  draft: Draft;
  choice: MapChoice;
}
export interface Report extends Partial<Inspection> {
  fingerprint: string;
  choice: MapChoice;
  status: "ok" | "refused";
  diagnostics: MapDiagnostic[];
  stage?: string;
  artifactId?: string;
  identity?: Extract<MapIdentity, { kind: "generated" }>["generation"];
  metrics?: JsonObject;
  counts?: JsonObject;
  encounter?: JsonObject;
  timings?: JsonObject;
}
export interface SightReport {
  status: "measured" | "unavailable";
  samples: number;
  median?: number;
  belowTarget?: number;
  target: number;
  step_m: number;
  bearings: number;
  elapsed_ms: number;
  reason?: string;
}
export interface SaveReview {
  candidateId: string;
  revision: string;
  files: Replacement[];
  diagnostics: MapDiagnostic[];
}

/** Native requests carry captured documents; only the local server supplies artifact paths. */
export interface NativeInputs {
  presets: string;
  defaults: string;
  templates: string;
  rules: string;
  catalog: string;
  recipes: string;
}
export type NativeRequest =
  | { operation: "validate"; inputs: NativeInputs }
  | { operation: "generate"; inputs: NativeInputs; choice: MapChoice; artifactDir: string }
  | { operation: "inspect"; artifactDir: string; crop?: string }
  | { operation: "sight"; artifactDir: string };
export interface NativeValidation {
  status: "valid" | "invalid";
  diagnostics: MapDiagnostic[];
  fields: Field[];
}
export type NativeReport = Omit<Report, "fingerprint" | "artifactId">;
export interface Inspection {
  svg: string;
  features: Feature[];
  summary?: string[];
  legend?: { label: string; color: string }[];
}
export interface WorkbenchAPI {
  snapshot(signal?: AbortSignal): Promise<Snapshot>;
  generate(request: RunRequest, signal?: AbortSignal): Promise<Report>;
  inspect(artifactId: string, crop?: string, signal?: AbortSignal): Promise<Inspection>;
  sight(artifactId: string, signal?: AbortSignal): Promise<SightReport>;
  preview(draft: Draft, signal?: AbortSignal): Promise<SaveReview>;
  save(review: SaveReview, signal?: AbortSignal): Promise<Snapshot>;
  export(artifactId: string, signal?: AbortSignal): Promise<JsonObject>;
}
