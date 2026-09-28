/** The HUD's look, from the one fixture owner's presentation block
 *  (`presentation.hud`, slice 27e): the holo-tactical accents, text and glass
 *  the readouts, callouts and panel draw with. Applied once as CSS custom
 *  properties (`--hud-<name>`: "r g b" channels, so a rule picks its own
 *  alpha with `rgb(var(--hud-accent) / 0.5)`), so the stylesheet holds no
 *  colour of its own. */
import village from "@fixtures/village.json";

type Rgb = readonly [number, number, number];

export interface HudTheme {
  /** Every label and readout: a local or system monospace stack. */
  font: string;
  /** The observing side's accent: leader lines, borders, selection. */
  accent: Rgb;
  /** The other side's accent. */
  enemy: Rgb;
  text: Rgb;
  /** Secondary text and idle lines. */
  dim: Rgb;
  /** Timers: aiming, reloading, setting up. */
  aim: Rgb;
  reload: Rgb;
  deploy: Rgb;
  /** Why a weapon can't fire, and warnings. */
  warn: Rgb;
  /** Accepted and rejected command outcomes. */
  good: Rgb;
  bad: Rgb;
  /** The panel's glass: its colour and opacity. */
  glass: readonly [number, number, number, number];
  /** Soft glow around lines and text, in CSS pixels. */
  glow_px: number;
}

/** How strongly the callouts (leader lines, rings, names) glow, 1 their
 *  designed glow and 0 none: `presentation.overlay.glow.callouts`, beside the
 *  ground marks' halo (`glow.ground`, the overlay pass's). */
export function validateCalloutGlow(callouts: number): number {
  if (!(callouts >= 0 && callouts <= 4))
    throw new Error(`presentation.overlay.glow.callouts: in [0, 4], got ${callouts}`);
  return callouts;
}

export const villageCalloutGlow: number = validateCalloutGlow(
  village.presentation.overlay.glow.callouts,
);

const COLOURS = [
  "accent",
  "enemy",
  "text",
  "dim",
  "aim",
  "reload",
  "deploy",
  "warn",
  "good",
  "bad",
] as const;

export function validateHudTheme(theme: HudTheme): HudTheme {
  const unit = (c: readonly number[], n: number) =>
    Array.isArray(c) && c.length === n && c.every((v) => v >= 0 && v <= 1);
  if (
    typeof theme.font !== "string" ||
    !COLOURS.every((k) => unit(theme[k], 3)) ||
    !unit(theme.glass, 4) ||
    !(theme.glow_px >= 0)
  )
    throw new Error(
      `presentation.hud: font, rgb in [0, 1] for ${COLOURS.join(", ")}, rgba glass, glow_px ≥ 0`,
    );
  return theme;
}

export const villageHud: HudTheme = validateHudTheme(
  village.presentation.hud as unknown as HudTheme,
);

const channels = (c: readonly number[]) =>
  c
    .slice(0, 3)
    .map((v) => Math.round(v * 255))
    .join(" ");

/** The theme as CSS custom properties, with the callouts' glow scale. */
export function hudProperties(theme: HudTheme, calloutGlow = 1): Record<string, string> {
  const out: Record<string, string> = {
    "--hud-font": theme.font,
    "--hud-callout-glow": String(calloutGlow),
    "--hud-glass": channels(theme.glass),
    "--hud-glass-alpha": String(theme.glass[3]),
    "--hud-glow": `${theme.glow_px}px`,
  };
  for (const k of COLOURS) out[`--hud-${k}`] = channels(theme[k]);
  return out;
}

/** Set the theme on `root` (the document's by default). */
export function applyHudTheme(theme = villageHud, root = document.documentElement) {
  for (const [k, v] of Object.entries(hudProperties(theme, villageCalloutGlow)))
    root.style.setProperty(k, v);
}
