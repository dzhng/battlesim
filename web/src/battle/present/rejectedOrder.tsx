/** A rejected order says why, briefly, beside the cursor: the newest
 *  acknowledgement's refusal in the HUD's type with the one "can't" colour,
 *  no box, gone after a moment. An accepted order says nothing; its marks on
 *  the ground are the confirmation. */
import { useEffect, useRef, useState } from "react";
import { hudIcon } from "@packages/scene-assets/src/icons";
import type { AckEntry } from "../input/useUnitControl";
import { Icon } from "./icons";

/** How long a refusal stays. */
const SHOWN_MS = 2200;
/** Its offset from the cursor, down and to the right, in CSS pixels. */
const FROM_CURSOR_PX = [18, 14] as const;

/** Each refusal the player can meet, in player words; any other reads as its
 *  own name. */
const REFUSAL_TEXT: Record<string, string> = {
  out_of_bounds: "OUTSIDE THE MAP",
  unknown_target: "TARGET LOST",
  destroyed: "UNIT LOST",
  not_infantry: "INFANTRY ONLY",
  not_a_building: "NOT A BUILDING",
  capacity_full: "SQUAD TOO BIG FOR IT",
  one_squad_per_building: "ONE SQUAD PER BUILDING",
  building_occupied: "BUILDING TAKEN",
};

const refusalText = (reason: string) =>
  REFUSAL_TEXT[reason] ?? reason.replaceAll("_", " ").toUpperCase();

export function RejectedOrder({ acks }: { acks: readonly AckEntry[] }) {
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
    if (!error) {
      setShown(null);
      return;
    }
    setShown({ text: refusalText(error.reason), at: pointer.current });
    const timer = setTimeout(() => setShown(null), SHOWN_MS);
    return () => clearTimeout(timer);
  }, [newest]);
  if (!shown) return null;
  const [x, y] = [shown.at[0] + FROM_CURSOR_PX[0], shown.at[1] + FROM_CURSOR_PX[1]];
  return (
    <div
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
