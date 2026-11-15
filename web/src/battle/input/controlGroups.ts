/** Control groups (the user's key decisions): Ctrl+N stores the selection as
 *  group N, Shift+N adds the selection to it, N selects it, and N twice
 *  centres the camera on it. A unit is in one group at most, so storing it in
 *  one takes it out of any other. Ten groups, on the number row. Groups are
 *  the client's alone: the simulation never hears of them. */
import type { OwnUnitView } from "../sim/observationView";
import { isTypingTarget, type KeyPress } from "./keyGuard";

/** The groups in the order the army deck shows them: 1 to 9, then 0. */
export const GROUP_KEYS = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"] as const;
export type GroupKey = (typeof GROUP_KEYS)[number];
/** Each group's units, by key. */
export type ControlGroups = ReadonlyMap<GroupKey, readonly number[]>;

export type GroupAction = "assign" | "add" | "recall";

/** What a key press asks of the groups, or null. Ctrl is this key's own
 *  modifier (Ctrl+1 is otherwise the browser's tab switch, which the battle
 *  takes), so it bypasses `isGameKey`; Cmd, Alt and auto-repeat stay the
 *  system's, and typing stays text. */
export function groupKeyPress(
  e: KeyPress & { shiftKey: boolean },
): { key: GroupKey; action: GroupAction } | null {
  const digit = /^Digit([0-9])$/.exec(e.code)?.[1] as GroupKey | undefined;
  if (!digit || e.repeat || e.metaKey || e.altKey || isTypingTarget(e.target)) return null;
  return { key: digit, action: e.ctrlKey ? "assign" : e.shiftKey ? "add" : "recall" };
}

/** `groups` with `units` stored in group `key`: replacing it (assign) or
 *  joining it (add), and out of every other group. Nothing to store leaves
 *  the groups as they were. */
export function storeGroup(
  groups: ControlGroups,
  key: GroupKey,
  units: readonly number[],
  add: boolean,
): ControlGroups {
  if (!units.length) return groups;
  const next = new Map<GroupKey, readonly number[]>();
  for (const [k, members] of groups) {
    const kept = k === key && add ? members : members.filter((id) => !units.includes(id));
    if (kept.length && (k !== key || add)) next.set(k, kept);
  }
  const base = next.get(key) ?? [];
  next.set(key, [...base, ...units.filter((id) => !base.includes(id))]);
  return next;
}

/** The groups as the player sees them: only units still on the side, a group
 *  with none left gone. Unit ids are never reused in a battle, so the dead
 *  need no pruning: they simply stop being present. */
export function liveGroups(groups: ControlGroups, present: ReadonlySet<number>): ControlGroups {
  const live = new Map<GroupKey, readonly number[]>();
  for (const key of GROUP_KEYS) {
    const members = groups.get(key)?.filter((id) => present.has(id));
    if (members?.length) live.set(key, members);
  }
  return live;
}

/** The unit nearest the units' centre: where the camera goes, so a group
 *  spread across the map never lands it on empty ground. */
export function nearestCentre(units: readonly OwnUnitView[]): OwnUnitView | null {
  if (!units.length) return null;
  const cx = units.reduce((s, u) => s + u.position[0], 0) / units.length;
  const cy = units.reduce((s, u) => s + u.position[1], 0) / units.length;
  const away = (u: OwnUnitView) => Math.hypot(u.position[0] - cx, u.position[1] - cy);
  return units.reduce((best, u) => (away(u) < away(best) ? u : best));
}
