// Battles on saved maps. Every route gets its map from the catalogue
// (`fixtures/maps/<id>/`) through the one resolver (`@web/maps/browser`),
// and lays an encounter on it under the shared rules: a saved encounter
// (`encounters/<name>.json`) for a lab, the simulation's own factory for the
// village and the endurance battle.
import type { ReactNode } from "react";
import { loadEncounter, loadMap } from "@web/maps/browser";
import type { MapDefinition } from "@web/maps/resolve";
import type { Wasm } from "@web/battle/sim/module";
import {
  labScenario,
  VILLAGE_RULES,
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

/** The village scenario for `variant`, built by the simulation from the
 *  rules and the village's resolved map. */
export async function villageScenario(
  wasm: Wasm,
  variant: string,
  rules = VILLAGE_RULES,
): Promise<string> {
  const { definition } = await loadMap("village");
  return wasm.village_scenario(JSON.stringify({ ...rules, map: definition }), variant);
}

/** The endurance battle for `seed` on its saved field, with the late state's
 *  remains when `late`. */
export async function enduranceScenario(wasm: Wasm, seed: number, late: boolean): Promise<string> {
  const { definition } = await loadMap("endurance");
  return wasm.endurance_scenario(
    JSON.stringify(definition),
    JSON.stringify(VILLAGE_RULES),
    BigInt(seed),
    late,
  );
}

/** The saved encounter `name` of the saved map `id`, as a battle under
 *  `rules`. */
export async function savedBattle(
  id: string,
  name: string,
  rules = VILLAGE_RULES,
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
    scenario: labScenario(definition, encounter.units, encounter.events, encounter.scripts, rules),
  };
}

/** What a route shows while its saved documents load (nothing) or when one is
 *  refused (the resolver's diagnostic); `children` once they have loaded. */
function Loaded<T>({
  value,
  what,
  children,
}: {
  value: T | { error: string } | null;
  what: string;
  children: (value: T) => ReactNode;
}) {
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
  return (
    <Loaded value={map} what={`the map "${id}"`}>
      {children}
    </Loaded>
  );
}

/** The saved map `map` with its saved encounters `encounters`, each as a
 *  battle. `rules` are the shared ones unless a lab pins an experiment
 *  control; they are fixed for the route. */
export function SavedEncounters<Name extends string>({
  map,
  encounters,
  rules = VILLAGE_RULES,
  children,
}: {
  map: string;
  encounters: readonly Name[];
  rules?: typeof VILLAGE_RULES;
  children: (battles: Record<Name, SavedBattle>) => ReactNode;
}) {
  const battles = useBuiltScenario({ map, encounters }, async (_, o) => {
    const loaded = await Promise.all(o.encounters.map((name) => savedBattle(o.map, name, rules)));
    return Object.fromEntries(o.encounters.map((name, k) => [name, loaded[k]])) as Record<
      Name,
      SavedBattle
    >;
  });
  return (
    <Loaded value={battles} what={`the map "${map}" and its encounters ${encounters.join(", ")}`}>
      {children}
    </Loaded>
  );
}

/** The saved map `map` with its one saved encounter `encounter`, as a battle. */
export function SavedEncounter({
  map,
  encounter,
  rules,
  children,
}: {
  map: string;
  encounter: string;
  rules?: typeof VILLAGE_RULES;
  children: (battle: SavedBattle) => ReactNode;
}) {
  return (
    <SavedEncounters map={map} encounters={[encounter]} rules={rules}>
      {(battles) => children(battles[encounter])}
    </SavedEncounters>
  );
}
