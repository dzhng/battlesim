/** One side's heard sounds, as captions: every decoded observation's hearing
 *  cues are captioned with exactly what the cue carries (kind, band and
 *  direction, never a position); `battle-audio` plays the same cues. The
 *  same sound (kind, direction and listener) keeps one row with a count;
 *  rows expire a few seconds after the sound was last heard, and only the
 *  newest few show. The battle shows them as subtitles, when the player
 *  turns them on (`soundSettings.subtitles`). */
import { useCallback, useState } from "react";
import village from "@fixtures/village.json";
import type { ObservationView, SoundCueView } from "../sim/observation";
import { unitName } from "./readouts";

/** Rows shown at most. */
export const CAPTION_ROWS = 3;
/** A row lasts this long after its sound was last heard (ticks). */
export const CAPTION_TICKS = 5 * village.tick_hz;

const DIRECTIONS = [
  "east",
  "north-east",
  "north",
  "north-west",
  "west",
  "south-west",
  "south",
  "south-east",
];

export function describeCue(cue: SoundCueView, listenerName: string): string {
  const what =
    cue.category === "shot"
      ? "gunfire"
      : cue.category === "vehicle"
        ? cue.moving
          ? "engine, moving"
          : "engine, idling"
        : cue.moving
          ? "footsteps"
          : "voices";
  return `Heard ${what}, ${cue.band}, ${DIRECTIONS[cue.sector]} of ${listenerName}`;
}

export interface CaptionLine {
  tick: number;
  text: string;
  /** The sound this row stands for: kind, direction and the listener as
   *  named (two tanks hearing the same shot are one row, as they read). */
  key: string;
  /** How many times it was heard while the row lasted. */
  count: number;
}

/** One heard cue as a caption row. */
export function cueLine(cue: SoundCueView, tick: number, listenerName: string): CaptionLine {
  return {
    key: `${cue.category}/${cue.moving}/${cue.sector}/${listenerName}`,
    tick,
    text: describeCue(cue, listenerName),
    count: 1,
  };
}

/** Fold newly heard lines into the rows at `tick`: expired rows go, a sound
 *  already listed moves to the top with its count raised and its latest
 *  wording, newest first, at most CAPTION_ROWS. */
export function foldCaptions(
  rows: readonly CaptionLine[],
  heard: readonly CaptionLine[],
  tick: number,
): CaptionLine[] {
  let next = rows.filter((r) => tick - r.tick < CAPTION_TICKS);
  for (const line of heard) {
    const old = next.find((r) => r.key === line.key);
    next = [
      { ...line, count: (old?.count ?? 0) + line.count },
      ...next.filter((r) => r.key !== line.key),
    ];
  }
  return next.slice(0, CAPTION_ROWS);
}

export function useCaptions() {
  const [captions, setCaptions] = useState<CaptionLine[]>([]);

  /** Caption one decoded frame's cues; expire old rows. */
  const note = useCallback((o: ObservationView) => {
    const lines = o.audible.map((cue) => {
      const listener = o.own.find((u) => u.id === cue.listener);
      return cueLine(cue, o.tick, listener ? unitName(listener) : "a unit");
    });
    setCaptions((current) => {
      const expiring = current.some((r) => o.tick - r.tick >= CAPTION_TICKS);
      return lines.length || expiring ? foldCaptions(current, lines, o.tick) : current;
    });
  }, []);

  /** A fresh battle starts with nothing heard. */
  const clear = useCallback(() => {
    setCaptions([]);
  }, []);

  return { note, clear, captions };
}

type Captions = ReturnType<typeof useCaptions>;

/** What was heard, newest first: nothing at all while nothing is. The
 *  battle's subtitles by default; a lab lists them in its own style. */
export function CaptionList({
  captions,
  className = "hud-subtitles",
}: {
  captions: Captions;
  className?: string;
}) {
  return (
    <ul className={className} data-testid="captions">
      {captions.captions.map((c) => (
        <li key={c.key} data-count={c.count}>
          {c.text}
          {c.count > 1 ? ` ×${c.count}` : ""}
        </li>
      ))}
    </ul>
  );
}
