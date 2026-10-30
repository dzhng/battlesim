import { lazy, Suspense, type ComponentType, type LazyExoticComponent } from "react";
import { Link, Navigate, useLocation } from "react-router";
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
  capture: lazy(() => import("./routes/capture")),
  "graphics-test": lazy(() => import("./routes/graphicsTest")),
  cursor: lazy(() => import("./routes/cursor")),
  frame: lazy(() => import("./routes/frame")),
  "geometry-scaling": lazy(() => import("./routes/geometry")),
  geometry: lazy(() => import("./routes/geometry")),
  authority: lazy(() => import("./routes/authority")),
  "ally-help": lazy(() => import("./routes/allyHelp")),
  movement: lazy(() => import("./routes/movement")),
  river: lazy(() => import("./routes/river")),
  sensors: lazy(() => import("./routes/sensors")),
  ballistics: lazy(() => import("./routes/ballistics")),
  artillery: lazy(() => import("./routes/ballistics")),
  weapons: lazy(() => import("./routes/weapons")),
  consequences: lazy(() => import("./routes/consequences")),
  "air-hover": lazy(() => import("./routes/airHover")),
  "air-crash": lazy(() => import("./routes/airCrash")),
  "air-high": lazy(() => import("./routes/airWar")),
  "air-defense": lazy(() => import("./routes/airWar")),
  "air-dogfight": lazy(() => import("./routes/airWar")),
  "air-ingress": lazy(() => import("./routes/airWar")),
  "air-ground": lazy(() => import("./routes/airWar")),
  "air-survivability": lazy(() => import("./routes/airWar")),
  "air-roster": lazy(() => import("./routes/airWar")),
  "air-hud": lazy(() => import("./routes/airWar")),
  "air-audio": lazy(() => import("./routes/airWar")),
  garrison: lazy(() => import("./routes/garrison")),
  transport: lazy(() => import("./routes/transport")),
  supply: lazy(() => import("./routes/supply")),
  ground: lazy(() => import("./routes/ground")),
  panels: lazy(() => import("./routes/panels")),
  ui: lazy(() => import("./routes/ui")),
  readouts: lazy(() => import("./routes/readouts")),
  camera: lazy(() => import("./routes/camera")),
  facade: lazy(() => import("./routes/facade")),
  "city-lineup": lazy(() => import("./routes/cityLineup")),
  "city-ruins": lazy(() => import("./routes/cityRuins")),
  street: lazy(() => import("./routes/street")),
  generated: lazy(() => import("./routes/battle")),
  benchmark: lazy(() => import("./routes/benchmark")),
  endurance: lazy(() => import("./routes/endurance")),
  fog: lazy(() => import("./routes/fog")),
  projectiles: lazy(() => import("./routes/projectiles")),
  workbench: lazy(() => import("./routes/workbench")),
  "fog-look": lazy(() => import("./routes/fogLook")),
  sound: lazy(() => import("./routes/sound")),
  scenario: lazy(() => import("./routes/scenario")),
  "map-picture": lazy(() => import("./routes/mapPicture")),
};

/** Canonical URLs for the developer fixture surface all live below `/labs`. */
export function canonicalLabPath(path: string): string {
  if (path.startsWith("/lab/")) return `/labs/${path.slice("/lab/".length)}`;
  if (path.startsWith("/labs/")) return `/labs/${path.slice("/labs/".length)}`;
  const legacy = [
    "/graphics-test",
    "/benchmark",
    "/workbench",
    "/map-workbench",
    "/mechanics",
    "/scenario",
    "/sound-workbench",
  ];
  return legacy.includes(path) ? `/labs${path}` : path;
}

/** `/` is the main menu, `/labs` the index of every fixture route. */
export function LabRouter() {
  const location = useLocation();
  const path = canonicalLabPath(location.pathname);
  if (path !== location.pathname)
    return <Navigate replace to={`${path}${location.search}${location.hash}`} />;
  return (
    <PageVisit>
      <LabPage path={path} />
    </PageVisit>
  );
}

const EDITORS: Record<string, LazyExoticComponent<ComponentType>> = {
  "/labs/sound-workbench": SoundWorkbench,
  "/labs/map-workbench": MapWorkbench,
  "/labs/mechanics": MechanicsEditor,
};

function resolvePage(path: string) {
  const normalized = canonicalLabPath(path.replace(/\/$/, "") || "/");
  const editor = import.meta.env.DEV ? EDITORS[normalized] : undefined;
  // A page is owned by the fixture whose route is its bare path; fixtures that
  // add a query (a variant, a tool's workload) play on that owner's page.
  const fixture = LAB_FIXTURES.find((f) => f.route === normalized);
  const Route = fixture && ROUTES[fixture.id];
  return { path: normalized, editor, fixture, Route };
}

/** Policy for menu warm-up and the menu bed, derived from the rendered page.
 * The player battle route (replays included) keeps the bed while its loading
 * cover is visible. */
export function screenForPath(path: string): "menu" | "loading" | "other" {
  const page = resolvePage(path);
  if (page.editor || page.path === "/labs") return "other";
  if (!page.Route) return "menu";
  return /^\/battle(\/|$)/.test(page.path) ? "loading" : "other";
}

export function labIndexSections() {
  return {
    scenes: LAB_FIXTURES.filter((fixture) => !fixture.tool),
    tools: LAB_FIXTURES.filter((fixture) => fixture.tool),
  };
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
    const { scenes, tools } = labIndexSections();
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
                <Link to={`/labs/geometry?map=${map.id}`}>{map.label}</Link>
                {map.status === "draft" && " — draft"}
              </li>
            ))}
        </ul>
        <h2>Scenes</h2>
        <p>
          These registered scenes are the test harness; every scene passes at implementation
          closeout.
        </p>
        <ul>
          {scenes.map((f) => (
            <li key={f.id}>
              <Link to={f.route}>{f.id}</Link> — {f.describe}
            </li>
          ))}
        </ul>
        <h2>Tools</h2>
        <p>Tools are human lab pages and measurements; they run when opened by id.</p>
        <ul>
          {tools.map((f) => (
            <li key={f.id}>
              <Link to={f.route}>{f.id}</Link> — {f.describe}
            </li>
          ))}
          {import.meta.env.DEV &&
            [
              ["Mechanics editor", "/labs/mechanics"],
              ["Map workbench", "/labs/map-workbench"],
              ["Sound workbench", "/labs/sound-workbench"],
            ].map(([label, href]) => (
              <li key={href}>
                <Link to={href}>{label}</Link> — development editor
              </li>
            ))}
        </ul>
      </main>
    );
  }
  const catalog = fixture.catalog ?? "test";
  const page = (
    <SessionCatalogScope set={catalog}>
      <Route />
    </SessionCatalogScope>
  );
  return fixture.loading === false ? (
    <Suspense fallback={null}>{page}</Suspense>
  ) : (
    <LabLoading key={path} subject={fixture.id.toUpperCase()}>
      {page}
    </LabLoading>
  );
}
