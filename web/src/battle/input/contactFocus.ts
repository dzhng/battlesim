// Jumps through the side's contacts, newest first: N older, Shift+N newer.
// A contact that left the live list starts the next jump at the newest.
import { ContactFocusBinding } from "./commandBindings";
import { isGameKey } from "./keyGuard";
import { emitUiSignal } from "./uiSignals";

/** One contact the camera can move to. */
export interface FocusTarget {
  /** The contact's handle. */
  id: number;
  /** Its ground centre. */
  target: readonly [number, number];
  /** Evidence generation, so reused handles do not inherit focus. */
  evidenceTick: number;
}

/** The contact fields this module reads. */
export interface FocusContact {
  id: number;
  center: readonly number[];
  evidenceTick: number;
  retiring: boolean;
}

/** The contacts N steps through, in its order: live ones, newest evidence first. */
export function liveContactsNewestFirst<
  C extends Pick<FocusContact, "id" | "evidenceTick" | "retiring">,
>(contacts: readonly C[]): C[] {
  return contacts
    .filter((c) => !c.retiring)
    .sort((a, b) => b.evidenceTick - a.evidenceTick || b.id - a.id);
}

/** Live contacts as camera targets, newest evidence first. */
export function contactFocusTargets(contacts: readonly FocusContact[]): FocusTarget[] {
  return liveContactsNewestFirst(contacts).map((c) => ({
    id: c.id,
    target: [c.center[0], c.center[1]] as const,
    evidenceTick: c.evidenceTick,
  }));
}

/** The target a jump lands on: -1 when empty, otherwise the newest when the
 *  last contact is gone, or one step older or newer, wrapping. */
export function focusStep(
  targets: readonly FocusTarget[],
  last: Pick<FocusTarget, "id" | "evidenceTick"> | null,
  newer: boolean,
): number {
  if (targets.length === 0) return -1;
  const at =
    last === null
      ? -1
      : targets.findIndex((t) => t.id === last.id && t.evidenceTick === last.evidenceTick);
  if (at < 0) return 0;
  return newer ? (at - 1 + targets.length) % targets.length : (at + 1) % targets.length;
}

/** Step the view through the side's contacts on the binding's key (Shift:
 *  newer) until `signal` aborts: each jump centres `centerOn` on the target,
 *  unless `blocked`, and tells the coach. */
export function listenContactFocus(
  signal: AbortSignal,
  targets: () => readonly FocusTarget[],
  blocked: () => boolean,
  centerOn: (target: readonly [number, number]) => void,
): void {
  /** The contact last visited; null before any. */
  let focused: Pick<FocusTarget, "id" | "evidenceTick"> | null = null;
  const jump = (newer: boolean) => {
    if (blocked()) return;
    const list = targets();
    const target = list[focusStep(list, focused, newer)];
    if (!target) return;
    focused = target;
    centerOn(target.target);
    emitUiSignal("contact_focus");
  };
  window.addEventListener(
    "keydown",
    (e) => {
      if (e.code === ContactFocusBinding.code && isGameKey(e)) jump(e.shiftKey);
    },
    { signal },
  );
}
