import { useEffect, useImperativeHandle, useRef, type Ref } from "react";
import "./frameRate.css";

export interface FrameRateHandle {
  /** One viewport frame opportunity; drawn reports an actual scene render. */
  frame(now: number, drawn: boolean): void;
}

/** Draw cadence, independent of simulation ticks and React updates. */
export function FrameRate({ handle }: { handle: Ref<FrameRateHandle> }) {
  const label = useRef<HTMLSpanElement>(null);
  const sample = useRef({ since: null as number | null, frames: 0 });
  useEffect(() => {
    const reset = () => {
      sample.current = { since: null, frames: 0 };
      if (label.current) label.current.textContent = "— FPS";
    };
    document.addEventListener("visibilitychange", reset);
    return () => document.removeEventListener("visibilitychange", reset);
  }, []);
  useImperativeHandle(handle, () => ({
    frame(now, drawn) {
      if (document.hidden) return;
      const current = sample.current;
      if (current.since === null) {
        current.since = now;
        return;
      }
      if (drawn) current.frames++;
      const elapsed = now - current.since;
      if (elapsed < 1000) return;
      if (label.current)
        label.current.textContent = `${Math.round((current.frames * 1000) / elapsed)} FPS`;
      current.since = now;
      current.frames = 0;
    },
  }));
  return (
    <span ref={label} className="frame-rate" aria-hidden="true">
      — FPS
    </span>
  );
}
