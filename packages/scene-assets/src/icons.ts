// The generated icons under `assets/icons/`: never hand-drawn. Each is a
// small SVG drawn in `currentColor`, so the UI tints it (the side colour
// lives only in the UI and markers).
//   - `weapons/<icon>.svg`: a weapon or ammunition row's `icon`;
//   - `roles/<role>.svg`: a role's NATO-style symbol, its filled frame with
//     the registry's modifiers drawn in order, bold enough to read at 20 px;
//   - `units/<type>.svg`: a unit type's silhouette, rendered from its own
//     model (`silhouette.ts`);
//   - `states/<state>.svg`: a unit state's mark in an info panel row
//     (`STATE_ICONS`), each told apart by its form.
// `asset icons` writes them; `asset check` and a vitest fail when one is
// missing or stale, so a weapon row, role or type without its icon is caught.

import { silhouetteSvg, type Solid } from "./silhouette.ts";
import type { UnitCatalog, UnitType } from "./units.ts";

const svg = (w: number, h: number, body: string, stroke = 1.5) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}" ` +
  `fill="none" stroke="currentColor" stroke-width="${stroke}" stroke-linecap="round" stroke-linejoin="round">` +
  `${body}</svg>\n`;

/** Each weapon icon's drawing on a 24 × 24 grid, the muzzle to the right. */
const WEAPON_ICONS: Record<string, string> = {
  rifle: '<path d="M2 13h13l2-1.5h5"/><path d="M5 13l-1 4h3l1.5-4"/><path d="M11 13l1 3"/>',
  grenade: '<circle cx="11" cy="14" r="5"/><path d="M11 9V6h3"/><path d="M14 6l3-2"/>',
  ap_shell: '<path d="M3 10h11l6 2-6 2H3z"/><path d="M6 10v4"/><path d="M14 10l-2-3M14 14l-2 3"/>',
  he_shell: '<path d="M3 9h11a4 3 0 0 1 0 6H3z"/><path d="M6 9v6"/><path d="M9 9v6"/>',
  hmg: '<path d="M2 11h16h4"/><path d="M4 9h9v5H4z"/><path d="M8 14l-1 5M8 14l3 5"/>',
  atgm: '<path d="M3 12h13l4-2v4l-4-2"/><path d="M6 12l-2-3M6 12l-2 3"/><path d="M3 12c-1 3 1 5 0 8"/>',
};

/** Each info panel state's mark on a 24 × 24 grid, drawn to sit inside a
 *  progress ring. The supply families differ by form: an arrow rising into a
 *  bar (being served), a ticked box (full), a crossed box (cannot). */
export const STATE_ICONS = {
  deploy: '<path d="M12 4v10"/><path d="M7 10l5 5 5-5"/><path d="M4 20h16"/>',
  pack: '<path d="M12 18V8"/><path d="M7 12l5-5 5 5"/><path d="M4 20h16"/>',
  deployed: '<path d="M12 4v7"/><path d="M12 11l-7 9M12 11l7 9M12 11v9"/>',
  stock: '<rect x="3" y="8" width="18" height="12"/><path d="M3 12h18M9 8v4M15 8v4"/>',
  resupply: '<path d="M12 21V9"/><path d="M6 14l6-6 6 6"/><path d="M4 4h16"/>',
  supply_full: '<rect x="4" y="4" width="16" height="16"/><path d="M8 12l3 3 5-6"/>',
  supply_blocked: '<rect x="4" y="4" width="16" height="16"/><path d="M8 8l8 8M16 8l-8 8"/>',
  suppressed: '<path d="M4 4l8 5 8-5"/><path d="M4 10l8 5 8-5"/><path d="M4 20h16"/>',
  building: '<path d="M4 20V10l8-6 8 6v10z"/><path d="M10 20v-6h4v6"/>',
  waiting: '<path d="M9 6v12M15 6v12"/>',
  last_seen:
    '<path d="M2 12c3-5 7-7 10-7s7 2 10 7c-3 5-7 7-10 7s-7-2-10-7z"/><circle cx="12" cy="12" r="3"/>',
  heard:
    '<path d="M3 10v4h3l5 4V6l-5 4z"/><path d="M15 9a4 4 0 0 1 0 6"/><path d="M18 6a8 8 0 0 1 0 12"/>',
  unknown: '<path d="M8.5 9a3.5 3.5 0 1 1 5 3.2c-1 .5-1.5 1.3-1.5 2.3"/><path d="M12 19v.5"/>',
} as const;

export type StateIcon = keyof typeof STATE_ICONS;

/** The icon file of a state's mark. */
export const stateIcon = (state: StateIcon) => `states/${state}.svg`;

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

function weaponIcon(icon: string): string {
  const body = WEAPON_ICONS[icon];
  if (!body) throw new Error(`weapon icon "${icon}" has no drawing in icons.ts WEAPON_ICONS`);
  return svg(24, 24, body);
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
 *  a type's posed model (`unitSolids`), or null when it isn't installed. */
export function iconFiles(
  weapons: Record<string, { icon: string }>,
  units: UnitCatalog,
  solids: (id: string) => Solid[] | null,
): Map<string, string> {
  const files = new Map<string, string>();
  for (const { icon } of Object.values(weapons)) files.set(`weapons/${icon}.svg`, weaponIcon(icon));
  for (const [role, r] of Object.entries(units.view.roles))
    files.set(`roles/${role}.svg`, roleSymbol(role, r.symbol));
  for (const [state, body] of Object.entries(STATE_ICONS))
    files.set(stateIcon(state as StateIcon), svg(24, 24, body, 2));
  for (const t of units.view.units) {
    const model = solids(t.id);
    if (!model?.length)
      throw new Error(
        `unit type ${t.id}: its model is not installed, so its silhouette can't be rendered (bake, or git lfs pull the runtime bundles)`,
      );
    files.set(unitIcons(t).silhouette, silhouetteSvg(model));
  }
  return files;
}

/** The icon files a unit type shows: its silhouette and its first role's symbol. */
export function unitIcons(t: Pick<UnitType, "id" | "roles">): { silhouette: string; role: string } {
  return { silhouette: `units/${t.id}.svg`, role: `roles/${t.roles[0]}.svg` };
}
