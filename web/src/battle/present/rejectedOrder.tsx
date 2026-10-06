/** A rejected order says why, briefly, beside the cursor: the newest
 *  acknowledgement's refusal in the HUD's type with the one "can't" colour,
 *  no box, gone after a moment. An accepted order says nothing; its marks on
 *  the ground are the confirmation. */
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { hudIcon } from "@packages/scene-assets/src/icons";
import type { AckEntry } from "../input/useUnitControl";
import { Icon } from "./icons";
import { GAME_CURSOR_SIZE } from "./gameCursor";

/** How long a refusal stays. */
const SHOWN_MS = 2200;
/** Beyond the complete arrow+badge, with space for its dark edge. */
const FROM_CURSOR_PX = [GAME_CURSOR_SIZE.width + 7, 18] as const;

/** Each refusal the player can meet, in player words; any other reads as its
 *  own name. */
const REFUSAL_TEXT: Record<string, string> = {
  out_of_bounds: "OUTSIDE THE MAP",
  no_valid_destination: "MOVE NOT AVAILABLE",
  unknown_target: "TARGET LOST",
  destroyed: "UNIT LOST",
  not_infantry: "INFANTRY ONLY",
  not_a_building: "NOT A BUILDING",
  capacity_full: "SQUAD TOO BIG FOR IT",
  one_squad_per_building: "ONE SQUAD PER BUILDING",
  building_occupied: "BUILDING TAKEN",
};

/** Refusals the player is never told about: an order past the rate limit
 *  is a stuck input or a script's flood, not a choice to explain. */
const SILENT = new Set(["rate_limited"]);

const refusalText = (reason: string) =>
  REFUSAL_TEXT[reason] ?? reason.replaceAll("_", " ").toUpperCase();

export function RejectedOrder({ acks }: { acks: readonly AckEntry[] }) {
  const layer = useRef<HTMLDivElement>(null);
  const pointer = useRef<readonly [number, number]>([0, 0]);
  const [shown, setShown] = useState<{ text: string; at: readonly [number, number] } | null>(null);
  useEffect(() => {
    const move = (e: PointerEvent) => (pointer.current = [e.clientX, e.clientY]);
    window.addEventListener("pointermove", move);
    return () => window.removeEventListener("pointermove", move);
  }, []);
  const newest = acks[0];
  useEffect(() => {
    const error = newest?.ack.error;
    if (error && SILENT.has(error.reason)) return;
    const destinations = newest?.ack.placement?.destinations;
    const partial = destinations?.some((mark) => !mark.placed);
    if (!error && !partial) {
      setShown(null);
      return;
    }
    setShown({
      text: error ? refusalText(error.reason) : "SOME MOVES NOT AVAILABLE",
      at: pointer.current,
    });
    const timer = setTimeout(() => setShown(null), SHOWN_MS);
    return () => clearTimeout(timer);
  }, [newest]);
  useLayoutEffect(() => {
    if (!shown) return;
    const place = () => {
      const element = layer.current!;
      const { width, height } = element.getBoundingClientRect();
      const [px, py] = shown.at;
      let x = px + FROM_CURSOR_PX[0];
      if (x + width > window.innerWidth - 8) x = px - width - 12;
      x = Math.max(8, Math.min(x, window.innerWidth - width - 8));
      let y = py + FROM_CURSOR_PX[1];
      // A clamped message may span the cursor: place it below the composite.
      if (x + width > px - 2 && x < px + GAME_CURSOR_SIZE.width)
        y = py + GAME_CURSOR_SIZE.height + 8;
      if (y + height > window.innerHeight - 8) y = py - height - 12;
      y = Math.max(8, Math.min(y, window.innerHeight - height - 8));
      element.style.transform = `translate(${x}px, ${y}px)`;
    };
    place();
    window.addEventListener("resize", place);
    return () => window.removeEventListener("resize", place);
  }, [shown]);
  if (!shown) return null;
  const [x, y] = [shown.at[0] + FROM_CURSOR_PX[0], shown.at[1] + FROM_CURSOR_PX[1]];
  return (
    <div
      ref={layer}
      className="hud-rejected"
      role="status"
      data-testid="rejected-order"
      style={{ transform: `translate(${x}px, ${y}px)` }}
    >
      <Icon path={hudIcon("rejected")} />
      {shown.text}
    </div>
  );
}
