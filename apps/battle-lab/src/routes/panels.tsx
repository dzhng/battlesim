import { useLabLoading } from "../LabLoading";
// The panel workbench: every info panel the battle can draw (the specimens
// in `panelSpecimens.ts`), through the battle's own `InfoPanel`, on a plain
// ground and one row over grass; `?scale=` zooms the page. No 3D scene: the
// panels are DOM over the battlefield. The scene (`web/scenes/panels.mjs`)
// shoots the contact sheet.
import game from "@fixtures/game.json";
import { InfoPanel, PanelCallout } from "@web/battle/present/infoPanel";
import type { PanelRules } from "@web/battle/present/panelRows";
import { useEffect } from "react";
import { panelSpecimens, type Specimen } from "../panelSpecimens";

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
  useLabLoading("renderer", true);
  const params = new URLSearchParams(location.search);
  const scale = Number(params.get("scale") ?? 1) || 1;
  const specimens = panelSpecimens(RULES);
  const groups = [...new Set(specimens.map((s) => s.group))];
  useEffect(() => {
    window.__lab = { ready: true, fixture: "panels", error: null, frame: async () => {} };
    return () => void delete window.__lab;
  }, []);
  return (
    <main className="pw" style={{ zoom: scale }}>
      <style>{CSS}</style>
      <header>
        <strong>Info panels</strong> · {specimens.length} panels
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
