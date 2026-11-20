/** The HUD's look, from `fixtures/presentation.json`
 *  (`presentation.hud`): the holo-tactical accents, text and glass
 *  the readouts, callouts and panel draw with. Applied once as CSS custom
 *  properties (`--hud-<name>`: "r g b" channels, so a rule picks its own
 *  alpha with `rgb(var(--hud-accent) / 0.5)`), so the stylesheet holds no
 *  colour of its own. */
import { presentationSettings } from "@web/presentationSettings";
import type { Vec3 } from "math";
import { applyHudProperties, hudProperties } from "./hudCss";

export interface HudTheme {
  /** Every label and readout: a local or system monospace stack. */
  font: string;
  /** The observing side's accent: the callouts, their leader lines, and
   *  borders. */
  accent: Readonly<Vec3>;
  /** Pale ice: the observing side's units another player on it owns (its
   *  allies'), told from the player's own by form as well (D24). */
  ally: Readonly<Vec3>;
  /** The other side's accent. */
  enemy: Readonly<Vec3>;
  text: Readonly<Vec3>;
  /** Secondary text and idle lines. */
  dim: Readonly<Vec3>;
  /** Why a weapon can't fire, and warnings. */
  warn: Readonly<Vec3>;
  /** A pinned squad: a hotter warning than `warn`, still apart from the
   *  enemy's red. */
  pinned: Readonly<Vec3>;
  /** An accepted command outcome (the labs' logs). */
  good: Readonly<Vec3>;
  /** The one "can't" colour: a rejected order, a route the unit can't take,
   *  a range ruler past its last reach. Told from the enemy's red by form. */
  bad: Readonly<Vec3>;
  /** Money: the credits and every price that can be paid. */
  credits: Readonly<Vec3>;
  /** A control group's key and ×, on the white tab in its bracket's corner. */
  group_ink: Readonly<Vec3>;
  /** An army card's blink when its unit is shot at or hurt (`fireBlink.ts`). */
  fire_blink: { on_s: number; off_s: number; wait_s: number; depth: number; glow: number };
  /** The main menu's backdrop film, graded to one military amber that the
   *  menu plate's blue reads against. Never a mark or readout. */
  film: Readonly<Vec3>;
  /** The panel's glass: its colour and opacity. */
  glass: readonly [number, number, number, number];
  /** Soft glow around lines and text, in CSS pixels. */
  glow_px: number;
  /** The least space between a callout and another callout, a unit's anchor
   *  or a HUD bar, in CSS pixels: stacked panels never touch. */
  panel_gap_px: number;
  /** Hide anchors this near the viewport edge. */
  panel_edge_hide_px: number;
}

/** How strongly the callouts (leader lines, rings, names) glow, 1 their
 *  designed glow and 0 none: `presentation.overlay.glow.callouts`, beside the
 *  ground marks' halo (`glow.ground`, the overlay pass's). */

export const gameCalloutGlow: number = presentationSettings.overlay.glow.callouts;

export const gameHud: HudTheme = presentationSettings.hud as unknown as HudTheme;

/** Set the game's theme on the document. */
export function applyHudTheme() {
  applyHudProperties(
    hudProperties(gameHud, presentationSettings.overlay.orders.selected, gameCalloutGlow),
  );
}
