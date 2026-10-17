/** A rejected order says why, briefly: the newest refusal in the HUD's
 *  type with the one "can't" colour, no box, gone after a moment. An order
 *  refused whole says it beside the cursor; a unit refused on its own (a
 *  squad left without a seat, a carrier with no room to unload, at the
 *  order or on arrival) says it beside that unit's card, or its carrier's.
 *  An accepted order says nothing; its marks on the ground are the
 *  confirmation. */
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { hudIcon } from "@packages/scene-assets/src/icons";
import type { AckEntry } from "../input/useUnitControl";
import type { RefusalView } from "../sim/observationView";
import { Icon } from "./icons";
import { GAME_CURSOR_SIZE } from "./gameCursor";
import type { ReadoutRect } from "./readoutLayout";

/** One per-unit refusal as published (`ObservationView.refusals`): its
 *  tick, and a key of its own. */
export interface RefusalEvent extends RefusalView {
  key: number;
  tick: number;
}

/** How long a refusal stays. */
const SHOWN_MS = 2200;
/** Beyond the complete arrow+badge, with space for its dark edge. */
const FROM_CURSOR_PX = [GAME_CURSOR_SIZE.width + 7, 18] as const;
/** Clear of a unit's card, beside its top. */
const FROM_CARD_PX = 6;

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
  no_room_to_unload: "NO ROOM TO UNLOAD",
  not_a_carrier: "NOT A CARRIER",
  aboard: "ABOARD",
};

/** Refusals the player is never told about: an order past the rate limit
 *  is a stuck input or a script's flood, not a choice to explain. */
const SILENT = new Set(["rate_limited"]);

const refusalText = (reason: string) =>
  REFUSAL_TEXT[reason] ?? reason.replaceAll("_", " ").toUpperCase();

type Shown = { text: string } & (
  | { at: readonly [number, number]; unit?: never }
  | { unit: number; at?: never }
);

export function RejectedOrder({
  acks,
  refusals = [],
  cardBox,
}: {
  acks: readonly AckEntry[];
  /** The side's per-unit refusals, newest first. */
  refusals?: readonly RefusalEvent[];
  /** Where a unit's card (or its carrier's) is drawn: a per-unit refusal
   *  stands beside it. */
  cardBox?: (unit: number) => ReadoutRect | null;
}) {
  const layer = useRef<HTMLDivElement>(null);
  const pointer = useRef<readonly [number, number]>([0, 0]);
  const [shown, setShown] = useState<Shown | null>(null);
  const cardBoxRef = useRef(cardBox);
  cardBoxRef.current = cardBox;
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
      // An accepted order retires the refusal of the one before; a unit's
      // own refusal stays its moment.
      setShown((now) => (now?.at ? null : now));
      return;
    }
    setShown({
      text: error ? refusalText(error.reason) : "SOME MOVES NOT AVAILABLE",
      at: pointer.current,
    });
    const timer = setTimeout(() => setShown(null), SHOWN_MS);
    return () => clearTimeout(timer);
  }, [newest]);
  const refused = refusals[0];
  useEffect(() => {
    if (!refused || SILENT.has(refused.reason)) return;
    setShown({ text: refusalText(refused.reason), unit: refused.unit });
    const timer = setTimeout(() => setShown(null), SHOWN_MS);
    return () => clearTimeout(timer);
  }, [refused]);
  useLayoutEffect(() => {
    if (!shown) return;
    const element = layer.current!;
    /** Beside `card` (right of its top, else left), clamped on screen. */
    const besideCard = (card: ReadoutRect, width: number, height: number) => {
      let x = card.x1 + FROM_CARD_PX;
      if (x + width > window.innerWidth - 8) x = card.x0 - width - FROM_CARD_PX;
      x = Math.max(8, Math.min(x, window.innerWidth - width - 8));
      const y = Math.max(8, Math.min(card.y0, window.innerHeight - height - 8));
      return [x, y] as const;
    };
    const besideCursor = ([px, py]: readonly [number, number], width: number, height: number) => {
      let x = px + FROM_CURSOR_PX[0];
      if (x + width > window.innerWidth - 8) x = px - width - 12;
      x = Math.max(8, Math.min(x, window.innerWidth - width - 8));
      let y = py + FROM_CURSOR_PX[1];
      // A clamped message may span the cursor: place it below the composite.
      if (x + width > px - 2 && x < px + GAME_CURSOR_SIZE.width)
        y = py + GAME_CURSOR_SIZE.height + 8;
      if (y + height > window.innerHeight - 8) y = py - height - 12;
      y = Math.max(8, Math.min(y, window.innerHeight - height - 8));
      return [x, y] as const;
    };
    const place = () => {
      const { width, height } = element.getBoundingClientRect();
      const card = shown.unit === undefined ? null : (cardBoxRef.current?.(shown.unit) ?? null);
      // A unit's refusal follows its card, and hides while the card does.
      element.style.visibility = shown.unit !== undefined && !card ? "hidden" : "";
      const [x, y] = card
        ? besideCard(card, width, height)
        : besideCursor(shown.at ?? pointer.current, width, height);
      element.style.transform = `translate(${x}px, ${y}px)`;
    };
    place();
    window.addEventListener("resize", place);
    let frame =
      shown.unit === undefined
        ? null
        : requestAnimationFrame(function follow() {
            place();
            frame = requestAnimationFrame(follow);
          });
    return () => {
      window.removeEventListener("resize", place);
      if (frame !== null) cancelAnimationFrame(frame);
    };
  }, [shown]);
  if (!shown) return null;
  const at = shown.at ?? [0, 0];
  const [x, y] = [at[0] + FROM_CURSOR_PX[0], at[1] + FROM_CURSOR_PX[1]];
  return (
    <div
      ref={layer}
      className="hud-rejected"
      role="status"
      data-testid="rejected-order"
      data-unit={shown.unit}
      style={{ transform: `translate(${x}px, ${y}px)` }}
    >
      <Icon path={hudIcon("rejected")} />
      {shown.text}
    </div>
  );
}
