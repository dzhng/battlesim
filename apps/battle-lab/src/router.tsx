import { lazy, Suspense, type ComponentType, type LazyExoticComponent } from "react";
import fixtures from "./fixtures.json";

// The one lab/scene registry: fixtures.json names every fixture id, route and
// purpose; scene.mjs verifies each id against a harness scene of the same name.
export interface LabFixture {
  id: string;
  route: string;
  describe: string;
  /** "production": a timing verdict, run against a production build. */
  build?: "production";
}

export const LAB_FIXTURES = fixtures as readonly LabFixture[];

/** The page for each fixture id. */
export const ROUTES: Record<string, LazyExoticComponent<ComponentType>> = {
  foundation: lazy(() => import("./routes/foundation")),
  geometry: lazy(() => import("./routes/geometry")),
  authority: lazy(() => import("./routes/authority")),
  movement: lazy(() => import("./routes/movement")),
  sensors: lazy(() => import("./routes/sensors")),
  ballistics: lazy(() => import("./routes/ballistics")),
  contacts: lazy(() => import("./routes/contacts")),
  weapons: lazy(() => import("./routes/weapons")),
  consequences: lazy(() => import("./routes/consequences")),
  deployment: lazy(() => import("./routes/deployment")),
  ambush: lazy(() => import("./routes/ambush")),
  garrison: lazy(() => import("./routes/garrison")),
  supply: lazy(() => import("./routes/supply")),
  readouts: lazy(() => import("./routes/readouts")),
  village: lazy(() => import("./routes/village")),
  "village-perf": lazy(() => import("./routes/village")),
  endurance: lazy(() => import("./routes/endurance")),
  "village-replay": lazy(() => import("./routes/villageReplay")),
};

export function LabRouter({ path }: { path: string }) {
  const fixture = LAB_FIXTURES.find((f) => f.route === path);
  const Route = fixture && ROUTES[fixture.id];
  if (!Route) {
    return (
      <main style={{ padding: 24 }}>
        <h1>Battle lab</h1>
        <ul>
          {LAB_FIXTURES.map((f) => (
            <li key={f.id}>
              <a href={f.route}>{f.id}</a> — {f.describe}
            </li>
          ))}
        </ul>
      </main>
    );
  }
  return (
    <Suspense fallback={null}>
      <Route />
    </Suspense>
  );
}
