/** Preparing a battle: the request is checked by the simulation, its map is
 *  generated through the one map owner, both factions are fielded on the map's admitted skirmish
 *  sites (a map without them is refused), and the result is the scenario
 *  JSON any battle authority runs. This module only carries text between those: every generation and
 *  placement rule is the Wasm module's. Pure over the module and the map
 *  adapter it is given, so the worker and a test share it. */
import type { SimBattle } from "../sim/authority";
import { between, MapRefused, resolveMap, type MapGenerator } from "../../maps/source.ts";
import type {
  PrepareBattleRequest,
  PrepareDiagnostic,
  PrepareDocuments,
  PreparedBattle,
  PrepareStage,
  RefusalStage,
  StressPreparation,
} from "./protocol.ts";

/** The simulation's functions preparation calls. */
export interface PreparationModule extends MapGenerator {
  city_stress_preparation(map: string, rules: string, seed: bigint, late: boolean): string;
  check_prepare_request(request: string): string;
  PreparedWorld: {
    new (map: string, rules: string): PreparedWorld;
    from_scenario(scenario: string): PreparedWorld;
  };
}

/** The simulation's world of one map, built once: the skirmish sites are
 *  admitted on it and the battle then takes it. */
export interface PreparedWorld {
  admit_skirmish(sites: string): string;
  skirmish_fields(sites: string, factions: string): string;
  extents(): string;
  into_battle(scenario: string, seed: number): SimBattle;
  into_replay(scenario: string, replay: string): SimBattle;
  free(): void;
}
export interface PreparationResult {
  scenario: string;
  report: PreparedBattle["report"];
  world: PreparedWorld;
}

/** Restore the captured world. The battle's replay constructor still checks
 *  the engine, scenario and rules identities before accepting commands. */
export function prepareReplay(wasm: PreparationModule, battle: PreparedBattle): PreparationResult {
  return { ...battle, world: wasm.PreparedWorld.from_scenario(battle.scenario) };
}

type Outcome<T> = ({ status: "ok" } & T) | { status: "error"; diagnostics: PrepareDiagnostic[] };

/** A request the simulation's check refused (`request`), a map its owner
 *  refused (`map`) or a map that cannot field the factions (`encounter`),
 *  with the diagnostics. */
export class PreparationRefused extends Error {
  constructor(
    readonly stage: RefusalStage,
    readonly diagnostics: PrepareDiagnostic[],
  ) {
    super(diagnostics.map((d) => d.message).join("; "));
  }
}

function accepted<T>(stage: RefusalStage, json: string): { status: "ok" } & T {
  const outcome = JSON.parse(json) as Outcome<T>;
  if (outcome.status !== "ok") throw new PreparationRefused(stage, outcome.diagnostics);
  return outcome;
}

/** The forces laid on the map: a stress scene's whole scenario, or the
 *  skirmish's scenario fields as text (everything after the map and the
 *  rules). */
type LaidEncounter =
  | {
      scenario: string;
      metadata: {
        start: PreparedBattle["report"]["start"];
        livingUnits: Record<"blue" | "red", number>;
      };
      stress: StressPreparation;
    }
  | { fields: string };

/** Throws `PreparationRefused` when the request, the map or the encounter
 *  is refused. */
export async function prepare(
  wasm: PreparationModule,
  memory: WebAssembly.Memory,
  request: PrepareBattleRequest,
  documents: PrepareDocuments,
  onStage: (stage: PrepareStage) => void = () => {},
  now: () => number = () => performance.now(),
  stress?: StressPreparation,
): Promise<PreparationResult> {
  const checked = accepted<{ request: PrepareBattleRequest }>(
    "request",
    wasm.check_prepare_request(JSON.stringify(request)),
  ).request;
  const source = checked.map_source;
  if (
    stress !== undefined &&
    (!stress || stress.kind !== "city-arena-2" || typeof stress.late !== "boolean")
  )
    throw new PreparationRefused("request", [
      {
        code: "invalid_request",
        feature: null,
        location: "$.stress",
        message: "city-arena-2 stress requires a boolean late state",
      },
    ]);
  const started = now();

  onStage("map");
  const map = await resolveMap(source, {
    generator: wasm,
    documents: {
      presets: documents.presets,
      templates: documents.templates,
      // Generation reads the physical fields of this same battle rule record.
      rules: documents.rules,
    },
  }).catch((error: unknown) => {
    throw error instanceof MapRefused ? new PreparationRefused("map", error.diagnostics) : error;
  });
  const resolvedAt = now();

  onStage("encounter");
  // The developer stress recipe owns its synthetic remains and full scenario.
  // Prepare its resulting map, which can differ from the generator's map.
  const stressResult = stress
    ? wasm.city_stress_preparation(
        map.json,
        documents.rules,
        BigInt(checked.battle_seed),
        stress.late,
      )
    : null;
  const stressScenario = stressResult
    ? between(stressResult, '{"scenario":', ',"report":{', "the stress scenario")
    : null;
  const metadata = stressResult
    ? (JSON.parse(between(stressResult, ',"report":', "}", "the stress report")) as {
        start: PreparedBattle["report"]["start"];
        livingUnits: Record<"blue" | "red", number>;
      })
    : null;
  const world = stressScenario
    ? wasm.PreparedWorld.from_scenario(stressScenario)
    : new wasm.PreparedWorld(map.json, documents.rules);
  const worldBuiltAt = now();
  try {
    const sites = JSON.parse(map.sites) as {
      settlements?: { center: [number, number] }[];
      skirmish?: { entries: { side: string; center: [number, number]; yaw: number }[] };
    };
    let skirmish = sites.skirmish;
    if (skirmish) {
      try {
        const admitted = JSON.parse(world.admit_skirmish(map.sites)) as {
          sites: { skirmish: NonNullable<typeof skirmish> };
        };
        map.sites = JSON.stringify(admitted.sites);
        skirmish = admitted.sites.skirmish;
      } catch (error) {
        throw new PreparationRefused("encounter", [
          {
            code: "invalid_skirmish_sites",
            feature: "skirmish_sites",
            location: "$.sites.skirmish",
            message: error instanceof Error ? error.message : String(error),
          },
        ]);
      }
    }
    let laid: LaidEncounter;
    let start: PreparedBattle["report"]["start"];
    if (stress && stressScenario && metadata) {
      laid = { scenario: stressScenario, metadata, stress };
      start = metadata.start;
    } else {
      const base = skirmish?.entries.find((e) => e.side === "blue");
      if (!base)
        throw new PreparationRefused("encounter", [
          {
            code: "invalid_request",
            feature: "skirmish_sites",
            location: "$.map_source",
            message: "the map has no skirmish base to field the factions on",
          },
        ]);
      laid = {
        fields: world.skirmish_fields(map.sites, JSON.stringify(checked.factions)).slice(1),
      };
      start = { at: base.center, yaw: base.yaw };
    }
    const buildings = map.definition.buildings ?? [];
    return {
      world,
      scenario:
        "scenario" in laid
          ? laid.scenario
          : `{"map":${map.json},"rules":${documents.rules},${laid.fields}`,
      report: {
        ...("metadata" in laid && {
          stress: { ...laid.stress, livingUnits: laid.metadata.livingUnits },
        }),
        request: checked,
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
        start,
        town: sites.settlements?.[0]?.center ?? null,
        timings: { map: resolvedAt - started, encounter: now() - resolvedAt },
        wasmBytes: memory.buffer.byteLength,
        worldBuildMs: worldBuiltAt - resolvedAt,
      },
    };
  } catch (error) {
    world.free();
    throw error;
  }
}
