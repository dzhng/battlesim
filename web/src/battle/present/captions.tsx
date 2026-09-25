/** One side's heard sounds, as sound and as captions carrying the same
 *  information: every decoded observation's cues are played (once the player
 *  turns sound on) and captioned, repeats collapsing into one line. */
import { useCallback, useEffect, useRef, useState } from "react";
import type { ObservationView } from "../sim/observation";
import { CueAudio, describeCue, type Caption } from "./audio";

const CAPTION_LINES = 6;

export interface CaptionLine extends Caption {
  /** How many times this sound repeated in a row. */
  count?: number;
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

  /** Play and caption one decoded frame's cues. */
  const note = useCallback((o: ObservationView) => {
    if (!o.audible.length) return;
    const lines = o.audible.map((cue) => {
      audio.current!.play(cue, yaw.current());
      const listener = o.own.find((u) => u.id === cue.listener);
      return {
        tick: o.tick,
        text: describeCue(cue, listener ? `${listener.kind} #${listener.id}` : "a unit"),
      };
    });
    transcript.current.push(...lines);
    setCaptions((current) => {
      const next = [...current];
      for (const line of lines) {
        if (next[0]?.text === line.text)
          next[0] = { ...next[0], tick: line.tick, count: (next[0].count ?? 1) + 1 };
        else next.unshift(line);
      }
      return next.slice(0, CAPTION_LINES);
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
        {cues.captions.map((c, k) => (
          <li key={`${c.tick}-${k}`}>
            {c.text}
            {c.count ? ` ×${c.count}` : ""}
          </li>
        ))}
      </ul>
    </>
  );
}
