/** The range ruler's text (Space held with a selection), in the holo
 *  callouts' style: past the cursor's end, on the far side from the unit
 *  so the line never runs through it, the distance in metres; and at each
 *  tick on the line, the name and range of the weapons whose reach ends
 *  there. A reach past the cursor has no tick: the line is lit to the
 *  cursor. Placed by the viewport every frame, straight into the DOM. */
import { useImperativeHandle, useRef, type Ref } from "react";
import type { RangeRuler, RulerMark } from "./rangeRuler";
import type { Project } from "./readouts";

export interface RangeRulerLabelsHandle {
  /** Show `ruler` (null hides it), projected by `project`. */
  place(project: Project, ruler: RangeRuler | null): void;
}

/** The readout's gap from the cursor's end, and a tick label's from its
 *  tick, in CSS pixels. */
const READOUT_PX = 16;
const TICK_LABEL_PX = 16;

const markName = (m: RulerMark) => m.names.join(" · ").toUpperCase();
const metresText = (m: number) => `${Math.round(m).toLocaleString("en-US")} m`;

export function RangeRulerLabels({ handle }: { handle: Ref<RangeRulerLabelsHandle> }) {
  const layer = useRef<HTMLDivElement>(null);
  const readout = useRef<HTMLDivElement>(null);
  /** What the DOM last showed, so an unchanged frame writes nothing. */
  const shown = useRef("");
  useImperativeHandle(handle, () => ({
    place(project, ruler) {
      const root = layer.current,
        box = readout.current;
      if (!root || !box) return;
      const end = ruler && project(ruler.to[0], ruler.to[1], ruler.to[2]);
      const start = ruler && project(ruler.from[0], ruler.from[1], ruler.from[2]);
      if (!ruler || !end || !start) {
        root.style.display = "none";
        shown.current = "";
        return;
      }
      root.style.display = "";
      // The text: rebuilt only when it changes.
      const key = `${ruler.unit}|${Math.round(ruler.distance_m)}|${ruler.marks.map((m) => `${m.range_m}${m.inRange ? "+" : "-"}`).join()}`;
      const ticks = [...root.querySelectorAll<HTMLElement>(".rr-tick")];
      if (key !== shown.current) {
        shown.current = key;
        box.textContent = metresText(ruler.distance_m);
        for (const t of ticks) t.remove();
        for (const m of ruler.marks)
          if (m.along_m !== null) {
            const t = span("rr-tick", `${markName(m)} ${m.range_m} m`);
            t.dataset.range = String(m.range_m);
            root.append(t);
          }
      }
      const [dx, dy] = [end[0] - start[0], end[1] - start[1]];
      const len = Math.hypot(dx, dy) || 1;
      // The readout continues the line past its end: its near edge the gap
      // beyond the end, on whichever side the line leaves, and centred on
      // the end's height, so the line points at it and never crosses it.
      const [ux, uy] = [dx / len, dy / len];
      const [bw, bh] = [box.offsetWidth, box.offsetHeight];
      const ax = end[0] + ux * READOUT_PX,
        ay = end[1] + uy * READOUT_PX;
      const x0 = ux >= 0 ? ax : ax - bw;
      const y0 = Math.abs(uy) > Math.abs(ux) ? (uy >= 0 ? ay : ay - bh) : ay - bh / 2;
      box.style.transform = `translate(${x0}px, ${y0}px)`;
      // Each tick's label sits off the line on its upper side on screen.
      let [nx, ny] = [-dy / len, dx / len];
      if (ny > 0) [nx, ny] = [-nx, -ny];
      const ground = Math.hypot(ruler.to[0] - ruler.from[0], ruler.to[1] - ruler.from[1]) || 1;
      const onLine = ruler.marks.filter((m) => m.along_m !== null);
      root.querySelectorAll<HTMLElement>(".rr-tick").forEach((t, i) => {
        const m = onLine[i];
        const s = m ? m.along_m! / ground : 0;
        const p =
          m &&
          project(
            ruler.from[0] + (ruler.to[0] - ruler.from[0]) * s,
            ruler.from[1] + (ruler.to[1] - ruler.from[1]) * s,
            ruler.from[2] + (ruler.to[2] - ruler.from[2]) * s,
          );
        t.style.display = p ? "" : "none";
        if (p)
          t.style.transform = `translate(${p[0] + nx * TICK_LABEL_PX}px, ${p[1] + ny * TICK_LABEL_PX}px) translate(-50%, -50%)`;
      });
    },
  }));
  return (
    <div className="rr-layer" ref={layer} data-testid="range-ruler" style={{ display: "none" }}>
      <div className="rr-distance" ref={readout} />
    </div>
  );
}

function span(className: string, text: string) {
  const e = document.createElement("span");
  e.className = className;
  e.textContent = text;
  return e;
}
