// Battles on saved maps. Every route gets its map from the catalogue
// (`fixtures/maps/<id>/`) through the one resolver (`@web/maps/browser`),
// and lays an encounter on it under the shared rules: a saved encounter
// (`encounters/<name>.json`) for a lab, the simulation's own factory for the
// village and the endurance battle.
import { fixtureMap } from "./fixtures";
import type { ReactNode } from "react";
import { loadEncounter, loadMap } from "@web/maps/browser";
import type { Encounter, MapDefinition } from "@web/maps/resolve";
import type { Wasm } from "@web/battle/sim/module";
import {
  GAME_RULES,
  type LabEncounter,
  type LabEvent,
  type LabScript,
  type LabUnit,
} from "./scenarios";
import { buildFailed, useBuiltScenario } from "./useBuiltScenario";

/** A battle a route plays: its map (what is drawn and picked), the saved
 *  encounter laid on it, and the scenario JSON the authority runs. */
export interface SavedBattle {
  map: MapDefinition;
  encounter: LabEncounter;
  scenario: string;
}

/** The village encounter factory on the fixture's saved map. */
export async function villageScenario(
  wasm: Pick<Wasm, "village_scenario">,
  fixture: string,
  variant: string,
  rules = GAME_RULES,
): Promise<string> {
  const { definition } = await loadMap(fixtureMap(fixture));
  return wasm.village_scenario(JSON.stringify({ ...rules, map: definition }), variant);
}

/** The endurance factory on the fixture's saved field, with the late state's
 *  remains when `late`. */
export async function enduranceScenario(
  wasm: Pick<Wasm, "endurance_scenario">,
  fixture: string,
  seed: number,
  late: boolean,
): Promise<string> {
  const { definition } = await loadMap(fixtureMap(fixture));
  return wasm.endurance_scenario(
    JSON.stringify(definition),
    JSON.stringify(GAME_RULES),
    BigInt(seed),
    late,
  );
}

/** The scenario a saved encounter makes on `map` under `rules`: the whole
 *  encounter, its opponent and completion rules included, as the native
 *  `EncounterDefinition::on` builds it, so a battle runs the same here. */
export function savedScenario(map: unknown, saved: Encounter, rules: unknown): string {
  return JSON.stringify({
    map,
    rules,
    units: saved.units,
    events: saved.events ?? [],
    scripts: saved.scripts ?? [],
    ...(saved.opponent != null && { opponent: saved.opponent }),
    ...(saved.encounter != null && { encounter: saved.encounter }),
  });
}

/** The saved encounter `name` of the saved map `id`, as a battle under
 *  `rules`. */
export async function savedBattle(
  id: string,
  name: string,
  rules = GAME_RULES,
): Promise<SavedBattle> {
  const [{ definition }, saved] = await Promise.all([loadMap(id), loadEncounter(id, name)]);
  const encounter: LabEncounter = {
    units: saved.units as LabUnit[],
    events: (saved.events ?? []) as LabEvent[],
    scripts: (saved.scripts ?? []) as LabScript[],
  };
  return {
    map: definition,
    encounter,
    scenario: savedScenario(definition, saved, rules),
  };
}

/** Render loading and refusal without passing the large result through
 *  component props: React's dev profiler would copy the map's arrays. */
function loaded<T>(
  value: T | { error: string } | null,
  what: string,
  children: (value: T) => ReactNode,
): ReactNode {
  if (value === null) return null;
  if (buildFailed(value))
    return (
      <main style={{ padding: 24 }} className="lab-rejected" data-testid="error">
        {what} could not be loaded: {value.error}
      </main>
    );
  return children(value);
}

/** The saved map `id`, resolved, for a route that draws a map and plays no
 *  saved encounter on it. */
export function SavedMap({
  id,
  children,
}: {
  id: string;
  children: (map: MapDefinition) => ReactNode;
}) {
  const map = useBuiltScenario(id, async (_, id) => (await loadMap(id)).definition);
  return loaded(map, `the map "${id}"`, children);
}

/** The fixture's saved map with its encounters `encounters`, each as a
 *  battle. `rules` are the shared ones unless a lab pins an experiment
 *  control; they are fixed for the route. */
export function SavedEncounters<Name extends string>({
  fixture,
  encounters,
  rules = GAME_RULES,
  children,
}: {
  fixture: string;
  encounters: readonly Name[];
  rules?: typeof GAME_RULES;
  children: (battles: Record<Name, SavedBattle>) => ReactNode;
}) {
  const map = fixtureMap(fixture);
  const battles = useBuiltScenario({ map, encounters }, async (_, o) => {
    const loaded = await Promise.all(o.encounters.map((name) => savedBattle(o.map, name, rules)));
    return Object.fromEntries(o.encounters.map((name, k) => [name, loaded[k]])) as Record<
      Name,
      SavedBattle
    >;
  });
  return loaded(battles, `the map "${map}" and its encounters ${encounters.join(", ")}`, children);
}

/** The fixture's saved map with its one encounter, as a battle. */
export function SavedEncounter({
  fixture,
  encounter,
  rules,
  children,
}: {
  fixture: string;
  encounter: string;
  rules?: typeof GAME_RULES;
  children: (battle: SavedBattle) => ReactNode;
}) {
  return (
    <SavedEncounters fixture={fixture} encounters={[encounter]} rules={rules}>
      {(battles) => children(battles[encounter])}
    </SavedEncounters>
  );
}
