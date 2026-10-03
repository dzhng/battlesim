import { lazy, Suspense, type ComponentType, type LazyExoticComponent } from "react";
import { LAB_FIXTURES } from "./fixtures";
import { listMaps } from "@web/maps/catalogue";
import { MainMenu } from "./MainMenu";

const MechanicsEditor = lazy(() => import("../../mechanics-editor/src/MechanicsEditor"));

/** The pages: the benchmark page also owns its named workload URLs. */
const ROUTES: Record<string, LazyExoticComponent<ComponentType>> = {
  "cursor-orders": lazy(() => import("./routes/cursorOrders")),
  cursor: lazy(() => import("./routes/cursor")),
  foundation: lazy(() => import("./routes/foundation")),
  geometry: lazy(() => import("./routes/geometry")),
  authority: lazy(() => import("./routes/authority")),
  movement: lazy(() => import("./routes/movement")),
  river: lazy(() => import("./routes/river")),
  sensors: lazy(() => import("./routes/sensors")),
  ballistics: lazy(() => import("./routes/ballistics")),
  contacts: lazy(() => import("./routes/contacts")),
  weapons: lazy(() => import("./routes/weapons")),
  consequences: lazy(() => import("./routes/consequences")),
  deployment: lazy(() => import("./routes/deployment")),
  ambush: lazy(() => import("./routes/ambush")),
  garrison: lazy(() => import("./routes/garrison")),
  supply: lazy(() => import("./routes/supply")),
  ground: lazy(() => import("./routes/ground")),
  panels: lazy(() => import("./routes/panels")),
  readouts: lazy(() => import("./routes/readouts")),
  camera: lazy(() => import("./routes/camera")),
  "city-block": lazy(() => import("./routes/cityBlock")),
  facade: lazy(() => import("./routes/facade")),
  "city-lineup": lazy(() => import("./routes/cityLineup")),
  "city-ruins": lazy(() => import("./routes/cityRuins")),
  village: lazy(() => import("./routes/village")),
  generated: lazy(() => import("./routes/battle")),
  benchmark: lazy(() => import("./routes/benchmark")),
  endurance: lazy(() => import("./routes/endurance")),
  "village-replay": lazy(() => import("./routes/villageReplay")),
  "village-watch": lazy(() => import("./routes/villageWatch")),
  "village-lean": lazy(() => import("./routes/villageLean")),
  fog: lazy(() => import("./routes/fog")),
  projectiles: lazy(() => import("./routes/projectiles")),
  workbench: lazy(() => import("./routes/workbench")),
  "fog-look": lazy(() => import("./routes/fogLook")),
  sound: lazy(() => import("./routes/sound")),
};

/** `/` is the main menu, `/labs` the index of every fixture route. */
export function LabRouter({ path }: { path: string }) {
  if (path === "/mechanics" && import.meta.env.DEV)
    return (
      <Suspense fallback={null}>
        <MechanicsEditor />
      </Suspense>
    );
  const fixture = LAB_FIXTURES.find((f) => f.route === path);
  const Route = fixture && ROUTES[fixture.id];
  if (!Route && path !== "/labs") return <MainMenu />;
  if (!Route) {
    return (
      <main
        className="lab-index"
        style={{ padding: 24, height: "100%", overflow: "auto", boxSizing: "border-box" }}
      >
        <p>
          <a href="/">Main menu</a>
        </p>
        <h1>Battle lab</h1>
        <h2>Saved maps</h2>
        <ul>
          {listMaps()
            .filter((map) => map.status !== "retired")
            .map((map) => (
              <li key={map.id}>
                <a href={`/lab/geometry?map=${map.id}`}>{map.label}</a>
                {map.status === "draft" && " — draft"}
              </li>
            ))}
        </ul>
        <h2>Fixtures and tools</h2>
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
