/** The HUD's controls a lesson can point at, by name. A control declares its
 *  anchor where it is drawn (`hudAnchor`), and the coach finds it by that
 *  name, never by its styling. */
import { CommandBindings, type KeyCommand } from "../input/commandBindings";

export type HudAnchor =
  /** The button that opens the shop. */
  | "shop"
  /** The player's credits and income. */
  | "credits"
  /** Start battle, during preparation. */
  | "start"
  /** The objectives' flags in the top bar, in every phase. */
  | "objectives"
  /** The victory score, once the battle runs. */
  | "score"
  /** The cards of the player's own units. */
  | "army"
  /** A command's button on the command bar. */
  | `command:${KeyCommand}`;

const NAMED = ["shop", "credits", "start", "objectives", "score", "army"] as const;

export function isHudAnchor(name: string): name is HudAnchor {
  if ((NAMED as readonly string[]).includes(name)) return true;
  return name.startsWith("command:") && name.slice("command:".length) in CommandBindings;
}

/** The attribute that makes an element anchor `name`. */
export function hudAnchor(name: HudAnchor): { "data-hud-anchor": HudAnchor } {
  return { "data-hud-anchor": name };
}

/** The element drawn for `name` now, if it is on screen. */
export function anchorElement(name: HudAnchor): HTMLElement | null {
  return document.querySelector<HTMLElement>(`[data-hud-anchor="${name}"]`);
}
