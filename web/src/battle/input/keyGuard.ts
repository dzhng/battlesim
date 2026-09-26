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

/** A press the game may act on: not typed into a control, not a browser or
 *  OS shortcut (Ctrl, Cmd, Alt), not auto-repeat. Shift passes: it queues. */
export function isGameKey(e: KeyPress): boolean {
  return !isTypingTarget(e.target) && !e.ctrlKey && !e.metaKey && !e.altKey && !e.repeat;
}
