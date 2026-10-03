import { useImperativeHandle, useRef, type Ref } from "react";
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
      root.style.transform = `translate(${position.x}px, ${position.y}px)`;
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
