import { Link } from "react-router";
import { useLabLoading } from "../LabLoading";
// The real cursor at native screen size, isolated from picking and simulation.
import { useLayoutEffect, useRef } from "react";
import {
  CURSOR_ACTIONS,
  CursorPicture,
  useCursorAction,
  type CursorAction,
} from "@web/battle/present/gameCursor";
import "./cursor.css";

const LABELS: Record<CursorAction, string> = {
  default: "Move",
  attack: "Attack",
  attack_move: "Attack move",
  attack_ground: "Attack ground",
  garrison: "Garrison",
  fast_move: "Fast move",
  reverse_move: "Reverse move",
  blocked: "Blocked",
};
const surfaces = ["grass", "road", "fog"] as const;

export default function CursorLab() {
  useLabLoading("renderer", true);
  const setCursorAction = useCursorAction();
  const action = useRef<CursorAction>("default");
  useLayoutEffect(() => {
    window.__lab = { ready: true, error: null, frame: async () => {} };
    return () => {
      delete window.__lab;
    };
  }, []);
  return (
    <main className="cursor-lab">
      <header>
        <strong>Arrow + action</strong>
        <Link to="/labs">Battle lab</Link>
      </header>
      {surfaces.map((surface) => (
        <section key={surface} className={`cursor-surface cursor-${surface}`}>
          <h2>{surface}</h2>
          <div className="cursor-grid">
            {CURSOR_ACTIONS.map((action) => {
              const id = `${surface}-${action}`;
              return (
                <figure key={action} data-specimen={id} data-action={action}>
                  <span className="cursor-tip">
                    <CursorPicture action={action} x={0} y={0} />
                  </span>
                  <figcaption>{LABELS[action]}</figcaption>
                </figure>
              );
            })}
          </div>
        </section>
      ))}
      <section className="cursor-demo">
        <h2>Move across the ground to try the cursor</h2>
        <div className="cursor-choices">
          {CURSOR_ACTIONS.map((name) => (
            <button
              key={name}
              onClick={() => {
                action.current = name;
              }}
            >
              {LABELS[name]}
            </button>
          ))}
        </div>
        <div
          className="cursor-playfield"
          data-testid="cursor-playfield"
          onPointerMove={() => setCursorAction(action.current)}
          onPointerLeave={() => setCursorAction("default")}
          onPointerCancel={() => setCursorAction("default")}
        >
          <span>Grass / road / fog</span>
        </div>
      </section>
    </main>
  );
}
