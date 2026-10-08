// The generated icons under `assets/icons/`: never hand-drawn. Each is a
// small SVG drawn in `currentColor`, so the UI tints it (the side colour
// lives only in the UI and markers).
//   - `weapons/<icon>.svg`: the `icon` of a weapon or ammunition row, of an
//     active protection, or of a disabled card's planned weapon;
//   - `roles/<role>.svg`: a role's NATO-style symbol, its filled frame with
//     the registry's modifiers drawn in order, bold enough to read at 20 px;
//   - `units/<card>.svg`: a card's silhouette, rendered from its own model
//     (`silhouette.ts`): a unit type's baked model, or a disabled card's
//     source model (it has no type);
//   - `states/<state>.svg`: a unit state's mark in an info panel row
//     (`STATE_ICONS`), each told apart by its form;
//   - `glyphs/<glyph>.svg`: a mark drawn where text would be ambiguous at
//     panel size (`GLYPHS`: unlimited ammunition's ∞);
//   - `hud/<mark>.svg`: a command tile's, a menu control's and a weapon's
//     can't-fire mark (`HUD_ICONS`).
// Weapon, state, glyph and HUD icons are centred on their ink (`inkBounds`), not
// on the grid they were drawn on, except the cursor arrow whose tip is its
// fixed hotspot. Centred marks sit exactly in the middle of the
// slot or ring the UI gives it.
// `asset icons` writes them; `asset check` and a vitest fail when one is
// missing or stale, so a weapon row, role or type without its icon is caught.

import { inkBounds } from "./inkBounds.ts";
import { silhouetteSvg, type Solid } from "./silhouette.ts";
import type { UnitCatalog, UnitType } from "./units.ts";

const svg = (w: number, h: number, body: string, stroke = 1.5) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}" ` +
  `fill="none" stroke="currentColor" stroke-width="${stroke}" stroke-linecap="round" stroke-linejoin="round">` +
  `${body}</svg>\n`;

/** A `w` × `h` icon whose view box is centred on its drawing's ink. */
const centred = (w: number, h: number, body: string, stroke = 1.5) => {
  const [x0, y0, x1, y1] = inkBounds(body);
  const at = (c: number, size: number) => Number((c - size / 2).toFixed(3));
  const box = `${at((x0 + x1) / 2, w)} ${at((y0 + y1) / 2, h)} ${w} ${h}`;
  return svg(w, h, body, stroke).replace(`viewBox="0 0 ${w} ${h}"`, `viewBox="${box}"`);
};

/** Each weapon icon's drawing on a 24 × 24 grid, the muzzle to the right
 *  (a falling bomb's nose and a launched missile's point where they fly).
 *  Each differs from its neighbours by form: a scope makes the sniper, a
 *  braked barrel on a block the autocannon, a cone the RPG's warhead. */
const WEAPON_ICONS: Record<string, string> = {
  trophy: '<path d="M12 3l8 4v6c0 4-4 7-8 9-4-2-8-5-8-9V7z"/><path d="M7 12h10M12 8v8"/>',
  rifle: '<path d="M2 13h13l2-1.5h5"/><path d="M5 13l-1 4h3l1.5-4"/><path d="M11 13l1 3"/>',
  grenade: '<circle cx="11" cy="14" r="5"/><path d="M11 9V6h3"/><path d="M14 6l3-2"/>',
  ap_shell: '<path d="M3 10h11l6 2-6 2H3z"/><path d="M6 10v4"/><path d="M14 10l-2-3M14 14l-2 3"/>',
  he_shell: '<path d="M3 9h11a4 3 0 0 1 0 6H3z"/><path d="M6 9v6"/><path d="M9 9v6"/>',
  hmg: '<path d="M2 11h16h4"/><path d="M4 9h9v5H4z"/><path d="M8 14l-1 5M8 14l3 5"/>',
  atgm: '<path d="M3 12h13l4-2v4l-4-2"/><path d="M6 12l-2-3M6 12l-2 3"/><path d="M3 12c-1 3 1 5 0 8"/>',
  sniper:
    '<path d="M2 14h14l2-1h4"/><path d="M5 14l-1 4h3l1.5-4"/><rect x="8" y="8.5" width="6" height="2.5" rx="1"/><path d="M11 11v3"/>',
  autocannon:
    '<path d="M3 9h9v6H3z"/><path d="M12 12h8"/><path d="M18 10v4M21 10v4"/><path d="M6 15v4h3v-4"/>',
  rpg: '<path d="M2 11h11v2H2z"/><path d="M13 8.5h3l5 3.5-5 3.5h-3z"/><path d="M5 13v4M9 13v3"/>',
  aa_missile: '<path d="M5 19L18 6"/><path d="M15 5h4v4"/><path d="M8 16l-4-1M8 16l1 4"/>',
  bomb: '<path d="M9 8h6v7a3 3 0 0 1-6 0z"/><path d="M10 8L7 3M14 8l3-5M12 8V3"/>',
  howitzer: '<path d="M5 16L20 5"/><circle cx="8" cy="17" r="3"/><path d="M8 17l-5 3"/>',
  rockets:
    '<path d="M3 7h14M3 12h14M3 17h14"/><path d="M17 5.5l3 1.5-3 1.5M17 10.5l3 1.5-3 1.5M17 15.5l3 1.5-3 1.5"/>',
  mortar: '<path d="M7 19L16 4"/><path d="M3 20h8"/><path d="M12 11l4 9"/>',
};

/** An open eye on a 24 × 24 grid: a last sighting, and (struck through)
 *  no line of sight. */
const EYE =
  '<path d="M2 12c3-5 7-7 10-7s7 2 10 7c-3 5-7 7-10 7s-7-2-10-7z"/><circle cx="12" cy="12" r="3"/>';

const CLOSED_EYE = `${EYE}<path d="M4 21L20 3"/>`;

/** Each info panel state's mark on a 24 × 24 grid, drawn to sit inside a
 *  progress ring. The supply families differ by form: an arrow rising into a
 *  bar (being served), a ticked box (full), a crossed box (cannot). Waiting
 *  (a pause) and a blocked route (an arrow meeting a wall) differ too. */
const STATE_ICONS = {
  withdrawing: '<path d="M20 5H8v14"/><path d="M3 14l5 5 5-5"/><path d="M16 19h5"/>',
  deploy: '<path d="M12 4v10"/><path d="M7 10l5 5 5-5"/><path d="M4 20h16"/>',
  pack: '<path d="M12 18V8"/><path d="M7 12l5-5 5 5"/><path d="M4 20h16"/>',
  deployed: '<path d="M12 4v7"/><path d="M12 11l-7 9M12 11l7 9M12 11v9"/>',
  stock: '<rect x="3" y="8" width="18" height="12"/><path d="M3 12h18M9 8v4M15 8v4"/>',
  resupply: '<path d="M12 21V9"/><path d="M6 14l6-6 6 6"/><path d="M4 4h16"/>',
  supply_full: '<rect x="4" y="4" width="16" height="16"/><path d="M8 12l3 3 5-6"/>',
  supply_blocked: '<rect x="4" y="4" width="16" height="16"/><path d="M8 8l8 8M16 8l-8 8"/>',
  suppressed: '<path d="M4 4l8 5 8-5"/><path d="M4 10l8 5 8-5"/><path d="M4 20h16"/>',
  // Pinned: pressed flat between two bars, where suppressed is pressed down.
  pinned:
    '<path d="M4 4h16"/><path d="M4 9l8 4 8-4"/><path d="M4 14l8 4 8-4"/><path d="M4 21h16"/>',
  building: '<path d="M4 20V10l8-6 8 6v10z"/><path d="M10 20v-6h4v6"/>',
  waiting: '<path d="M9 6v12M15 6v12"/>',
  route_blocked: '<path d="M3 12h11"/><path d="M10 8l4 4-4 4"/><path d="M19 5v14"/>',
  hidden: CLOSED_EYE,
  last_seen: EYE,
  heard:
    '<path d="M3 10v4h3l5 4V6l-5 4z"/><path d="M15 9a4 4 0 0 1 0 6"/><path d="M18 6a8 8 0 0 1 0 12"/>',
  unknown: '<path d="M8.5 9a3.5 3.5 0 1 1 5 3.2c-1 .5-1.5 1.3-1.5 2.3"/><path d="M12 19v.5"/>',
} as const;

export type StateIcon = keyof typeof STATE_ICONS;

/** Marks drawn where a font's glyph misreads at panel size, on a 16 × 8
 *  grid: unlimited ammunition's ∞, two even loops crossing at the middle (a
 *  monospace ∞ at 12 px read as an 8 or a 2). */
const GLYPHS = {
  unlimited: '<path d="M8 4C10 1 14.5 1 14.5 4S10 7 8 4 1.5 1 1.5 4 6 7 8 4z"/>',
} as const;

/** The icon file of a glyph. */
export const glyphIcon = (glyph: keyof typeof GLYPHS) => `glyphs/${glyph}.svg`;

/** Each HUD mark on a 24 × 24 grid: a command tile's, the menu button's, and
 *  why a weapon can't fire (an info panel's warning mark). The HUD draws no
 *  font glyph or emoji as an icon; every one is here or among the state
 *  marks. Each differs from its neighbours by form. */
const HUD_ICONS = {
  objective: '<path d="M6 21V3h13l-3 4 3 4H6"/>',
  // Tip at (2, 2) is a cursor hotspot, so this icon keeps its authored grid.
  cursor_arrow: '<path d="M2 2L25 11L14 15L11 26Z"/>',
  move: '<path d="M4 12h15"/><path d="M13 6l6 6-6 6"/>',
  attack_move:
    '<path d="M3 12h8"/><path d="M8 8l4 4-4 4"/><circle cx="17" cy="12" r="4"/><path d="M17 5v3M17 16v3M22 12h-1"/>',
  reverse: '<path d="M20 12H5"/><path d="M11 6l-6 6 6 6"/><path d="M20 7v10"/>',
  attack_ground:
    '<circle cx="12" cy="10" r="5"/><path d="M12 2v4M12 14v4M4 10h4M16 10h4"/><path d="M3 21h18"/>',
  fast_move: '<path d="M4 6l6 6-6 6"/><path d="M11 6l6 6-6 6"/><path d="M18 6v12"/>',
  stop: '<rect x="6" y="6" width="12" height="12"/>',
  fire_at_will:
    '<circle cx="12" cy="12" r="3"/><path d="M12 2v5M12 17v5M2 12h5M17 12h5M5 5l3.5 3.5M15.5 15.5L19 19M19 5l-3.5 3.5M8.5 15.5L5 19"/>',
  hold_fire: '<circle cx="12" cy="12" r="8"/><path d="M7 12h10"/>',
  leave_building: '<path d="M4 20V10l7-6 7 6"/><path d="M11 15h10"/><path d="M18 12l3 3-3 3"/>',
  menu: '<path d="M4 6h16M4 12h16M4 18h16"/>',
  // A bare chevron: back out of a menu page, unlike reverse's barred arrow.
  back: '<path d="M15 4l-8 8 8 8"/>',
  rejected: '<path d="M6 6l12 12M18 6L6 18"/>',
  guiding: '<path d="M3 20c5 0 6-8 10-8"/><path d="M13 12h5l3-2v4l-3-2"/>',
  blocked_shot:
    '<path d="M2 12h3M8 12h3"/><path d="M15 3v18"/><path d="M15 8l5-5M15 14l5-5M15 20l5-5"/>',
  friendly_in_line:
    '<path d="M2 12h5"/><rect x="8" y="7" width="8" height="10"/><path d="M17 12h5"/>',
  turret:
    '<circle cx="12" cy="12" r="3"/><path d="M20 12a8 8 0 1 1-2.3-5.7"/><path d="M18 2v5h-5"/>',
  must_stop: '<path d="M8.5 3h7L21 8.5v7L15.5 21h-7L3 15.5v-7z"/><path d="M8 12h8"/>',
  no_ammo: '<rect x="8" y="3" width="8" height="18"/><path d="M4 20L20 4"/>',
  no_sight: CLOSED_EYE,
} as const;

export type HudIcon = keyof typeof HUD_ICONS;

/** The icon file of a HUD mark. */
export const hudIcon = (icon: HudIcon) => `hud/${icon}.svg`;

/** The icon file of a state's mark. */
export const stateIcon = (state: StateIcon) => `states/${state}.svg`;

/** The icon file of a weapon or ammunition row's `icon`. */
export const weaponIcon = (icon: string) => `weapons/${icon}.svg`;

/** The icon file of a role's symbol. */
const roleIcon = (role: string) => `roles/${role}.svg`;

/** The NATO-style frame (a friendly unit's rectangle, 36 × 22 at (3, 3)) and
 *  each modifier drawn inside or under it. */
const ROLE_FRAME =
  '<rect x="3" y="3" width="36" height="22" fill="currentColor" fill-opacity="0.28"/>';
const ROLE_MODIFIERS: Record<string, string> = {
  infantry: '<path d="M3 3l36 22M39 3L3 25"/>',
  armour: '<rect x="11" y="9" width="20" height="10" rx="5"/>',
  recon: '<path d="M3 25L39 3"/>',
  anti_armour: '<path d="M3 25L21 3l18 22"/>',
  supply: '<path d="M3 17h36"/>',
  wheeled:
    '<circle cx="14" cy="30" r="2.2" fill="currentColor"/><circle cx="28" cy="30" r="2.2" fill="currentColor"/>',
};

function weaponSvg(icon: string): string {
  const body = WEAPON_ICONS[icon];
  if (!body) throw new Error(`weapon icon "${icon}" has no drawing in icons.ts WEAPON_ICONS`);
  return centred(24, 24, body);
}

function roleSymbol(role: string, modifiers: readonly string[]): string {
  const parts = modifiers.map((m) => {
    const body = ROLE_MODIFIERS[m];
    if (!body) throw new Error(`role ${role}: symbol modifier "${m}" has no drawing in icons.ts`);
    return body;
  });
  return svg(42, 34, `${ROLE_FRAME}${parts.join("")}`, 3.2);
}

/** Every generated icon file, by path under `assets/icons/`. `solids` gives
 *  a type's posed model (`unitSolids`) or a disabled card's
 *  (`disabledLookup`), by id, or null when it isn't installed. */
export function iconFiles(
  weapons: Record<string, { icon: string }>,
  units: UnitCatalog,
  solids: (id: string) => Solid[] | null,
): Map<string, string> {
  const files = new Map<string, string>();
  const weapon = (icon: string) => files.set(weaponIcon(icon), weaponSvg(icon));
  for (const { icon } of Object.values(weapons)) weapon(icon);
  for (const t of units.view.units) {
    const protection = t.capabilities.active_protection;
    if (protection) weapon(protection.icon);
  }
  for (const card of units.cards) for (const { icon } of card.planned_weapons) weapon(icon);
  for (const [role, r] of Object.entries(units.view.roles))
    files.set(roleIcon(role), roleSymbol(role, r.symbol));
  for (const [state, body] of Object.entries(STATE_ICONS))
    files.set(stateIcon(state as StateIcon), centred(24, 24, body, 2));
  for (const [glyph, body] of Object.entries(GLYPHS))
    files.set(glyphIcon(glyph as keyof typeof GLYPHS), centred(16, 8, body, 1.6));
  for (const [icon, body] of Object.entries(HUD_ICONS))
    files.set(
      hudIcon(icon as HudIcon),
      icon === "cursor_arrow" ? svg(32, 32, body, 1.8) : centred(24, 24, body, 2),
    );
  for (const t of units.view.units) {
    const model = solids(t.id);
    if (!model?.length)
      throw new Error(
        `unit type ${t.id}: its model is not installed, so its silhouette can't be rendered (bake, or git lfs pull the runtime bundles)`,
      );
    files.set(unitIcons(t).silhouette, silhouetteSvg(model));
  }
  for (const card of units.cards) {
    if (card.disabled_reason === null || units.has(card.id)) continue;
    const model = solids(card.id);
    if (!model?.length)
      throw new Error(
        `disabled card ${card.id}: its source model is not installed, so its silhouette can't be rendered (git lfs pull its source_path in fixtures/units/model-manifest.json)`,
      );
    files.set(cardIcon(card.id), silhouetteSvg(model));
  }
  return files;
}

/** The silhouette icon of a card, by its id: a unit type's or a disabled card's. */
export const cardIcon = (id: string) => `units/${id}.svg`;

/** The icon files a unit type shows: its silhouette and its first role's symbol. */
export function unitIcons(t: Pick<UnitType, "id" | "roles">): { silhouette: string; role: string } {
  return { silhouette: cardIcon(t.id), role: roleIcon(t.roles[0]) };
}
