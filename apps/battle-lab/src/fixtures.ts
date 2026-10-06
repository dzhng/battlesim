// Routes and scene identities belong to the fixture registry; their saved
// map references name catalogue folders, never another map definition.
import fixtures from "./fixtures.json";

export interface LabFixture {
  id: string;
  route: string;
  describe: string;
  /** The fixed saved map, or null for tools and generated/synthetic worlds. */
  map: string | null;
  /** A timing verdict runs against a production build. */
  build?: "production";
  /** Seconds its scene may run, past the runner's default: a scene whose
   *  work grows with the art library. */
  timeout_s?: number;
}

export const LAB_FIXTURES: readonly LabFixture[] = fixtures as readonly LabFixture[];

/** The fixed saved-map source declared by a fixture. */
export function fixtureMap(id: string): string {
  const fixture = LAB_FIXTURES.find((fixture) => fixture.id === id);
  if (!fixture?.map) throw new Error(`fixture ${id} has no fixed saved map`);
  return fixture.map;
}
