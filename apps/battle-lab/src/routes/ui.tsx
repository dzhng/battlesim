import { useLabLoading } from "../LabLoading";
// The UI gallery: every piece of the battle's interface that the panel
// workbench (`panels.tsx`) and cursor lab do not already draw, each in fixed
// states through the game's own components over a plain battlefield
// stand-in. `?shot=<id>` draws one shot full-viewport, composed as the battle
// composes it (its overlays are fixed to the viewport, so one shot per page);
// without it the page lists them (the list the scene reads). The scene (`web/scenes/ui.mjs`) matches
// every shot against its approved picture. Test units only: the roster grows.
import game from "@fixtures/game.json";
import { Link } from "react-router";
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import type { UnitCard, UnitCategory } from "@packages/scene-assets/src/units";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { ArmyDeck } from "@web/battle/present/armyDeck";
import { CaptionList, type CaptionLine } from "@web/battle/present/captions";
import { FrameRate, type FrameRateHandle } from "@web/battle/present/frameRate";
import { GameCursor, type GameCursorHandle } from "@web/battle/present/gameCursor";
import {
  ObjectiveMarkers,
  type ObjectiveMarkersHandle,
} from "@web/battle/present/objectiveMarkers";
import type { PanelRules } from "@web/battle/present/panelRows";
import { PurchasePicker } from "@web/battle/present/purchasePicker";
import {
  RangeRulerLabels,
  type RangeRulerLabelsHandle,
} from "@web/battle/present/rangeRulerLabels";
import type { RangeRuler } from "@web/battle/present/rangeRuler";
import type { Project } from "@web/battle/present/readouts";
import { RejectedOrder } from "@web/battle/present/rejectedOrder";
import type { AckEntry } from "@web/battle/input/useUnitControl";
import type { ObjectiveView, SkirmishView } from "@web/battle/sim/observation";
import { BattleClock, SkirmishStatus } from "../battleStatus";
import { LoadingScreen } from "../LoadingScreen";
import { MenuButton, PauseMenu } from "../PauseMenu";
import { specimenUnit } from "../panelSpecimens";

const RULES = game as unknown as PanelRules;

const MATCH: SkirmishView = {
  phase: "active",
  ready: [true, true],
  preparationRemainingS: 0,
  credits: 769,
  scores: [120, 85],
  result: null,
  objectives: [],
  occupiedSlots: 6,
  maxUnits: 30,
  pending: [],
};

/** A purchasable card for a test unit (its silhouette is the unit's). */
const card = (
  id: string,
  category: UnitCategory,
  family_name: string,
  variant: string,
  cost: number,
  disabled_reason: string | null = null,
): UnitCard => ({
  id,
  name: `${family_name} ${variant}`,
  description: "",
  family: family_name.toLowerCase().replaceAll(" ", "_"),
  cost,
  roster: { factions: ["us"], family_name, category, variant },
  disabled_reason,
  planned_weapons: [],
});

const CARDS: UnitCard[] = [
  card("test_recon", "rec", "Recon Team", "Base", 120),
  card("test_rifle", "inf", "Rifle Section", "Base", 100),
  card("test_at", "inf", "AT Team", "Base", 150),
  card("test_tank", "veh", "Tank", "Base", 400),
  card("test_tank_trophy", "veh", "Tank", "Trophy", 520, "Protection deferred"),
  card("test_jeep", "veh", "Jeep", "Base", 80),
  card("test_supply", "sup", "Supply Truck", "Base", 90),
];

/** World metres to the screen: a flat map seen from straight above. */
const project: Project = (x, y) => [640 + x * 3, 330 + y * 3];

const OBJECTIVES: ObjectiveView[] = [
  {
    id: "A",
    center: [-120, -30],
    radiusM: 20,
    owner: null,
    capturing: null,
    captureProgress: 0,
    contested: false,
  },
  {
    id: "B",
    center: [-40, 20],
    radiusM: 20,
    owner: "blue",
    capturing: null,
    captureProgress: 1,
    contested: false,
  },
  {
    id: "C",
    center: [40, -20],
    radiusM: 20,
    owner: "red",
    capturing: "blue",
    captureProgress: 0.4,
    contested: false,
  },
  {
    id: "D",
    center: [120, 30],
    radiusM: 20,
    owner: "blue",
    capturing: "red",
    captureProgress: 0.6,
    contested: true,
  },
];

const RULER: RangeRuler = {
  unit: 1,
  from: [-110, 40, 0],
  to: [90, -30, 0],
  distance_m: 212,
  marks: [
    { names: ["Rifle"], range_m: 300, inRange: true, along_m: null },
    { names: ["ATGM 1", "ATGM 2"], range_m: 150, inRange: false, along_m: 150 },
  ],
};

const HEARD: CaptionLine[] = [
  { key: "a", tick: 0, count: 3, text: "Heard gunfire, nearby, east of Rifle squad" },
  { key: "b", tick: 0, count: 1, text: "Heard a tank engine, far, north of Tank #2" },
  { key: "c", tick: 0, count: 1, text: "Heard an explosion, nearby, south of AT Team #4" },
];

const STAGES = [
  { id: "map", label: "Generating the map" },
  { id: "world", label: "Building the world" },
  { id: "units", label: "Deploying units" },
];

/** The battle's own top strip, menu button and (in preparation) ready button. */
function Status({ match }: { match: SkirmishView }) {
  return (
    <div className="hud">
      <header className="hud-panel hud-top" data-occludes-readouts>
        <SkirmishStatus match={match} />
      </header>
      {match.phase === "preparation" && (
        <div className="hud-ready-layer" data-occludes-readouts>
          <button
            type="button"
            className="hud-menu-choice hud-purchase-ready"
            disabled={match.ready[0]}
          >
            {match.ready[0] ? "READY" : "START BATTLE"}
          </button>
        </div>
      )}
      <MenuButton onOpen={() => {}} />
    </div>
  );
}

/** The deck with its reinforcements, as a battle in preparation shows it. */
function Purchase({ match }: { match: SkirmishView }) {
  const catalog = useSessionCatalog();
  const own = ["test_tank", "test_rifle", "test_at"].map((kind, i) =>
    specimenUnit(catalog, kind, { id: i + 1 }),
  );
  return (
    <div className="hud">
      <ArmyDeck
        own={own}
        selected={[1]}
        onSelect={() => {}}
        rules={RULES}
        captions={null}
        reinforcements={
          <PurchasePicker cards={CARDS} faction="us" match={match} onChoose={() => {}} />
        }
      />
    </div>
  );
}

function Objectives() {
  const handle = useRef<ObjectiveMarkersHandle>(null);
  useLayoutEffect(() => handle.current?.place(project, () => 0));
  return <ObjectiveMarkers objectives={OBJECTIVES} handle={handle} />;
}

function Ruler() {
  const handle = useRef<RangeRulerLabelsHandle>(null);
  useLayoutEffect(() => handle.current?.place(project, RULER));
  return <RangeRulerLabels handle={handle} />;
}

/** A refused order: the cursor's blocked badge and the refusal beside it. */
function Refusal() {
  const cursor = useRef<GameCursorHandle>(null);
  const [acks, setAcks] = useState<AckEntry[]>([]);
  // An effect, not a layout effect: the refusal's own pointer listener is
  // attached by then, and reads where the pointer is.
  useEffect(() => {
    window.dispatchEvent(new PointerEvent("pointermove", { clientX: 640, clientY: 400 }));
    cursor.current?.place({ x: 640, y: 400 }, "blocked");
    setAcks([
      {
        seq: 1,
        label: "Garrison",
        order: { kind: "garrison", units: [1], building: 3 },
        ack: { seq: 1, applied_tick: 0, error: { reason: "building_occupied" } },
      },
    ]);
  }, []);
  return (
    <>
      <GameCursor handle={cursor} />
      <RejectedOrder acks={acks} />
    </>
  );
}

/** A steady 60 frames a second, as the counter reads it. */
function Frames() {
  const handle = useRef<FrameRateHandle>(null);
  useLayoutEffect(() => {
    for (let frame = 0; frame <= 60; frame++) handle.current?.frame((frame * 1000) / 60, true);
  }, []);
  return <FrameRate handle={handle} />;
}

/** Every shot: its id (the baseline's name) and what it draws. */
const SHOTS: { id: string; draw: () => ReactNode }[] = [
  {
    id: "status-preparation",
    draw: () => (
      <Status
        match={{
          ...MATCH,
          phase: "preparation",
          ready: [false, false],
          preparationRemainingS: 42,
          scores: [0, 0],
        }}
      />
    ),
  },
  {
    id: "status-preparation-ready",
    draw: () => (
      <Status
        match={{
          ...MATCH,
          phase: "preparation",
          ready: [true, false],
          preparationRemainingS: 42,
          scores: [0, 0],
        }}
      />
    ),
  },
  { id: "status-active", draw: () => <Status match={{ ...MATCH, objectives: OBJECTIVES }} /> },
  {
    id: "status-won",
    draw: () => (
      <Status match={{ ...MATCH, phase: "finished", result: "blue", scores: [500, 310] }} />
    ),
  },
  {
    id: "status-lost",
    draw: () => (
      <Status match={{ ...MATCH, phase: "finished", result: "red", scores: [280, 500] }} />
    ),
  },
  {
    id: "status-draw",
    draw: () => (
      <Status match={{ ...MATCH, phase: "finished", result: "draw", scores: [500, 500] }} />
    ),
  },
  {
    id: "battle-clock",
    draw: () => (
      <div className="hud">
        <header className="hud-panel hud-top">
          <BattleClock tick={game.tick_hz * 754} />
        </header>
      </div>
    ),
  },
  {
    id: "purchase",
    draw: () => <Purchase match={{ ...MATCH, phase: "preparation", ready: [false, false] }} />,
  },
  {
    id: "purchase-short-of-credits",
    draw: () => (
      <Purchase match={{ ...MATCH, phase: "preparation", ready: [false, false], credits: 110 }} />
    ),
  },
  { id: "objectives", draw: () => <Objectives /> },
  { id: "range-ruler", draw: () => <Ruler /> },
  { id: "refused-order", draw: () => <Refusal /> },
  { id: "frame-rate", draw: () => <Frames /> },
  {
    id: "captions",
    draw: () => (
      <div className="hud">
        <div className="hud-lower">
          <CaptionList captions={{ captions: HEARD, note: () => {}, clear: () => {} }} />
        </div>
      </div>
    ),
  },
  { id: "pause-menu", draw: () => <PauseMenu onClose={() => {}} onRestart={() => {}} /> },
  { id: "pause-menu-no-restart", draw: () => <PauseMenu onClose={() => {}} /> },
  {
    id: "loading",
    draw: () => (
      <LoadingScreen
        title="Battle"
        subject="Open · small · seed 1"
        stages={STAGES}
        current="world"
      />
    ),
  },
  {
    id: "loading-failed",
    draw: () => (
      <LoadingScreen
        title="Battle"
        subject="Open · small · seed 1"
        stages={STAGES}
        current="world"
        failure={{
          message: "The map could not be generated.",
          advice: "Seed 1 stands as asked: try another seed, or report this one.",
          details: ["mapgen: no admissible layout after 8 candidates"],
        }}
      />
    ),
  },
];

export default function UiGallery() {
  useLabLoading("renderer", true);
  const shot = SHOTS.find((s) => s.id === new URLSearchParams(location.search).get("shot"));
  useEffect(() => {
    window.__lab = {
      ready: true,
      error: null,
      frame: async () => {},
    };
    return () => void delete window.__lab;
  }, []);
  if (!shot)
    return (
      <main style={{ padding: 16, font: "13px ui-monospace, Menlo, monospace" }}>
        <strong>UI gallery</strong> · {SHOTS.length} shots
        <ul data-testid="shots">
          {SHOTS.map((s) => (
            <li key={s.id}>
              <Link to={`?shot=${s.id}`}>{s.id}</Link>
            </li>
          ))}
        </ul>
      </main>
    );
  return (
    <main
      data-shot={shot.id}
      style={{ height: "100%", background: "linear-gradient(#324227, #526441 65%, #62625d 65%)" }}
    >
      {shot.draw()}
    </main>
  );
}
