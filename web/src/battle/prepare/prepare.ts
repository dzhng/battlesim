/** Preparing a battle: the request is checked by the simulation, its map is
 *  resolved through the one map owner (a saved map by id, or the generator's
 *  for a request), the encounter is laid on it (the simulation's planner
 *  places a recipe on a generated map; a saved map plays its saved
 *  encounter), and the result is the scenario JSON any battle authority
 *  runs. This module only carries text between those: every generation and
 *  placement rule is the Wasm module's. Pure over the module and the map
 *  adapter it is given, so the worker and a test share it. */
import type { SimBattle } from "../sim/authority";
import type { Encounter, ResolvedMap } from "../../maps/resolve.ts";
import { MapResolveError } from "../../maps/resolve.ts";
import { between, MapRefused, resolveMap, type MapGenerator } from "../../maps/source.ts";
import type {
  EncounterPlacement,
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

/** The simulation's world of one map, built once: the planner places the
 *  encounter on it and the battle then takes it. */
export interface PreparedWorld {
  admit_skirmish(sites: string): string;
  skirmish_fields(sites: string, factions: string): string;
  extents(): string;
  plan_encounter(sites: string, recipe: string, encounterSeed: string): string;
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

/** The saved catalogue, as an adapter reaches it (`browser.ts`, `node.ts`). */
export interface SavedMaps {
  loadMap(id: string): ResolvedMap | Promise<ResolvedMap>;
  loadEncounter(id: string, name: string): Encounter | Promise<Encounter>;
}

/** One unit of a scenario, as far as preparation reads it. */
interface PlacedUnit {
  side: "blue" | "red";
  position: [number, number];
  yaw?: number;
}

/** `contract::encounter::EncounterDefinition`, as far as preparation reads it. */
interface PlannedEncounter {
  recipe_hash: string;
  encounter_seed: string;
  setup: { units: PlacedUnit[]; encounter?: CompletionRule | null };
  placement: EncounterPlacement;
}

/** `contract::scenario::EncounterRules`, as far as preparation reads it. */
interface CompletionRule {
  success_zone_center: [number, number];
  success_zone_radius_m: number;
  hold_s: number;
}

type Outcome<T> = ({ status: "ok" } & T) | { status: "error"; diagnostics: PrepareDiagnostic[] };

/** A request the simulation's check refused (`request`), a map its owner
 *  refused (`map`) or an encounter the planner refused (`encounter`), with
 *  the diagnostics. */
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

/** The encounter laid on the map: its scenario fields as text (everything
 *  after the map and the rules), its units, and the plan if it was planned. */
type LaidEncounter = (
  | {
      scenario: string;
      metadata: {
        start: PreparedBattle["report"]["start"];
        livingUnits: Record<"blue" | "red", number>;
      };
      stress: StressPreparation;
    }
  | { fields: string; units: PlacedUnit[] }
) & {
  rule: CompletionRule | null;
  planned: PreparedBattle["report"]["planned"];
};

/** Throws `PreparationRefused` when the request, the map or the encounter
 *  is refused. */
export async function prepare(
  wasm: PreparationModule,
  memory: WebAssembly.Memory,
  request: PrepareBattleRequest,
  documents: PrepareDocuments,
  saved: SavedMaps,
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
    (!stress ||
      stress.kind !== "city-arena-2" ||
      typeof stress.late !== "boolean" ||
      source.kind !== "generated")
  )
    throw new PreparationRefused("request", [
      {
        code: "invalid_request",
        feature: null,
        location: "$.stress",
        message: "city-arena-2 stress requires a generated map and a boolean late state",
      },
    ]);
  const started = now();

  onStage("map");
  const map = await resolveMap(source, {
    loadMap: saved.loadMap,
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
    let skirmish =
      map.sites === null
        ? null
        : (
            JSON.parse(map.sites) as {
              skirmish?: { entries: { side: string; center: [number, number]; yaw: number }[] };
            }
          ).skirmish;
    if (skirmish) {
      try {
        const admitted = JSON.parse(world.admit_skirmish(map.sites!)) as {
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
    if (stress && stressScenario && metadata) {
      laid = { scenario: stressScenario, metadata, stress, rule: null, planned: null };
    } else if (checked.skirmish) {
      if (!skirmish || map.sites === null) throw new Error("the skirmish has no admitted sites");
      laid = {
        fields: world.skirmish_fields(map.sites, JSON.stringify(checked.skirmish)).slice(1),
        units: [],
        rule: null,
        planned: null,
      };
    } else if (source.kind === "generated") {
      const recipe = (JSON.parse(documents.recipes) as { recipes: Record<string, unknown> })
        .recipes[checked.recipe_id];
      if (recipe === undefined || map.sites === null)
        throw new PreparationRefused("encounter", [
          {
            code: "invalid_request",
            feature: null,
            location: "$.recipe_id",
            message: `there is no encounter recipe "${checked.recipe_id}"`,
          },
        ]);
      const outcome = world.plan_encounter(
        map.sites,
        JSON.stringify(recipe),
        checked.encounter_seed,
      );
      const { encounter } = accepted<{ encounter: PlannedEncounter }>("encounter", outcome);
      laid = {
        // `EncounterDefinition`'s fields in order: …, setup, placement. The
        // setup is the scenario's own fields after its map and rules, kept as
        // the planner's bytes.
        fields: between(outcome, ',"setup":{', ',"placement":{', "the encounter"),
        units: encounter.setup.units,
        rule: encounter.setup.encounter ?? null,
        planned: {
          recipe_hash: encounter.recipe_hash,
          encounter_seed: encounter.encounter_seed,
          placement: encounter.placement,
        },
      };
    } else {
      const encounter = await (async () =>
        saved.loadEncounter(source.id, checked.recipe_id))().catch((error: unknown) => {
        if (!(error instanceof MapResolveError)) throw error;
        const { code, location, message } = error;
        throw new PreparationRefused("encounter", [{ code, feature: null, location, message }]);
      });
      laid = {
        fields: JSON.stringify(encounter).slice(1),
        units: encounter.units as PlacedUnit[],
        rule: (encounter.encounter as CompletionRule | null | undefined) ?? null,
        planned: null,
      };
    }
    let start: PreparedBattle["report"]["start"];
    if ("metadata" in laid) start = laid.metadata.start;
    else {
      const first = laid.units.find((u) => u.side === "blue");
      const base = skirmish?.entries.find((e) => e.side === "blue");
      if (!first && !base) throw new Error("the encounter has no blue unit or admitted base");
      const column = laid.planned?.placement.deployments.find((d) => d.side === "blue");
      start = {
        at: column?.head ?? base?.center ?? first!.position,
        yaw: column?.yaw ?? base?.yaw ?? first?.yaw ?? 0,
      };
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
        planned: laid.planned,
        objective: laid.rule && {
          center: laid.rule.success_zone_center,
          radius_m: laid.rule.success_zone_radius_m,
          hold_s: laid.rule.hold_s,
        },
        start,
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
