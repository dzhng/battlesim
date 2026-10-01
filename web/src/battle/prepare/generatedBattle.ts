/** Preparing a battle on a generated map: the simulation's generator makes
 *  the map and its plan from the request, the developer encounter is laid on
 *  them, and the result is the scenario JSON any battle authority runs. Pure
 *  over the Wasm module it is given, so the worker and a test share it. */
import type { Wasm } from "../sim/module";
import {
  developerEncounter,
  type EncounterRuleView,
  type MapView,
  type PlanView,
} from "./developerEncounter";
import type {
  GenerationIdentity,
  MapDiagnostic,
  PreparedBattle,
  PrepareRequest,
  PrepareStage,
} from "./protocol";

/** The generator's functions preparation calls. */
export type MapGenerator = Pick<
  Wasm,
  "generate_map" | "generate_map_plan" | "map_generator_version" | "template_catalogue_json"
>;

/** The compiled map, as far as preparation reads it. */
interface CompiledMap extends MapView {
  surfaces: unknown[];
  forests: unknown[];
  buildings: (MapView["buildings"][number] & { parts: unknown[] })[];
}

type Outcome<T> = ({ status: "ok" } & T) | { status: "error"; diagnostics: MapDiagnostic[] };

/** A request the generator or compiler refused, with its diagnostics. */
export class MapRefused extends Error {
  constructor(readonly diagnostics: MapDiagnostic[]) {
    super(diagnostics.map((d) => d.message).join("; "));
  }
}

function accepted<T>(json: string): { status: "ok" } & T {
  const outcome = JSON.parse(json) as Outcome<T>;
  if (outcome.status !== "ok") throw new MapRefused(outcome.diagnostics);
  return outcome;
}

const MAP_OPENS = '{"status":"ok","result":{"map":';
const MAP_CLOSES = ',"identity":{';

/** The map's own text inside a `generate_map` outcome (`GeneratedMap`'s
 *  fields in order: map, identity, report). The scenario carries these bytes
 *  rather than a re-serialised parse, which would lose what a JSON number
 *  cannot hold in JavaScript: the sign of a zero. */
function mapText(outcome: string): string {
  const end = outcome.lastIndexOf(MAP_CLOSES);
  if (!outcome.startsWith(MAP_OPENS) || end < 0)
    throw new Error("generate_map's outcome is not laid out as map, identity, report");
  return outcome.slice(MAP_OPENS.length, end);
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

/** Throws `MapRefused` when the generator or compiler refuses the request. */
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
  const outcome = wasm.generate_map(generation, request.presets, request.templates);
  const { result } = accepted<{
    result: { map: CompiledMap; identity: GenerationIdentity };
  }>(outcome);
  const generated = now();

  onStage("placing");
  // The map carries none of the plan's settlement layers, and the encounter
  // stands on them: the same request gives the same plan.
  const { plan } = accepted<{ plan: PlanView }>(
    wasm.generate_map_plan(generation, request.presets, request.templates),
  );
  const rules = JSON.parse(request.rules) as EncounterRuleView;
  const { anchors, ...encounter } = developerEncounter(plan, result.map, request.encounter, rules);
  const rest = JSON.stringify({ ...encounter, events: [], scripts: [] });
  return {
    scenario: `{"map":${mapText(outcome)},"rules":${request.rules},${rest.slice(1)}`,
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
      anchors,
      timings: { generating: generated - started, placing: now() - generated },
      wasmBytes: memory.buffer.byteLength,
    },
  };
}
