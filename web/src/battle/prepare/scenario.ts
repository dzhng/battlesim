/** Preparing an authored scenario: a fixed battle on a `scenario` saved map,
 *  resolved by the simulation under the build's own rules
 *  (`ScenarioDocument::on` through `authored_scenario`). The page names the
 *  map and the scenario; nothing else, since the document fixes the players,
 *  the seed and the forces. */
import { loadMap, loadScenarioDocument, loadScenarioSolution } from "../../maps/browser";
import type { Resume } from "../sim/protocol";
import { MapResolveError } from "../../maps/resolve";
import { PreparationRefused, type PreparationModule, type PreparationResult } from "./prepare";
import type { PrepareDocuments, PrepareStage, ScenarioRequest } from "./protocol";

/** The simulation's scenario resolver, beside what preparation calls. */
export interface ScenarioPreparationModule extends PreparationModule {
  authored_scenario(map: string, document: string, rules: string): string;
  scenario_chapter(
    map: string,
    document: string,
    solution: string,
    rules: string,
    chapter: string,
    beat?: string,
  ): string;
}

/** The sites a resolved scenario carries: where blue enters, for the camera,
 *  and its objectives, the first of which a view of the battle looks at. */
interface ScenarioSites {
  skirmish: {
    sites: {
      entries: { side: string; center: [number, number]; yaw: number }[];
      objectives: { center: [number, number] }[];
    };
  };
}

/** Throws `PreparationRefused` when the map or the scenario is refused. */
export async function prepareScenario(
  wasm: ScenarioPreparationModule,
  memory: WebAssembly.Memory,
  request: ScenarioRequest,
  documents: PrepareDocuments,
  onStage: (stage: PrepareStage) => void = () => {},
  now: () => number = () => performance.now(),
): Promise<PreparationResult> {
  const started = now();
  onStage("map");
  const refused = (stage: "map" | "encounter", location: string, error: unknown) =>
    new PreparationRefused(stage, [
      {
        code: error instanceof MapResolveError ? error.code : "invalid_scenario",
        feature: null,
        location: error instanceof MapResolveError ? error.location : location,
        message: error instanceof Error ? error.message : String(error),
      },
    ]);
  const { chapter } = request;
  const [map, document, solution] = await Promise.all([
    loadMap(request.map),
    loadScenarioDocument(request.map, request.name),
    chapter === undefined ? null : loadScenarioSolution(request.map, request.name),
  ]).catch((error: unknown) => {
    throw refused("map", request.map, error);
  });
  const resolvedAt = now();
  onStage("encounter");
  let resolved: string;
  try {
    const definition = JSON.stringify(map.definition);
    resolved =
      solution === null || chapter === undefined
        ? wasm.authored_scenario(definition, document, documents.rules)
        : wasm.scenario_chapter(
            definition,
            document,
            solution,
            documents.rules,
            chapter,
            request.beat,
          );
  } catch (error) {
    throw refused("encounter", `${request.map}/scenarios/${request.name}.json`, error);
  }
  // `{"battle_seed":<n>,…,"scenario":<text>}`: the scenario stays the exact
  // text a battle takes; a chapter's start also names where it resumes.
  const at = resolved.indexOf(',"scenario":');
  const head = JSON.parse(`${resolved.slice(0, at)}}`) as { battle_seed: number; resume?: Resume };
  const battleSeed = Number(head.battle_seed);
  const scenario = resolved.slice(at + ',"scenario":'.length, -1);
  const world = wasm.PreparedWorld.from_scenario(scenario);
  const worldBuiltAt = now();
  const { sites } = (JSON.parse(scenario) as ScenarioSites).skirmish;
  const blue = sites.entries.find((entry) => entry.side === "blue")!;
  const buildings = map.definition.buildings ?? [];
  return {
    world,
    scenario,
    picture: null,
    report: {
      scenario: { ...request, battleSeed },
      ...(head.resume && { resume: head.resume }),
      identity: map.identity,
      size: map.definition.size,
      extents: JSON.parse(world.extents()),
      counts: {
        buildings: buildings.length,
        parts: buildings.reduce((n, b) => n + b.parts.length, 0),
        props: map.definition.props.length,
        surfaces: map.definition.surfaces.length,
        forests: map.definition.forests.length,
      },
      start: { at: blue.center, yaw: blue.yaw },
      town: sites.objectives[0]?.center ?? null,
      timings: { map: resolvedAt - started, encounter: now() - resolvedAt },
      wasmBytes: memory.buffer.byteLength,
      worldBuildMs: worldBuiltAt - resolvedAt,
    },
  };
}
