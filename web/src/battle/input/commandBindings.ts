/** The one command key table (the user's key decisions): the keys
 *  `useUnitControl` dispatches. Camera keys (WASD, Q/E, arrows) belong to
 *  renderer-core's camera, and UI keys to their own owners (contact focus in
 *  `LabViewport`, the shop and the pause menu in their components); no
 *  command may take one of them. */
import { isGameKey, type KeyPress } from "./keyGuard";

export type KeyCommand =
  | "stop"
  | "attack_move"
  | "reverse_move"
  | "toggle_fire_policy"
  | "attack_ground"
  | "toggle_deployment"
  | "toggle_afterburner"
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
  toggle_fire_policy: { code: "KeyF", label: "F" },
  attack_ground: { code: "KeyG", label: "G" },
  toggle_deployment: { code: "KeyT", label: "T" },
  toggle_afterburner: { code: "KeyB", label: "B" },
  disarm: { code: "Escape", label: "Esc" },
};

/** Held, not pressed (D2+): every own unit's current and final markers,
 *  routes and cover icons. */
export const ShowOrdersBinding: CommandBinding = { code: "Space", label: "Hold Space" };

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
