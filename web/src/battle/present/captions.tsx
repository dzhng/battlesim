/** One side's heard sounds, as sound and as captions carrying the same
 *  information: every decoded observation's cues are played (once the player
 *  turns sound on) and captioned. The same sound (kind, direction and
 *  listener) keeps one row with a count; rows expire a few seconds after the
 *  sound was last heard, and only the newest few show. */
import { useCallback, useEffect, useRef, useState } from "react";
import village from "@fixtures/village.json";
import type { ObservationView, SoundCueView } from "../sim/observation";
import { CueAudio, describeCue, type Caption } from "./audio";

/** Rows shown at most. */
export const CAPTION_ROWS = 3;
/** A row lasts this long after its sound was last heard (ticks). */
export const CAPTION_TICKS = 5 * village.tick_hz;

export interface CaptionLine extends Caption {
  /** The sound this row stands for: kind, direction and listener. */
  key: string;
  /** How many times it was heard while the row lasted. */
  count: number;
}

/** One heard cue as a caption row. */
export function cueLine(cue: SoundCueView, tick: number, listenerName: string): CaptionLine {
  return {
    key: `${cue.category}/${cue.moving}/${cue.sector}/${cue.listener}`,
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

/** `cameraYaw` pans each sound to where it was heard from, as seen now. */
export function useSoundCues(cameraYaw: () => number) {
  const audio = useRef<CueAudio | null>(null);
  audio.current ??= new CueAudio();
  const [captions, setCaptions] = useState<CaptionLine[]>([]);
  const [soundOn, setSoundOn] = useState(false);
  const transcript = useRef<Caption[]>([]);
  const yaw = useRef(cameraYaw);
  yaw.current = cameraYaw;
  useEffect(() => () => audio.current?.dispose(), []);

  /** Play and caption one decoded frame's cues; expire old rows. */
  const note = useCallback((o: ObservationView) => {
    const lines = o.audible.map((cue) => {
      audio.current!.play(cue, yaw.current());
      const listener = o.own.find((u) => u.id === cue.listener);
      return cueLine(cue, o.tick, listener ? `${listener.kind} #${listener.id}` : "a unit");
    });
    transcript.current.push(...lines.map(({ tick, text }) => ({ tick, text })));
    setCaptions((current) => {
      const expiring = current.some((r) => o.tick - r.tick >= CAPTION_TICKS);
      return lines.length || expiring ? foldCaptions(current, lines, o.tick) : current;
    });
  }, []);

  /** A fresh battle starts with nothing heard. */
  const clear = useCallback(() => {
    transcript.current = [];
    setCaptions([]);
  }, []);

  /** Audio starts from a user gesture. */
  const setSound = useCallback((on: boolean) => {
    if (on) audio.current!.enable();
    setSoundOn(on);
  }, []);

  return {
    note,
    clear,
    captions,
    soundOn,
    setSound,
    transcript,
    scheduled: () => audio.current!.scheduled,
  };
}

export type SoundCues = ReturnType<typeof useSoundCues>;

/** The switch that turns sound on (captions are always shown). */
export function SoundSwitch({ cues }: { cues: SoundCues }) {
  return (
    <label>
      <input
        type="checkbox"
        checked={cues.soundOn}
        onChange={(e) => cues.setSound(e.target.checked)}
      />
      Play sounds (captions always shown)
    </label>
  );
}

/** What was heard, newest first. */
export function Captions({ cues }: { cues: SoundCues }) {
  return (
    <>
      <div className="lab-hint">Heard (newest first)</div>
      <ul className="lab-log" data-testid="captions">
        {cues.captions.length === 0 && <li>Nothing heard</li>}
        {cues.captions.map((c) => (
          <li key={c.key} data-count={c.count}>
            {c.text}
            {c.count > 1 ? ` ×${c.count}` : ""}
          </li>
        ))}
      </ul>
    </>
  );
}
