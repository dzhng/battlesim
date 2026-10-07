import { lazy, Suspense, type ComponentType, type LazyExoticComponent } from "react";
import { Link, useLocation } from "react-router";
import { PageVisit } from "./navigation";
import { LAB_FIXTURES } from "./fixtures";
import { listMaps } from "@web/maps/catalogue";
import { LabLoading } from "./LabLoading";
import { MainMenu } from "./MainMenu";
import { SessionCatalogScope } from "@web/battle/catalog/context";

const MechanicsEditor = lazy(() => import("../../mechanics-editor/src/MechanicsEditor"));
const MapWorkbench = lazy(() => import("../../map-workbench/src/MapWorkbench"));
const SoundWorkbench = lazy(() => import("../../sound-workbench/src/SoundWorkbench"));

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
  street: lazy(() => import("./routes/street")),
  generated: lazy(() => import("./routes/battle")),
  benchmark: lazy(() => import("./routes/benchmark")),
  endurance: lazy(() => import("./routes/endurance")),
  lean: lazy(() => import("./routes/lean")),
  fog: lazy(() => import("./routes/fog")),
  projectiles: lazy(() => import("./routes/projectiles")),
  workbench: lazy(() => import("./routes/workbench")),
  "fog-look": lazy(() => import("./routes/fogLook")),
  sound: lazy(() => import("./routes/sound")),
};

/** `/` is the main menu, `/labs` the index of every fixture route. */
export function LabRouter() {
  const location = useLocation();
  const path = location.pathname;
  return (
    <PageVisit>
      <LabPage path={path} />
    </PageVisit>
  );
}

const EDITORS: Record<string, LazyExoticComponent<ComponentType>> = {
  "/sound-workbench": SoundWorkbench,
  "/map-workbench": MapWorkbench,
  "/mechanics": MechanicsEditor,
};

function resolvePage(path: string) {
  const normalized = path.replace(/\/$/, "") || "/";
  const editor = import.meta.env.DEV ? EDITORS[normalized] : undefined;
  const fixture = LAB_FIXTURES.find((f) => f.route === normalized);
  const Route = fixture && ROUTES[fixture.id];
  return { path: normalized, editor, fixture, Route };
}

/** Policy for menu warm-up and the menu bed, derived from the rendered page.
 * Player battle/replay routes keep the bed while their loading cover is visible. */
export function screenForPath(path: string): "menu" | "loading" | "other" {
  const page = resolvePage(path);
  if (page.editor || page.path === "/labs") return "other";
  if (!page.Route) return "menu";
  return /^\/(battle|replay)(\/|$)/.test(page.path) ? "loading" : "other";
}

function LabPage({ path: requestedPath }: { path: string }) {
  const { path, editor: Editor, fixture, Route } = resolvePage(requestedPath);
  if (Editor)
    return (
      <Suspense fallback={null}>
        <Editor />
      </Suspense>
    );
  if (!Route && path !== "/labs") return <MainMenu />;
  if (!fixture || !Route) {
    return (
      <main
        className="lab-index"
        style={{ padding: 24, height: "100%", overflow: "auto", boxSizing: "border-box" }}
      >
        <p>
          <Link to="/">Main menu</Link>
        </p>
        <h1>Battle lab</h1>
        <h2>Saved maps</h2>
        <ul>
          {listMaps()
            .filter((map) => map.status !== "retired")
            .map((map) => (
              <li key={map.id}>
                <Link to={`/lab/geometry?map=${map.id}`}>{map.label}</Link>
                {map.status === "draft" && " — draft"}
              </li>
            ))}
        </ul>
        <h2>Fixtures and tools</h2>
        <ul>
          {LAB_FIXTURES.map((f) => (
            <li key={f.id}>
              <Link to={f.route}>{f.id}</Link> — {f.describe}
            </li>
          ))}
        </ul>
      </main>
    );
  }
  // Player battles and replays run the game's catalog; labs run the test
  // set, which adds the test units their fixtures field.
  const page = (
    <SessionCatalogScope set={fixture.id === "generated" ? "game" : "test"}>
      <Route />
    </SessionCatalogScope>
  );
  return fixture.id === "generated" || fixture.id === "sound" ? (
    <Suspense fallback={null}>{page}</Suspense>
  ) : (
    <LabLoading key={path} subject={fixture.id.toUpperCase()}>
      {page}
    </LabLoading>
  );
}
