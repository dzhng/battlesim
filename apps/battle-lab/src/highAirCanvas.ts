import { HIGH_PLANE_ATTR } from "@web/battle/present/readouts";

/** The high-air models' canvas, drawn by the same frame as `world`: laid over
 *  it between the card planes once the page has them (`HIGH_PLANE_ATTR`), so a
 *  high-air model rises above every ground and low-air card. A viewport
 *  without one (control plates, which a second client's single picture is
 *  held to) draws the models with the world. Removed when `signal` aborts. */
export function highAirCanvas(world: HTMLCanvasElement, signal: AbortSignal) {
  const canvas = document.createElement("canvas");
  Object.assign(canvas.style, { position: "absolute", pointerEvents: "none" });
  signal.addEventListener("abort", () => canvas.remove());
  let at = "";
  /** Follow the world canvas into the high plane, each frame. */
  const place = () => {
    const host = document.querySelector(`[${HIGH_PLANE_ATTR}]`) ?? world.parentElement!;
    if (canvas.parentElement !== host) host.appendChild(canvas);
    const r = world.getBoundingClientRect();
    const h = host.getBoundingClientRect();
    const next = `${r.left - h.left} ${r.top - h.top} ${r.width} ${r.height}`;
    if (next === at) return;
    at = next;
    Object.assign(canvas.style, {
      left: `${r.left - h.left}px`,
      top: `${r.top - h.top}px`,
      width: `${r.width}px`,
      height: `${r.height}px`,
    });
  };
  return { canvas, place };
}
