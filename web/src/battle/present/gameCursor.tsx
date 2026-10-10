import { createContext, useContext, useLayoutEffect, useRef, type ReactNode } from "react";
import { hudIcon, stateIcon } from "@packages/scene-assets/src/icons";
import { channels, gameHud } from "./hudTheme";
import { iconSvg } from "./icons";
import "./gameCursor.css";

const ACTION_ICONS = {
  attack: hudIcon("fire_at_will"),
  attack_move: hudIcon("attack_move"),
  attack_ground: hudIcon("attack_ground"),
  garrison: stateIcon("building"),
  fast_move: hudIcon("fast_move"),
  reverse_move: hudIcon("reverse"),
  blocked: hudIcon("rejected"),
} as const;

export type CursorAction = "default" | keyof typeof ACTION_ICONS;
export const CURSOR_ACTIONS = ["default", ...Object.keys(ACTION_ICONS)] as CursorAction[];

const CURSOR_SCALE = 0.7;
const CURSOR_ART_SIZE = { width: 41, height: 38 } as const;
/** Complete arrow and badge bounds, also used to keep callouts clear. */
export const GAME_CURSOR_SIZE = {
  width: CURSOR_ART_SIZE.width * CURSOR_SCALE,
  height: CURSOR_ART_SIZE.height * CURSOR_SCALE,
} as const;
/** The image's side and where the arrow's tip (the pointer) sits in it, in
 *  CSS pixels. 32 is the largest a browser draws everywhere: a bigger image
 *  falls back to the plain arrow near the window's edge. */
const IMAGE_PX = 32;
const HOTSPOT_PX = 2;

/** The icon at `path` under `assets/icons/`, nested at `x, y` art units
 *  `size` wide, in `colour`. */
function nested(path: string, x: number, y: number, size: number, colour: string, fill?: string) {
  const svg = new DOMParser().parseFromString(iconSvg(path)!, "image/svg+xml").documentElement;
  svg.setAttribute("x", String(x));
  svg.setAttribute("y", String(y));
  svg.setAttribute("width", String(size));
  svg.setAttribute("height", String(size));
  svg.setAttribute("style", `color: rgb(${colour})`);
  if (fill) svg.setAttribute("fill", fill);
  return new XMLSerializer().serializeToString(svg);
}

/** The cursor for `action` as an SVG image `scale` device pixels per CSS
 *  pixel: the arrow tip at (HOTSPOT_PX, HOTSPOT_PX), its badge below right. */
export function cursorImage(action: CursorAction, scale = 1): string {
  const icon = action === "default" ? null : ACTION_ICONS[action];
  const badge =
    action === "blocked"
      ? gameHud.bad
      : action === "attack" || action === "attack_move" || action === "attack_ground"
        ? gameHud.enemy
        : gameHud.accent;
  const side = IMAGE_PX * scale;
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${side}" height="${side}" viewBox="0 0 ${IMAGE_PX} ${IMAGE_PX}">` +
    `<filter id="halo" x="-50%" y="-50%" width="200%" height="200%">` +
    // The halo's lengths are in art units, scaled with the arrow.
    `<feDropShadow dx="0" dy="0" stdDeviation="1" flood-color="#000"/>` +
    `<feDropShadow dx="0" dy="0" stdDeviation="2" flood-color="#000" flood-opacity="0.9"/>` +
    `</filter>` +
    `<g filter="url(#halo)" transform="translate(${HOTSPOT_PX} ${HOTSPOT_PX}) scale(${CURSOR_SCALE})">` +
    // The generated arrow's tip is (2, 2); every action uses this same hotspot.
    nested(
      hudIcon("cursor_arrow"),
      -2,
      -2,
      32,
      channels(gameHud.text),
      `rgb(${channels(gameHud.glass)} / 0.9)`,
    ) +
    (icon ? nested(icon, 22, 19, 19, channels(badge)) : "") +
    `</g></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

const styles = new Map<CursorAction, string>();
/** The CSS `cursor` for `action`: drawn by the system, so it moves with the
 *  pointer however long the page takes to draw a frame. */
export function cursorStyle(action: CursorAction): string {
  let style = styles.get(action);
  if (!style) {
    style =
      `image-set(url("${cursorImage(action)}") 1x, url("${cursorImage(action, 2)}") 2x) ` +
      `${HOTSPOT_PX} ${HOTSPOT_PX}, default`;
    styles.set(action, style);
  }
  return style;
}

/** The cursor for `action` drawn into the page with its tip at `x, y`: what
 *  the system draws, for galleries and labs, whose screenshots omit the
 *  system's own pointer. */
export function CursorPicture({ action, x, y }: { action: CursorAction; x: number; y: number }) {
  return (
    <img
      className="game-cursor-picture"
      data-action={action}
      src={cursorImage(action, window.devicePixelRatio > 1 ? 2 : 1)}
      width={IMAGE_PX}
      height={IMAGE_PX}
      style={{ left: x - HOTSPOT_PX, top: y - HOTSPOT_PX }}
      alt=""
    />
  );
}

/** Show `action` as the app's pointer. The document's `data-cursor` names it. */
function showCursor(action: CursorAction) {
  const root = document.documentElement;
  if (root.dataset.cursor === action) return;
  root.dataset.cursor = action;
  root.style.setProperty("--game-cursor", cursorStyle(action));
}

const SetCursorAction = createContext<(action: CursorAction) => void>(() => {});

/** The page's action for the app cursor; "default" when it has none to show. */
export function useCursorAction() {
  return useContext(SetCursorAction);
}

/** The one pointer for the whole app: the game's arrow, which the system
 *  draws over every element (see gameCursor.css). Pages set its action. */
export function AppCursor({ children }: { children: ReactNode }) {
  const setAction = useRef(showCursor).current;
  useLayoutEffect(() => {
    showCursor("default");
    return () => {
      delete document.documentElement.dataset.cursor;
      document.documentElement.style.removeProperty("--game-cursor");
    };
  }, []);
  return <SetCursorAction.Provider value={setAction}>{children}</SetCursorAction.Provider>;
}
