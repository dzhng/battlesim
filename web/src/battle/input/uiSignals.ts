/** What the player did with the page's own controls, which no battle
 *  observes: the camera moved, a selection made, the shop opened. The input
 *  owners emit into this one bus where the act happens; the tutorial's coach
 *  listens to know a lesson was done. The simulation never sees it. */

export type UiSignal =
  | "camera_pan"
  | "camera_rotate"
  | "camera_tilt"
  | "camera_zoom"
  /** A selection of the player's own units, by a click, a box or a card. */
  | "selected"
  /** A move ordered with a facing (a right-drag). */
  | "moved_with_facing"
  /** The view jumped to a contact. */
  | "contact_focus"
  | "shop_opened"
  /** Every unit's orders shown (the key held). */
  | "show_orders"
  | "group_set";

export const UI_SIGNALS: readonly UiSignal[] = [
  "camera_pan",
  "camera_rotate",
  "camera_tilt",
  "camera_zoom",
  "selected",
  "moved_with_facing",
  "contact_focus",
  "shop_opened",
  "show_orders",
  "group_set",
];

const listeners = new Set<(signal: UiSignal) => void>();

export function emitUiSignal(signal: UiSignal): void {
  for (const listener of listeners) listener(signal);
}

/** The camera moves between two poses, as the coach hears them. */
export function emitCameraMoves(before: CameraMoves, after: CameraMoves): void {
  if (after.target.some((v, i) => v !== before.target[i])) emitUiSignal("camera_pan");
  if (after.yaw !== before.yaw) emitUiSignal("camera_rotate");
  if (after.pitch !== before.pitch) emitUiSignal("camera_tilt");
  if (after.distance !== before.distance) emitUiSignal("camera_zoom");
}

/** The parts of a camera pose the coach tells apart. */
interface CameraMoves {
  target: readonly number[];
  yaw: number;
  pitch: number;
  distance: number;
}

/** Hear every signal until the returned function is called. */
export function onUiSignal(listener: (signal: UiSignal) => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
