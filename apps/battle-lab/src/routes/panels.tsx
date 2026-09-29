// The panel workbench: every info panel the battle can draw (the specimens
// in `panelSpecimens.ts`), through the battle's own `InfoPanel`, on a plain
// ground and one row over grass, in the layout variant `?variant=` names.
// No 3D scene: the panels are DOM over the battlefield. The scene
// (`web/scenes/panels.mjs`) shoots one contact sheet per variant.
import village from "@fixtures/village.json";
import { InfoPanel, PANEL_VARIANTS, type PanelVariant } from "@web/battle/present/infoPanel";
import type { PanelRules } from "@web/battle/present/panelRows";
import { useEffect } from "react";
import { panelSpecimens, type Specimen } from "../panelSpecimens";

const RULES = village as unknown as PanelRules;

/** The specimens shown again over grass: one of each kind of panel. */
const OVER_GRASS = [
  "own, busiest/squad in a fight",
  "own weapons/tank loading HE",
  "own states/supplying",
  "enemy identified/tank",
  "heard/tank_ap + tank_he + hmg",
  "last seen/rifle, 18 s ago",
];

function Card({ s, variant }: { s: Specimen; variant: PanelVariant }) {
  return (
    <figure className="pw-card" data-specimen={s.id}>
      <figcaption>{s.label}</figcaption>
      <div
        className={`ro-unit ro-${s.owner}${s.selected ? " ro-selected" : ""}`}
        data-owner={s.owner}
      >
        <InfoPanel panel={s.panel} variant={variant} zoom={s.zoom} />
      </div>
    </figure>
  );
}

/** A representative few, set side by side in every variant (`?compare`). */
const COMPARED = [
  "own, idle/rifle",
  "own, idle/tank",
  "own states/deployed",
  "own weapons/tank loading HE",
  "own, busiest/squad in a fight",
  "own, busiest/AT team pinned",
  "enemy identified/tank",
  "last seen/rifle, 18 s ago",
  "heard/rifle + atgm",
  "far zoom/tank, selected",
];

function Compare({ specimens }: { specimens: Specimen[] }) {
  return (
    <table className="pw-compare">
      <thead>
        <tr>
          <th />
          {PANEL_VARIANTS.map((v) => (
            <th key={v}>{v}</th>
          ))}
        </tr>
      </thead>
      <tbody>
        {COMPARED.map((id) => specimens.find((s) => s.id === id))
          .filter((s): s is Specimen => !!s)
          .map((s) => (
            <tr key={s.id}>
              <th>{s.id}</th>
              {PANEL_VARIANTS.map((v) => (
                <td key={v}>
                  <Card s={{ ...s, label: v }} variant={v} />
                </td>
              ))}
            </tr>
          ))}
      </tbody>
    </table>
  );
}

export default function Panels() {
  const params = new URLSearchParams(location.search);
  const asked = params.get("variant") as PanelVariant | null;
  const variant = asked && PANEL_VARIANTS.includes(asked) ? asked : PANEL_VARIANTS[0];
  const scale = Number(params.get("scale") ?? 1) || 1;
  const specimens = panelSpecimens(RULES);
  const groups = [...new Set(specimens.map((s) => s.group))];
  useEffect(() => {
    window.__lab = { ready: true, fixture: "panels", error: null, frame: async () => {} };
    return () => void delete window.__lab;
  }, []);
  if (params.has("compare"))
    return (
      <main className="pw" style={{ zoom: scale }}>
        <style>{CSS}</style>
        <Compare specimens={specimens} />
      </main>
    );
  return (
    <main className="pw" data-variant={variant} style={{ zoom: scale }}>
      <style>{CSS}</style>
      <header>
        <strong>Info panels</strong> · variant{" "}
        {PANEL_VARIANTS.map((v) => (
          <a key={v} href={`?variant=${v}`} aria-current={v === variant ? "page" : undefined}>
            {v}
          </a>
        ))}
        <span> · {specimens.length} panels</span>
      </header>
      {groups.map((g) => (
        <section key={g}>
          <h2>{g}</h2>
          <div className="pw-grid">
            {specimens
              .filter((s) => s.group === g)
              .map((s) => (
                <Card key={s.id} s={s} variant={variant} />
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
              <Card key={s.id} s={s} variant={variant} />
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
.pw header a { margin: 0 4px; color: #9aa3ad; }
.pw header a[aria-current] { color: #fff; font-weight: 700; }
.pw h2 { font-size: 11px; font-weight: 400; text-transform: uppercase; letter-spacing: 0.1em;
  margin: 14px 0 6px; color: #7d8791; border-bottom: 1px solid #3c4148; padding-bottom: 2px; }
.pw-grid { display: flex; flex-wrap: wrap; gap: 6px 10px; align-items: flex-start; }
.pw-card { margin: 0; padding: 4px 8px 8px; min-width: 150px; }
.pw-card figcaption { font-size: 10px; color: #7d8791; margin-bottom: 4px; white-space: nowrap; }
.pw-card .ro-unit { position: relative; display: inline-flex; }
.pw-compare { border-collapse: collapse; }
.pw-compare th { text-align: left; font-weight: 400; font-size: 10px; color: #7d8791; padding: 4px 10px; }
.pw-compare td { vertical-align: top; border-left: 1px solid #3c4148; }
.pw-grass { margin-top: 18px; padding: 4px 8px 12px;
  background: radial-gradient(circle at 30% 40%, #5c7a3a, #46632b 60%, #3d5725); }
.pw-grass h2 { color: #d7e3c8; border-color: #6d8a4c; }
.pw-grass figcaption { color: #d7e3c8; }
`;
