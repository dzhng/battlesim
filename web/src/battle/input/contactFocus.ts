// Jumps through the side's contacts, newest first: N older, Shift+N newer.
// A contact that left the live list starts the next jump at the newest.

/** One contact the camera can move to. */
export interface FocusTarget {
  /** The contact's handle. */
  id: number;
  /** Its ground centre. */
  target: readonly [number, number];
}

/** The contact fields this module reads. */
export interface FocusContact {
  id: number;
  center: readonly number[];
  evidenceTick: number;
  retiring: boolean;
}

/** Live contacts as camera targets, newest evidence first. */
export function contactFocusTargets(contacts: readonly FocusContact[]): FocusTarget[] {
  return contacts
    .filter((c) => !c.retiring)
    .sort((a, b) => b.evidenceTick - a.evidenceTick || b.id - a.id)
    .map((c) => ({ id: c.id, target: [c.center[0], c.center[1]] as const }));
}

/** The target a jump lands on: -1 when empty, otherwise the newest when the
 *  last contact is gone, or one step older or newer, wrapping. */
export function focusStep(
  targets: readonly FocusTarget[],
  lastId: number | null,
  newer: boolean,
): number {
  if (targets.length === 0) return -1;
  const at = lastId === null ? -1 : targets.findIndex((t) => t.id === lastId);
  if (at < 0) return 0;
  return newer ? (at - 1 + targets.length) % targets.length : (at + 1) % targets.length;
}
