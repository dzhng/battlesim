/** The one command key table (the user's key decisions): the keys
 *  `useUnitControl` dispatches. Camera keys (WASD, Q/E, arrows) belong to
 *  renderer-core's camera, and UI keys to their own owners (contact focus in
 *  `LabViewport`, the shop and the pause menu in their components, full
 *  screen at the app's root); no command may take one of them. */
import { isGameKey, type KeyPress } from "./keyGuard";

export type KeyCommand =
  | "stop"
  | "attack_move"
  | "reverse_move"
  | "toggle_fire_policy"
  | "attack_ground"
  | "toggle_deployment"
  | "toggle_afterburner"
  | "unload"
  | "pop_smoke"
  | "return_to_base"
  | "disarm";

export interface CommandBinding {
  /** `KeyboardEvent.code` of the key. */
  code: string;
  /** How the command bar names the key. */
  label: string;
}

export const CommandBindings: Readonly<Record<KeyCommand, CommandBinding>> = {
  stop: { code: "Backspace", label: "Backspace" },
  attack_move: { code: "KeyC", label: "C or Ctrl+right-click" },
  reverse_move: { code: "KeyR", label: "R, or right-click behind one vehicle" },
  /** F is full screen's, so the fire policy takes H. */
  toggle_fire_policy: { code: "KeyH", label: "H" },
  attack_ground: { code: "KeyG", label: "G" },
  toggle_deployment: { code: "KeyT", label: "T" },
  toggle_afterburner: { code: "KeyV", label: "V" },
  /** Arms Unload; pressed again while armed, unloads where the carriers stand. */
  unload: { code: "KeyU", label: "U" },
  pop_smoke: { code: "KeyM", label: "M" },
  /** Refund: the selection heads home and is refunded on arrival; Shift
   *  sends it once its other orders are done. */
  return_to_base: { code: "KeyB", label: "B" },
  disarm: { code: "Escape", label: "Esc" },
};

/** Held, not pressed (D2+): every own unit's current and final markers,
 *  routes and cover icons. */
export const ShowOrdersBinding: CommandBinding = { code: "Space", label: "Space" };

/** The view to the newest contact, then each older one (Shift: newer). */
export const ContactFocusBinding: CommandBinding = { code: "KeyN", label: "N" };

/** Full screen on and off, anywhere in the app (`fullscreen.ts`). */
export const FullscreenBinding: CommandBinding = { code: "KeyF", label: "F" };

const BY_CODE = new Map(
  (Object.entries(CommandBindings) as [KeyCommand, CommandBinding][]).map(([c, b]) => [b.code, c]),
);

/** The command a key press asks for, or null (not a command key, or guarded). */
export function commandForKey(e: KeyPress): KeyCommand | null {
  return isGameKey(e) ? (BY_CODE.get(e.code) ?? null) : null;
}

/** Ctrl+right-click attack-moves to the clicked ground at once, armed or not. */
export function isAttackMoveClick(pick: { button: "left" | "right"; ctrl: boolean }): boolean {
  return pick.button === "right" && pick.ctrl;
}
