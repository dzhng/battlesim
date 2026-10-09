import { useLabLoading } from "../LabLoading";
// The panel workbench: every info panel the battle can draw (the specimens
// in `panelSpecimens.ts`), through the battle's own `InfoPanel`, on a plain
// ground and one row over grass; `?scale=` zooms the page. No 3D scene: the
// panels are DOM over the battlefield. The scene (`web/scenes/panels.mjs`)
// shoots the contact sheet.
import game from "@fixtures/game.json";
import { InfoPanel, PanelCallout } from "@web/battle/present/infoPanel";
import type { PanelRules } from "@web/battle/present/panelRows";
import { useEffect, useState } from "react";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { panelSpecimens, specimenUnit, type Specimen } from "../panelSpecimens";

import { CommandBar } from "@web/battle/present/readouts";
import { ArmyDeck } from "@web/battle/present/armyDeck";
import { PurchasePicker } from "@web/battle/present/purchasePicker";
import { CaptionList } from "@web/battle/present/captions";

const RULES = game as unknown as PanelRules;

/** The specimens shown again over grass: one of each kind of panel. */
const OVER_GRASS = [
  "own, busiest/squad in a fight",
  "own weapons/tank loading HE",
  "own states/supplying",
  "enemy identified/tank",
  "heard/tank_ap + tank_he + hmg",
  "last seen/rifle, 18 s ago",
  "key cases/two launchers, separate reloads",
  "key cases/two launchers, enemy equipment",
  "key cases/turret and hull HMG",
];

function Card({ s }: { s: Specimen }) {
  return (
    <figure className="pw-card" data-specimen={s.id}>
      <figcaption>{s.label}</figcaption>
      <PanelCallout owner={s.owner} selected={s.selected}>
        <InfoPanel panel={s.panel} zoom={s.zoom} />
      </PanelCallout>
    </figure>
  );
}

export default function Panels() {
  const [deck, setDeck] = useState(false);
  useLabLoading("renderer", true);
  const params = new URLSearchParams(location.search);
  const scale = Number(params.get("scale") ?? 1) || 1;
  const catalog = useSessionCatalog();
  const specimens = panelSpecimens(catalog, RULES);
  const groups = [...new Set(specimens.map((s) => s.group))];
  useEffect(() => {
    window.__lab = { ready: true, fixture: "panels", error: null, frame: async () => {} };
    return () => void delete window.__lab;
  }, []);
  if (deck) return <DeckReview onBack={() => setDeck(false)} />;
  return (
    <main className="pw" style={{ zoom: scale }}>
      <style>{CSS}</style>
      <header>
        <strong>Info panels</strong> · {specimens.length} panels{" "}
        <button onClick={() => setDeck(true)}>Review battle deck</button>
      </header>
      {groups.map((g) => (
        <section key={g}>
          <h2>{g}</h2>
          <div className="pw-grid">
            {specimens
              .filter((s) => s.group === g)
              .map((s) => (
                <Card key={s.id} s={s} />
              ))}
          </div>
        </section>
      ))}
      <section className="pw-grass">
        <h2>over grass</h2>
        <div className="pw-grid">
          {OVER_GRASS.map((id) => specimens.find((s) => s.id === id))
            .filter((s): s is Specimen => !!s)
            .map((s) => (
              <Card key={s.id} s={s} />
            ))}
        </div>
      </section>
    </main>
  );
}

/** The real selection and command components over controlled presentation data. */
function DeckReview({ onBack }: { onBack: () => void }) {
  const catalog = useSessionCatalog();
  const cases: {
    name: string;
    units: ReturnType<typeof specimenUnit>[];
    replay?: boolean;
    selected?: number[];
  }[] = [
    { name: "Rifle squad", units: [specimenUnit(catalog, "test_rifle")] },
    { name: "Tank ammunition", units: [specimenUnit(catalog, "test_tank")] },
    {
      name: "Suppressed and resupplying",
      units: [
        specimenUnit(catalog, "test_rifle", { suppression: "suppressed", service: "serving" }),
      ],
    },
    {
      name: "Deploying supply truck",
      units: [
        specimenUnit(catalog, "test_supply", {
          deployment: { progress: 0.4, target: "deployed" },
          stock: 250,
        }),
      ],
    },
    {
      name: "Mixed capabilities",
      units: [
        specimenUnit(catalog, "test_tank", { id: 1 }),
        specimenUnit(catalog, "test_supply", { id: 2 }),
      ],
    },
    {
      name: "Garrison exit",
      units: [
        specimenUnit(catalog, "test_rifle", {
          garrison: { building: 3, phase: "inside", progress: 1, center: [0, 0], half: [10, 10] },
        }),
      ],
    },
    {
      name: "Large selection",
      units: ["test_tank", "test_rifle", "test_at", "test_supply", "test_recon", "test_jeep"].map(
        (kind, id) => specimenUnit(catalog, kind, { id }),
      ),
    },
    {
      name: "Army roster",
      units: [
        "test_tank",
        "test_rifle",
        "test_at",
        "test_supply",
        "test_recon",
        "test_jeep",
        "test_rifle",
        "test_at",
        "test_tank",
        "test_rifle",
        "test_recon",
        "test_supply",
      ].map((kind, id) => specimenUnit(catalog, kind, { id: id + 1 })),
      selected: [1, 2, 3, 4, 5, 6],
    },
    {
      name: "Entire force",
      units: Array.from({ length: 48 }, (_, id) =>
        specimenUnit(catalog, "test_rifle", { id, suppression: "suppressed", service: "serving" }),
      ),
    },
    {
      name: "No selection",
      units: [specimenUnit(catalog, "test_rifle"), specimenUnit(catalog, "test_tank", { id: 2 })],
      selected: [],
    },
    { name: "Replay", units: [specimenUnit(catalog, "test_tank")], replay: true },
  ];
  const [chosen, setChosen] = useState(0);
  const [mode, setMode] = useState<Parameters<typeof CommandBar>[0]["control"]["mode"]>("move");
  const [action, setAction] = useState("");
  const selection = cases[chosen];
  const [selected, setSelected] = useState<number[]>([1]);
  const control = {
    selectedUnits: selection.units.filter((unit) => selected.includes(unit.id)),
    mode,
    setMode,
    stop: () => setAction("Stop"),
    togglePolicy: () => setAction("Fire policy"),
    toggleDeployment: () => setAction("Deployment"),
    exitBuilding: () => setAction("Leave building"),
  };
  const skirmish = {
    phase: "preparation" as const,
    ready: [false, false] as [boolean, boolean],
    preparationRemainingS: 60,
    credits: 769,
    occupiedSlots: selection.units.length,
    maxUnits: 30,
    pending: [],
    scores: [0, 0] as [number, number],
    result: null,
    objectives: [],
  };
  return (
    <main
      style={{ height: "100%", background: "linear-gradient(#324227, #526441 65%, #62625d 65%)" }}
    >
      <nav
        style={{
          position: "absolute",
          top: 16,
          left: 16,
          display: "flex",
          flexWrap: "wrap",
          gap: 8,
        }}
      >
        <button onClick={onBack}>Info panels</button>
        {cases.map((c, index) => (
          <button
            key={c.name}
            aria-pressed={index === chosen}
            onClick={() => {
              setChosen(index);
              setSelected(c.selected ?? c.units.map((unit) => unit.id));
              setMode("move");
            }}
          >
            {c.name}
          </button>
        ))}
      </nav>
      <output style={{ position: "absolute", top: 90, left: 16 }}>{action}</output>
      <div className="hud">
        <ArmyDeck
          own={selection.units}
          selected={selected}
          onSelect={setSelected}
          rules={RULES}
          control={selection.replay ? undefined : control}
          captions={
            <CaptionList
              captions={{
                captions: [
                  {
                    key: "heard",
                    text: "Heard gunfire, nearby, east of Rifle squad",
                    count: 2,
                    tick: 0,
                  },
                ],
                note: () => {},
                clear: () => {},
              }}
            />
          }
          reinforcements={
            <PurchasePicker cards={[]} faction="us" match={skirmish} onChoose={() => {}} />
          }
        />
      </div>
    </main>
  );
}

/** The workbench's own page chrome (lab grey, never the HUD's look). */
const CSS = `
.pw { padding: 16px 20px 32px; background: #2b2e33; min-height: 100%; box-sizing: border-box;
  font: 12px ui-monospace, Menlo, monospace; color: #9aa3ad; overflow: auto; height: 100%; }
.pw header { margin-bottom: 8px; }
.pw h2 { font-size: 11px; font-weight: 400; text-transform: uppercase; letter-spacing: 0.1em;
  margin: 14px 0 6px; color: #7d8791; border-bottom: 1px solid #3c4148; padding-bottom: 2px; }
.pw-grid { display: flex; flex-wrap: wrap; gap: 6px 10px; align-items: flex-start; }
.pw-card { margin: 0; padding: 4px 8px 8px; min-width: 150px; }
.pw-card figcaption { font-size: 10px; color: #7d8791; margin-bottom: 4px; white-space: nowrap; }
.pw-card .ro-unit { position: relative; display: inline-flex; }
.pw-grass { margin-top: 18px; padding: 4px 8px 12px;
  background: radial-gradient(circle at 30% 40%, #5c7a3a, #46632b 60%, #3d5725); }
.pw-grass h2 { color: #d7e3c8; border-color: #6d8a4c; }
.pw-grass figcaption { color: #d7e3c8; }
`;
