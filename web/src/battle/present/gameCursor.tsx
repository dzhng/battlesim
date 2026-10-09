import {
  createContext,
  useContext,
  useEffect,
  useImperativeHandle,
  useRef,
  type ReactNode,
  type Ref,
} from "react";
import { Icon } from "./icons";
import { hudIcon, stateIcon } from "@packages/scene-assets/src/icons";
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
const CURSOR_SCALE = 0.7;
const CURSOR_ART_SIZE = { width: 41, height: 38 } as const;
/** Complete arrow and badge bounds, also used to keep callouts clear. */
export const GAME_CURSOR_SIZE = {
  width: CURSOR_ART_SIZE.width * CURSOR_SCALE,
  height: CURSOR_ART_SIZE.height * CURSOR_SCALE,
} as const;

export interface GameCursorHandle {
  /** Viewport client coordinates; null hides the complete cursor. */
  place(position: { x: number; y: number } | null, action: CursorAction): void;
}

/** Screen-space feedback only: the caller owns picking and accepted action. */
export function GameCursor({ handle }: { handle: Ref<GameCursorHandle> }) {
  const layer = useRef<HTMLDivElement>(null);
  const badge = useRef<HTMLSpanElement>(null);
  const shown = useRef<CursorAction>("default");
  useImperativeHandle(handle, () => ({
    place(position, action) {
      const root = layer.current;
      if (!root) return;
      root.hidden = position === null;
      if (!position) return;
      root.style.transform = `translate(${position.x}px, ${position.y}px) scale(${CURSOR_SCALE})`;
      if (action !== shown.current) {
        shown.current = action;
        root.dataset.action = action;
        badge.current!.hidden = action === "default";
        for (const icon of badge.current!.children)
          (icon as HTMLElement).hidden = (icon as HTMLElement).dataset.action !== action;
      }
    },
  }));
  return (
    <div
      ref={layer}
      className="game-cursor"
      data-testid="game-cursor"
      data-action="default"
      style={CURSOR_ART_SIZE}
      hidden
      aria-hidden="true"
    >
      <Icon path={hudIcon("cursor_arrow")} className="game-cursor-arrow" />
      <span ref={badge} className="game-cursor-badge" hidden>
        {Object.entries(ACTION_ICONS).map(([action, path]) => (
          <span key={action} data-action={action} hidden>
            <Icon path={path} />
          </span>
        ))}
      </span>
    </div>
  );
}

const SetCursorAction = createContext<(action: CursorAction) => void>(() => {});

/** The page's action for the app cursor; "default" when it has none to show. */
export function useCursorAction() {
  return useContext(SetCursorAction);
}

/** The one visible pointer for the whole app. The native cursor is never
 *  drawn (see gameCursor.css); this follows the pointer over every element and
 *  hides only when the pointer leaves the window. Pages set its action. */
export function AppCursor({ children }: { children: ReactNode }) {
  const cursor = useRef<GameCursorHandle>(null);
  const action = useRef<CursorAction>("default");
  const at = useRef<{ x: number; y: number } | null>(null);
  const setAction = useRef((next: CursorAction) => {
    action.current = next;
    cursor.current?.place(at.current, next);
  }).current;
  useEffect(() => {
    const lifetime = new AbortController();
    const { signal } = lifetime;
    window.addEventListener(
      "pointermove",
      (e) => {
        at.current = { x: e.clientX, y: e.clientY };
        cursor.current?.place(at.current, action.current);
      },
      { capture: true, passive: true, signal },
    );
    window.addEventListener(
      "pointerout",
      (e) => {
        if (e.relatedTarget !== null) return;
        at.current = null;
        cursor.current?.place(null, action.current);
      },
      { capture: true, signal },
    );
    return () => lifetime.abort();
  }, []);
  return (
    <SetCursorAction.Provider value={setAction}>
      {children}
      <GameCursor handle={cursor} />
    </SetCursorAction.Provider>
  );
}
