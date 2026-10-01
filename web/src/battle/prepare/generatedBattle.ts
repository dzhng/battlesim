/** Preparing a battle on a generated map: the simulation's generator makes
 *  the map and its sites from the request, the simulation's planner places
 *  the recipe's encounter on them, and the result is the scenario JSON any
 *  battle authority runs. This module only carries text between the two:
 *  every generation and placement rule is the Wasm module's. Pure over the
 *  module it is given, so the worker and a test share it. */
import type { Wasm } from "../sim/module";
import type {
  EncounterPlacement,
  GenerationIdentity,
  PrepareDiagnostic,
  PreparedBattle,
  PrepareRequest,
  PrepareStage,
} from "./protocol";

/** The generator's and the planner's functions preparation calls. */
export type MapGenerator = Pick<
  Wasm,
  "generate_map" | "plan_encounter" | "map_generator_version" | "template_catalogue_json"
>;

/** The compiled map, as far as preparation reads it. */
interface CompiledMap {
  size: [number, number];
  surfaces: unknown[];
  forests: unknown[];
  buildings: { parts: unknown[] }[];
}

/** `contract::encounter::EncounterDefinition`, as far as preparation reads it. */
interface PlannedEncounter {
  recipe_hash: string;
  encounter_seed: string;
  setup: { units: { side: "blue" | "red"; position: [number, number]; yaw: number }[] };
  placement: EncounterPlacement;
}

type Outcome<T> = ({ status: "ok" } & T) | { status: "error"; diagnostics: PrepareDiagnostic[] };

/** A request the generator or compiler refused (`generating`) or an
 *  encounter the planner refused (`placing`), with the diagnostics. */
export class PreparationRefused extends Error {
  constructor(
    readonly stage: PrepareStage,
    readonly diagnostics: PrepareDiagnostic[],
  ) {
    super(diagnostics.map((d) => d.message).join("; "));
  }
}

function accepted<T>(stage: PrepareStage, json: string): { status: "ok" } & T {
  const outcome = JSON.parse(json) as Outcome<T>;
  if (outcome.status !== "ok") throw new PreparationRefused(stage, outcome.diagnostics);
  return outcome;
}

/** The text of one field of a Rust outcome record, between the record's
 *  opening (or the field before it) and the field after it (or the record's
 *  close). The scenario carries these bytes rather than a re-serialised
 *  parse, which would lose what a JSON number cannot hold in JavaScript:
 *  the sign of a zero. */
function between(outcome: string, opens: string, closes: string, what: string): string {
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

/** The generation request for `request`'s map, pinned to this build's
 *  generator, the presets' revision and the catalogue's hash. */
export function generationRequest(wasm: MapGenerator, request: PrepareRequest): string {
  const { hash } = JSON.parse(wasm.template_catalogue_json(request.templates)) as { hash: string };
  const { revision } = JSON.parse(request.presets) as { revision: string };
  return JSON.stringify({
    generator_version: wasm.map_generator_version(),
    preset_revision: revision,
    seed: request.map.seed,
    template_catalog_hash: hash,
    type: request.map.type,
    size: request.map.size,
    limits: request.limits,
  });
}

/** Throws `PreparationRefused` when the generator or compiler refuses the
 *  map, or the planner the encounter. */
export function prepareGeneratedBattle(
  wasm: MapGenerator,
  memory: WebAssembly.Memory,
  request: PrepareRequest,
  onStage: (stage: PrepareStage) => void = () => {},
  now: () => number = () => performance.now(),
): PreparedBattle {
  const started = now();
  onStage("generating");
  const generation = generationRequest(wasm, request);
  const generated = wasm.generate_map(generation, request.presets, request.templates);
  const { result } = accepted<{
    result: { map: CompiledMap; identity: GenerationIdentity };
  }>("generating", generated);
  const { map, sites } = mapAndSites(generated);
  const generatedAt = now();

  onStage("placing");
  const planned = wasm.plan_encounter(
    map,
    sites,
    request.rules,
    request.recipe,
    request.encounterSeed,
  );
  const { encounter } = accepted<{ encounter: PlannedEncounter }>("placing", planned);
  // `EncounterDefinition`'s fields in order: …, setup, placement. The setup
  // is the scenario's own fields after its map and rules.
  const setup = between(planned, ',"setup":{', ',"placement":{', "the encounter");
  const { placement } = encounter;
  const column = placement.deployments.find((d) => d.side === "blue");
  const first = encounter.setup.units.find((u) => u.side === "blue");
  if (!first) throw new Error("the planned encounter has no blue unit");
  return {
    scenario: `{"map":${map},"rules":${request.rules},${setup}`,
    report: {
      map: request.map,
      identity: result.identity,
      size: result.map.size,
      counts: {
        buildings: result.map.buildings.length,
        parts: result.map.buildings.reduce((n, b) => n + b.parts.length, 0),
        surfaces: result.map.surfaces.length,
        forests: result.map.forests.length,
      },
      encounter: { recipe_hash: encounter.recipe_hash, encounter_seed: encounter.encounter_seed },
      placement,
      anchors: {
        blue: column?.head ?? first.position,
        blueYaw: column?.yaw ?? first.yaw,
        town: placement.objective.center,
      },
      timings: { generating: generatedAt - started, placing: now() - generatedAt },
      wasmBytes: memory.buffer.byteLength,
    },
  };
}
