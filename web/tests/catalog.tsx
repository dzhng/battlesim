// The catalog tests run on: the test set (the game's units and the test
// units), resolved by the session factory as a lab's is, and how a test
// builds a lab scenario or renders a view under it.
import type { ReactNode } from "react";
import { catalogSet } from "@web/battle/catalog/sets";
import { SessionCatalogProvider } from "@web/battle/catalog/context";
import type { GameRules } from "@web/battle/catalog/compose";
import type { LabEvent, LabScript, LabUnit } from "@apps/battle-lab/src/scenarios";

export const TEST_CATALOG = await catalogSet("test");
export const UNITS = TEST_CATALOG.units;
export const WEAPONS = TEST_CATALOG.weapons;
/** `game.json` with the test set's documents as its catalog. */
export const TEST_RULES = TEST_CATALOG.rules;

/** A lab scenario: `map` with `units`, `events` and scripted orders under
 *  `rules` (the test set's unless a test pins an experiment control). */
export function labScenario(
  map: unknown,
  units: LabUnit[],
  events: LabEvent[] = [],
  scripts: LabScript[] = [],
  rules: GameRules = TEST_RULES,
): string {
  // The rules read the sections they own from the one fixture and ignore the rest.
  return JSON.stringify({ map, rules, units, events, scripts });
}

/** `children` under the test set, as a lab page provides it. */
export function WithTestCatalog({ children }: { children: ReactNode }) {
  return <SessionCatalogProvider catalog={TEST_CATALOG}>{children}</SessionCatalogProvider>;
}
