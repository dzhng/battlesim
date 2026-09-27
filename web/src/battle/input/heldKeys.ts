/** Which keys are held down right now, for continuous input like camera
 *  panning. Presses pass the shared typing guard; blur releases everything,
 *  so a key let go while the page lacked focus never sticks. */
import { useEffect, useState } from "react";
import { isGameHold } from "./keyGuard";

export interface HeldKeys {
  /** Held keys by `KeyboardEvent.code`. */
  readonly held: ReadonlySet<string>;
  detach(): void;
}

/** Track the keys `claim` accepts on `win`; their browser default (arrow
 *  scrolling, Space's page scroll) is suppressed. `changed` hears each
 *  press and release that changes the held set. */
export function trackHeldKeys(
  win: Window,
  claim: (code: string) => boolean,
  changed?: (held: ReadonlySet<string>) => void,
): HeldKeys {
  const held = new Set<string>();
  const down = (e: KeyboardEvent) => {
    if (!claim(e.code) || !isGameHold(e)) return;
    e.preventDefault();
    if (held.has(e.code)) return;
    held.add(e.code);
    changed?.(held);
  };
  const up = (e: KeyboardEvent) => {
    if (!held.delete(e.code)) return;
    // Space on a focused button would click it on release.
    e.preventDefault();
    changed?.(held);
  };
  const release = () => {
    if (!held.size) return;
    held.clear();
    changed?.(held);
  };
  win.addEventListener("keydown", down);
  win.addEventListener("keyup", up);
  win.addEventListener("blur", release);
  return {
    held,
    detach() {
      win.removeEventListener("keydown", down);
      win.removeEventListener("keyup", up);
      win.removeEventListener("blur", release);
      held.clear();
    },
  };
}

/** Whether the key `code` is held on `win` right now, as React state. */
export function useHeldKey(code: string, win: Window | null = globalThis.window ?? null): boolean {
  const [held, setHeld] = useState(false);
  useEffect(() => {
    if (!win) return;
    const keys = trackHeldKeys(
      win,
      (c) => c === code,
      (h) => setHeld(h.has(code)),
    );
    return () => {
      keys.detach();
      setHeld(false);
    };
  }, [code, win]);
  return held;
}
