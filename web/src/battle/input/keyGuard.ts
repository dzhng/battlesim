/** Which key presses belong to the game at all: the one typing guard that
 *  command keys and held camera keys share. */

/** A key event as the game reads it; a DOM `KeyboardEvent` satisfies it. */
export interface KeyPress {
  /** Physical key (`KeyboardEvent.code`), so bindings keep their place on any layout. */
  code: string;
  target: EventTarget | null;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  repeat: boolean;
}

/** Whether keys pressed at `target` are text going into a control. */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!target || typeof (target as Element).tagName !== "string") return false;
  const el = target as HTMLElement;
  const tag = el.tagName.toUpperCase();
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || el.isContentEditable === true;
}

/** A key the game may hold: not typed into a control, not part of a browser
 *  or OS shortcut (Ctrl, Cmd, Alt). Shift passes: it queues. */
export function isGameHold(e: Omit<KeyPress, "repeat">): boolean {
  return !isTypingTarget(e.target) && !e.ctrlKey && !e.metaKey && !e.altKey;
}

/** A press the game may act on once: a game hold that is not auto-repeat. */
export function isGameKey(e: KeyPress): boolean {
  return isGameHold(e) && !e.repeat;
}
