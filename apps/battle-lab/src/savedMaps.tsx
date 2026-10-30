// Battles on saved maps. Every route gets its map from the catalogue
// (`fixtures/maps/<id>/`) through the one resolver (`@web/maps/browser`),
// and lays an encounter on it under the shared rules: a saved encounter
// (`encounters/<name>.json`) for a lab or the menu, the simulation's own
// factory for the endurance battle.
import { fixtureMap } from "./fixtures";
import type { ReactNode } from "react";
import { loadEncounter, loadMap } from "@web/maps/browser";
import { MapResolveError, type Encounter, type MapDefinition } from "@web/maps/resolve";
import { loadWasm, type Wasm } from "@web/battle/sim/module";
import type { ResolvedRules } from "@web/battle/catalog/compose";
import { useSessionCatalog } from "@web/battle/catalog/context";
import type { LabEncounter, LabEvent, LabScript, LabUnit } from "./scenarios";
import { buildFailed, useBuiltScenario } from "./useBuiltScenario";

/** A battle a route plays: its map (what is drawn and picked), the saved
 *  encounter laid on it, and the scenario JSON the authority runs. */
export interface SavedBattle {
  map: MapDefinition;
  encounter: LabEncounter;
  scenario: string;
}

/** The endurance factory on the fixture's saved field, with the late state's
 *  remains when `late`. */
export async function enduranceScenario(
  wasm: Pick<Wasm, "endurance_scenario">,
  fixture: string,
  seed: number,
  late: boolean,
  rules: ResolvedRules,
): Promise<string> {
  const { definition } = await loadMap(fixtureMap(fixture));
  return wasm.endurance_scenario(
    JSON.stringify(definition),
    JSON.stringify(rules),
    BigInt(seed),
    late,
  );
}

/** The scenario a saved encounter makes on its map under `rules`, made by
 *  the simulation's own `EncounterDefinition::on`: the whole encounter, its
 *  opponent, completion rules and `overrides` included, so a battle runs the
 *  same here as natively. `map` is the resolver's text (`ResolvedMap.json`). */
export function savedScenario(
  wasm: Pick<Wasm, "encounter_scenario">,
  map: string,
  saved: Encounter,
  rules: unknown,
): string {
  return wasm.encounter_scenario(map, JSON.stringify(saved), JSON.stringify(rules));
}

/** The saved encounter `name` of the saved map `id`, as a battle under
 *  `rules`, or the refusal naming the encounter. */
export async function savedBattle(
  id: string,
  name: string,
  rules: ResolvedRules,
): Promise<SavedBattle> {
  const [wasm, map, saved] = await Promise.all([loadWasm(), loadMap(id), loadEncounter(id, name)]);
  const encounter: LabEncounter = {
    units: saved.units as LabUnit[],
    events: (saved.events ?? []) as LabEvent[],
    scripts: (saved.scripts ?? []) as LabScript[],
  };
  let scenario: string;
  try {
    scenario = savedScenario(wasm, map.json, saved, rules);
  } catch (e) {
    throw new MapResolveError(
      "invalid_encounter",
      `${id}/encounters/${name}.json`,
      (e as Error).message,
    );
  }
  return { map: map.definition, encounter, scenario };
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
 *  battle. `rules` are the session catalog's unless a lab pins an
 *  experiment control, for every encounter or per encounter; they are fixed
 *  for the route. */
export function SavedEncounters<Name extends string>({
  fixture,
  encounters,
  rules: pinned,
  children,
}: {
  fixture: string;
  encounters: readonly Name[];
  rules?: ResolvedRules | ((encounter: Name) => ResolvedRules);
  children: (battles: Record<Name, SavedBattle>) => ReactNode;
}) {
  const catalog = useSessionCatalog();
  const rules = (name: Name) =>
    typeof pinned === "function" ? pinned(name) : (pinned ?? catalog.rules);
  const map = fixtureMap(fixture);
  const battles = useBuiltScenario({ map, encounters }, async (_, o) => {
    const loaded = await Promise.all(
      o.encounters.map((name) => savedBattle(o.map, name, rules(name))),
    );
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
  rules?: ResolvedRules;
  children: (battle: SavedBattle) => ReactNode;
}) {
  return (
    <SavedEncounters fixture={fixture} encounters={[encounter]} rules={rules}>
      {(battles) => children(battles[encounter])}
    </SavedEncounters>
  );
}
