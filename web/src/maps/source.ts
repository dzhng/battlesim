// Where a map comes from, and the one way a battle gets it
// (`contract::maps::MapSource`): a saved map of the catalogue, read by id
// through an adapter (`browser.ts`, `node.ts`), or a map the simulation's
// generator makes from a request. Either way the answer is a resolved map:
// its physical definition and what it is. The battle never learns which.
//
// Plain TypeScript with no bundler features, so Node tools import it too.
import { MapResolveError, type MapIdentity, type ResolvedMap } from "./resolve.ts";

export type MapType = "open" | "mixed" | "metro";
export type MapSize = "small" | "medium" | "large";
export const MAP_TYPES: readonly MapType[] = ["open", "mixed", "metro"];
export const MAP_SIZES: readonly MapSize[] = ["small", "medium", "large"];

/** What a player chooses: the two composition controls and the seed. */
export interface MapChoice {
  type: MapType;
  size: MapSize;
  /** Canonical u64 decimal text: a JS number cannot hold every seed. */
  seed: string;
}

/** Mirrors `contract::generation::CompileLimits`: the compiler's admission. */
export interface CompileLimits {
  max_authored_parts: number;
  max_bay_positions: number;
  max_ground_points: number;
}

/** Mirrors `contract::generation::GenerationRequest`: a choice pinned to the
 *  generator, presets and physical catalogue that make it one map. The
 *  generator refuses a request pinned to others. */
export interface GenerationRequest extends MapChoice {
  generator_version: string;
  preset_revision: string;
  template_catalog_hash: string;
  limits: CompileLimits;
}

/** Mirrors `contract::maps::MapSource`. */
export type MapSource =
  | { kind: "catalogue"; id: string }
  | { kind: "generated"; request: GenerationRequest };

/** A refusal's row: the generator's `mapgen::Diagnostic`, or a catalogue
 *  refusal in the same shape. */
export interface MapDiagnostic {
  code: string;
  feature: string | null;
  location: string;
  message: string;
}

/** A source that yielded no map, with every diagnostic. Nothing stands in
 *  for it: never another seed, never another map. */
export class MapRefused extends Error {
  constructor(readonly diagnostics: MapDiagnostic[]) {
    super(diagnostics.map((d) => d.message).join("; "));
    this.name = "MapRefused";
  }
}

/** A resolved map as preparation hands it on. */
export interface SourcedMap extends ResolvedMap {
  /** `contract::encounter::EncounterSites` as the generator wrote them, for
   *  the encounter planner; a saved map has none. */
  sites: string | null;
}

/** The generator's documents: `fixtures/map-presets.json` and the physical
 *  template descriptors a request pins, as text. */
export interface GeneratorDocuments {
  presets: string;
  templates: string;
}

/** The simulation's generator, as the Wasm module exports it. */
export interface MapGenerator {
  generate_map(request: string, presets: string, templates: string): string;
  map_generator_version(): string;
  template_catalogue_json(templates: string): string;
}

/** What `resolveMap` reaches a map through. */
export interface MapAccess {
  /** The catalogue's adapter (`loadMap` of `browser.ts` or `node.ts`). */
  loadMap(id: string): ResolvedMap | Promise<ResolvedMap>;
  generator: MapGenerator;
  documents: GeneratorDocuments;
}

const U64_MAX = 2n ** 64n - 1n;

/** `text` as a canonical u64 decimal seed, or null when it is not one. A
 *  seed never passes through a JS number, which rounds above 2^53. */
export function canonicalSeed(text: string): string | null {
  const digits = text.trim();
  if (!/^\d{1,20}$/.test(digits)) return null;
  const value = BigInt(digits);
  return value <= U64_MAX ? value.toString() : null;
}

/** A fresh seed, drawn over the whole u64 range. */
export function newSeed(): string {
  const [high, low] = crypto.getRandomValues(new Uint32Array(2));
  return ((BigInt(high) << 32n) | BigInt(low)).toString();
}

/** `choice` pinned to this build's generator, the presets' revision and the
 *  catalogue's hash, under `limits`. */
export function generationRequest(
  generator: MapGenerator,
  choice: MapChoice,
  documents: GeneratorDocuments,
  limits: CompileLimits,
): GenerationRequest {
  const { hash } = JSON.parse(generator.template_catalogue_json(documents.templates)) as {
    hash: string;
  };
  const { revision } = JSON.parse(documents.presets) as { revision: string };
  return {
    generator_version: generator.map_generator_version(),
    preset_revision: revision,
    seed: choice.seed,
    template_catalog_hash: hash,
    type: choice.type,
    size: choice.size,
    limits,
  };
}

/** The text of one field of a Rust outcome record, between the record's
 *  opening (or the field before it) and the field after it (or the record's
 *  close). */
export function between(outcome: string, opens: string, closes: string, what: string): string {
  const start = outcome.lastIndexOf(opens);
  const end = outcome.lastIndexOf(closes);
  if (start < 0 || end < start + opens.length)
    throw new Error(`${what} is not laid out as ${opens}…${closes}`);
  return outcome.slice(start + opens.length, end);
}

/** The map and its sites as `generate_map`'s accepted outcome wrote them:
 *  `GeneratedMap`'s fields in order are map, identity, report, sites. */
export function mapAndSites(generated: string): { map: string; sites: string } {
  return {
    map: between(generated, '{"status":"ok","result":{"map":', ',"identity":{', "the map"),
    sites: between(generated, ',"sites":', "}}", "the map's sites"),
  };
}

type GenerateOutcome =
  | {
      status: "ok";
      result: {
        map: ResolvedMap["definition"];
        identity: Extract<MapIdentity, { kind: "generated" }>["generation"];
      };
    }
  | { status: "error"; diagnostics: MapDiagnostic[] };

/** The map `source` names, resolved, or `MapRefused` with the catalogue's or
 *  the generator's diagnostics. */
export async function resolveMap(source: MapSource, access: MapAccess): Promise<SourcedMap> {
  if (source.kind === "catalogue") {
    try {
      return { ...(await access.loadMap(source.id)), sites: null };
    } catch (error) {
      if (!(error instanceof MapResolveError)) throw error;
      const { code, location, message } = error;
      throw new MapRefused([{ code, feature: null, location, message }]);
    }
  }
  const generated = access.generator.generate_map(
    JSON.stringify(source.request),
    access.documents.presets,
    access.documents.templates,
  );
  const outcome = JSON.parse(generated) as GenerateOutcome;
  if (outcome.status !== "ok") throw new MapRefused(outcome.diagnostics);
  const { map, sites } = mapAndSites(generated);
  return {
    definition: outcome.result.map,
    identity: { kind: "generated", generation: outcome.result.identity },
    json: map,
    sites,
  };
}
