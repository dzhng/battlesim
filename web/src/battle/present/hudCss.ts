/** The HUD theme as the stylesheet reads it: CSS custom properties on the
 *  document (`hudTheme.ts`). It needs no WebAssembly, so a page that never
 *  loads the game (the unsupported-browser notice) still wears its look. */
import type { HudTheme } from "./hudTheme";

const COLOURS = [
  "accent",
  "ally",
  "enemy",
  "text",
  "dim",
  "warn",
  "pinned",
  "good",
  "bad",
  "credits",
  "group_ink",
  "film",
] as const;

/** A 0–1 colour's rgb as CSS channels ("r g b", 0–255): `rgb(${…})`. */
export const channels = (c: readonly number[]) =>
  c
    .slice(0, 3)
    .map((v) => Math.round(v * 255))
    .join(" ");

/** The theme as CSS custom properties, with the callouts' glow scale and the
 *  selection ring's colour (`presentation.overlay.orders.selected`), which a
 *  selected unit's leader shares to tie its ring to its card. */
export function hudProperties(
  theme: HudTheme,
  selected: readonly number[],
  calloutGlow = 1,
): Record<string, string> {
  const out: Record<string, string> = {
    "--hud-font": theme.font,
    "--hud-callout-glow": String(calloutGlow),
    "--hud-selected": channels(selected),
    "--hud-glass": channels(theme.glass),
    "--hud-glass-alpha": String(theme.glass[3]),
    "--hud-glow": `${theme.glow_px}px`,
    "--hud-blink-depth": `${theme.fire_blink.depth * 100}%`,
    "--hud-blink-glow": String(theme.fire_blink.glow),
  };
  for (const k of COLOURS) out[`--hud-${k.replaceAll("_", "-")}`] = channels(theme[k]);
  return out;
}

/** Set `properties` on the document. */
export function applyHudProperties(properties: Record<string, string>) {
  for (const [k, v] of Object.entries(properties)) document.documentElement.style.setProperty(k, v);
}
