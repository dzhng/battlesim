// Routes and scene identities belong to the fixture registry; their saved
// map references name catalogue folders, never another map definition.
import fixtures from "./fixtures.json";
import { categoryMap, listMaps } from "@web/maps/catalogue";

/** Registered scenes form the browser test harness and closeout gate. Tools are lab pages for humans and runnable measurements, not tests; verify skips them and explicit ids run them. */
export interface LabFixture {
  id: string;
  route: string;
  describe: string;
  /** The fixed saved map, or null for tools and generated/synthetic worlds. */
  map: string | null;
  /** A lab page for humans and a runnable measurement, not a test; verify skips it and an explicit id runs it. */
  tool?: boolean;
  /** A timing verdict runs against a production build. */
  build?: "production";
  /** Runs alone: timing and parity scenes GPU contention would invalidate. */
  serial?: boolean;
  /** Catalog scope used by the router; omitted fixtures use the test catalog. */
  catalog?: "game" | "capture" | "test";
  /** Show the lab loading cover while this fixture prepares (default true). */
  loading?: boolean;
  /** Seconds its scene may run, other than the runner's default. */
  timeout_s?: number;
}

export const LAB_FIXTURES: readonly LabFixture[] = fixtures as readonly LabFixture[];

/** The fixed saved map declared by a fixture: always a `test` map, since a
 *  lab stands on no capture map's battlefield. */
export function fixtureMap(id: string): string {
  const fixture = LAB_FIXTURES.find((fixture) => fixture.id === id);
  if (!fixture?.map) throw new Error(`fixture ${id} has no fixed saved map`);
  return categoryMap(listMaps(), fixture.map, "test").id;
}
