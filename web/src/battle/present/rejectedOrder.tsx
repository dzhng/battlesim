/** A rejected order says why, briefly: the newest refusal in the HUD's
 *  type with the one "can't" colour, no box, gone after a moment. An order
 *  refused whole says it beside the cursor; a unit refused on its own (a
 *  squad left without a seat, a carrier with no room to unload, at the
 *  order or on arrival) says it beside that unit's card, or its carrier's,
 *  each refused unit its own.
 *  An accepted order says nothing; its marks on the ground are the
 *  confirmation. */
import { useCallback, useEffect, useLayoutEffect, useRef, useState, type RefObject } from "react";
import { hudIcon } from "@packages/scene-assets/src/icons";
import type { AckEntry } from "../input/useUnitControl";
import type { RefusalView } from "../sim/observationView";
import { Icon } from "./icons";
import { GAME_CURSOR_SIZE } from "./gameCursor";
import type { ReadoutRect } from "./readoutLayout";

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
  capacity_full: "TOO MANY TO FIT",
  one_squad_per_building: "ONE SQUAD PER BUILDING",
  building_occupied: "BUILDING TAKEN",
  no_room_to_unload: "NO ROOM TO UNLOAD",
  not_a_carrier: "NOT A CARRIER",
  aboard: "ABOARD",
  wrong_player: "AN ALLY'S UNIT",
  ai_controlled: "AI IN COMMAND",
};

/** Refusals the player is never told about: an order past the rate limit
 *  is a stuck input or a script's flood, not a choice to explain. */
const SILENT = new Set(["rate_limited"]);

const refusalText = (reason: string) =>
  REFUSAL_TEXT[reason] ?? reason.replaceAll("_", " ").toUpperCase();

type Point = readonly [number, number];
type CardBox = (unit: number) => ReadoutRect | null;

/** Beside `card` (right of its top, else left), clamped on screen. */
function besideCard(card: ReadoutRect, width: number, height: number): Point {
  let x = card.x1 + FROM_CARD_PX;
  if (x + width > window.innerWidth - 8) x = card.x0 - width - FROM_CARD_PX;
  x = Math.max(8, Math.min(x, window.innerWidth - width - 8));
  const y = Math.max(8, Math.min(card.y0, window.innerHeight - height - 8));
  return [x, y];
}

/** Beside the cursor at `px, py`, clear of its arrow and badge, clamped on screen. */
function besideCursor([px, py]: Point, width: number, height: number): Point {
  let x = px + FROM_CURSOR_PX[0];
  if (x + width > window.innerWidth - 8) x = px - width - 12;
  x = Math.max(8, Math.min(x, window.innerWidth - width - 8));
  let y = py + FROM_CURSOR_PX[1];
  // A clamped message may span the cursor: place it below the composite.
  if (x + width > px - 2 && x < px + GAME_CURSOR_SIZE.width) y = py + GAME_CURSOR_SIZE.height + 8;
  if (y + height > window.innerHeight - 8) y = py - height - 12;
  y = Math.max(8, Math.min(y, window.innerHeight - height - 8));
  return [x, y];
}

/** A unit's refusal, until its moment passes: a repeat is a new object. */
interface UnitShown {
  unit: number;
  text: string;
}

export function RejectedOrder({
  acks,
  refusals = [],
  cardBox,
}: {
  acks: readonly AckEntry[];
  /** The side's per-unit refusals, newest first: each a new object, so a
   *  repeat of the same refusal shows again. */
  refusals?: readonly RefusalView[];
  /** Where a unit's card (or its carrier's) is drawn: a per-unit refusal
   *  stands beside it. */
  cardBox?: CardBox;
}) {
  const pointer = useRef<Point>([0, 0]);
  const [atCursor, setAtCursor] = useState<{ text: string; at: Point } | null>(null);
  const [atUnits, setAtUnits] = useState<readonly UnitShown[]>([]);
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
      setAtCursor(null);
      return;
    }
    setAtCursor({
      text: error ? refusalText(error.reason) : "SOME MOVES NOT AVAILABLE",
      at: pointer.current,
    });
    const timer = setTimeout(() => setAtCursor(null), SHOWN_MS);
    return () => clearTimeout(timer);
  }, [newest]);
  // Every refusal newer than the last seen says it, one callout per unit.
  const seen = useRef<RefusalView | undefined>(undefined);
  useEffect(() => {
    const until = seen.current ? refusals.indexOf(seen.current) : -1;
    const fresh = (until < 0 ? refusals : refusals.slice(0, until)).filter(
      (refusal) => !SILENT.has(refusal.reason),
    );
    seen.current = refusals[0];
    if (!fresh.length) return;
    const shown = new Map<number, UnitShown>();
    // Newest first: a unit refused twice at once says its newest.
    for (const { unit, reason } of [...fresh].reverse())
      shown.set(unit, { unit, text: refusalText(reason) });
    setAtUnits((now) => [...now.filter((s) => !shown.has(s.unit)), ...shown.values()]);
  }, [refusals]);
  const passed = useCallback(
    (gone: UnitShown) => setAtUnits((now) => now.filter((s) => s !== gone)),
    [],
  );
  return (
    <>
      {atCursor && <CursorCallout text={atCursor.text} at={atCursor.at} />}
      {atUnits.map((shown) => (
        <UnitCallout key={shown.unit} shown={shown} passed={passed} cardBox={cardBoxRef} />
      ))}
    </>
  );
}

/** A whole order's refusal, beside where the cursor was when it was refused. */
function CursorCallout({ text, at }: { text: string; at: Point }) {
  const layer = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const element = layer.current!;
    const place = () => {
      const { width, height } = element.getBoundingClientRect();
      const [x, y] = besideCursor(at, width, height);
      element.style.transform = `translate(${x}px, ${y}px)`;
    };
    place();
    window.addEventListener("resize", place);
    return () => window.removeEventListener("resize", place);
  }, [text, at]);
  const [x, y] = [at[0] + FROM_CURSOR_PX[0], at[1] + FROM_CURSOR_PX[1]];
  return <Callout layer={layer} text={text} transform={`translate(${x}px, ${y}px)`} />;
}

/** A unit's own refusal: it follows the unit's card, hides while the card
 *  does, and passes after its moment. */
function UnitCallout({
  shown,
  passed,
  cardBox,
}: {
  shown: UnitShown;
  passed: (shown: UnitShown) => void;
  cardBox: { current: CardBox | undefined };
}) {
  const { unit, text } = shown;
  const layer = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const timer = setTimeout(() => passed(shown), SHOWN_MS);
    return () => clearTimeout(timer);
  }, [shown, passed]);
  useLayoutEffect(() => {
    const element = layer.current!;
    const place = () => {
      const { width, height } = element.getBoundingClientRect();
      const card = cardBox.current?.(unit) ?? null;
      element.style.visibility = card ? "" : "hidden";
      if (!card) return;
      const [x, y] = besideCard(card, width, height);
      element.style.transform = `translate(${x}px, ${y}px)`;
    };
    place();
    window.addEventListener("resize", place);
    let frame = requestAnimationFrame(function follow() {
      place();
      frame = requestAnimationFrame(follow);
    });
    return () => {
      window.removeEventListener("resize", place);
      cancelAnimationFrame(frame);
    };
  }, [unit, cardBox]);
  return <Callout layer={layer} text={text} unit={unit} />;
}

function Callout({
  layer,
  text,
  unit,
  transform,
}: {
  layer: RefObject<HTMLDivElement | null>;
  text: string;
  unit?: number;
  transform?: string;
}) {
  return (
    <div
      ref={layer}
      className="hud-rejected"
      role="status"
      data-testid="rejected-order"
      data-unit={unit}
      style={transform ? { transform } : undefined}
    >
      <Icon path={hudIcon("rejected")} />
      {text}
    </div>
  );
}
