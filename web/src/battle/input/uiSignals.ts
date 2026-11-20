/** What the player did with the page's own controls, which no battle
 *  observes: the camera moved, a selection made, the shop opened. The input
 *  owners emit into this one bus where the act happens; the tutorial's coach
 *  listens to know a lesson was done. The simulation never sees it. */

export const UI_SIGNALS = [
  "camera_pan",
  "camera_rotate",
  "camera_tilt",
  "camera_zoom",
  /** The player selected their own units: a click, a box, a card or a
   *  group's key. */
  "selected",
  /** A move ordered with a facing (a right-drag). */
  "moved_with_facing",
  /** The contact jump's key pressed, whether or not a contact was there. */
  "contact_focus",
  "shop_opened",
  /** Every unit's orders shown (the key held). */
  "show_orders",
  "group_set",
] as const;

export type UiSignal = (typeof UI_SIGNALS)[number];

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
