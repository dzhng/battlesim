/** Which keys are held down right now, for continuous input like camera
 *  panning. Presses pass the shared typing guard; blur releases everything,
 *  so a key let go while the page lacked focus never sticks. */
import { isGameHold } from "./keyGuard";

export interface HeldKeys {
  /** Held keys by `KeyboardEvent.code`. */
  readonly held: ReadonlySet<string>;
  detach(): void;
}

/** Track the keys `claim` accepts on `win`; their browser default (arrow
 *  scrolling) is suppressed. */
export function trackHeldKeys(win: Window, claim: (code: string) => boolean): HeldKeys {
  const held = new Set<string>();
  const down = (e: KeyboardEvent) => {
    if (!claim(e.code) || !isGameHold(e)) return;
    e.preventDefault();
    held.add(e.code);
  };
  const up = (e: KeyboardEvent) => held.delete(e.code);
  const release = () => held.clear();
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
