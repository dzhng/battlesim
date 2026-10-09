import { Link } from "react-router";
import { useLabLoading } from "../LabLoading";
// The real cursor at native screen size, isolated from picking and simulation.
import { useLayoutEffect, useRef } from "react";
import {
  GameCursor,
  useCursorAction,
  type CursorAction,
  type GameCursorHandle,
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
const actions = Object.keys(LABELS) as CursorAction[];
const surfaces = ["grass", "road", "fog"] as const;

export default function CursorLab() {
  useLabLoading("renderer", true);
  const sheet = useRef<HTMLElement>(null);
  const handles = useRef(new Map<string, GameCursorHandle>());
  const setCursorAction = useCursorAction();
  const action = useRef<CursorAction>("default");
  useLayoutEffect(() => {
    const place = () => {
      for (const specimen of sheet.current!.querySelectorAll<HTMLElement>("[data-specimen]")) {
        const box = specimen.getBoundingClientRect();
        handles.current
          .get(specimen.dataset.specimen!)
          ?.place(
            { x: box.left + box.width / 2 - 16, y: box.top + 14 },
            specimen.dataset.action as CursorAction,
          );
      }
    };
    place();
    window.addEventListener("resize", place);
    window.addEventListener("scroll", place, true);
    window.__lab = {
      ready: true,
      fixture: "cursor",
      error: null,
      frame: async () => place(),
    };
    return () => {
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", place, true);
      delete window.__lab;
    };
  }, []);
  return (
    <main ref={sheet} className="cursor-lab">
      <header>
        <strong>Arrow + action</strong>
        <Link to="/labs">Battle lab</Link>
      </header>
      {surfaces.map((surface) => (
        <section key={surface} className={`cursor-surface cursor-${surface}`}>
          <h2>{surface}</h2>
          <div className="cursor-grid">
            {actions.map((action) => {
              const id = `${surface}-${action}`;
              return (
                <figure key={action} data-specimen={id} data-action={action}>
                  <GameCursor
                    handle={(handle) => {
                      if (handle) handles.current.set(id, handle);
                      else handles.current.delete(id);
                    }}
                  />
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
          {actions.map((name) => (
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
