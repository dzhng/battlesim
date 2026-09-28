// The generated icons under `assets/icons/`: never hand-drawn. Each is a
// small SVG drawn in `currentColor`, so the UI tints it (the side colour
// lives only in the UI and markers).
//   - `weapons/<icon>.svg`: a weapon or ammunition row's `icon`;
//   - `roles/<role>.svg`: a role's NATO-style symbol, its frame with the
//     registry's modifiers drawn in order;
//   - `units/<type>-placeholder.svg`: a unit type's silhouette. A placeholder
//     until silhouettes are rendered from the type's model: a side view drawn
//     from its resolved numbers (hull box, turret and gun, or its soldiers).
// `asset icons` writes them; `asset check` and a vitest fail when one is
// missing or stale, so a weapon row, role or type without its icon is caught.

import type { UnitCatalog, UnitType } from "./units.ts";

const svg = (w: number, h: number, body: string) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}" ` +
  `fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">` +
  `${body}</svg>\n`;

const n = (v: number) => Number(v.toFixed(2)).toString();

/** Each weapon icon's drawing on a 24 × 24 grid, the muzzle to the right. */
const WEAPON_ICONS: Record<string, string> = {
  rifle: '<path d="M2 13h13l2-1.5h5"/><path d="M5 13l-1 4h3l1.5-4"/><path d="M11 13l1 3"/>',
  grenade: '<circle cx="11" cy="14" r="5"/><path d="M11 9V6h3"/><path d="M14 6l3-2"/>',
  ap_shell: '<path d="M3 10h11l6 2-6 2H3z"/><path d="M6 10v4"/><path d="M14 10l-2-3M14 14l-2 3"/>',
  he_shell: '<path d="M3 9h11a4 3 0 0 1 0 6H3z"/><path d="M6 9v6"/><path d="M9 9v6"/>',
  hmg: '<path d="M2 11h16h4"/><path d="M4 9h9v5H4z"/><path d="M8 14l-1 5M8 14l3 5"/>',
  atgm: '<path d="M3 12h13l4-2v4l-4-2"/><path d="M6 12l-2-3M6 12l-2 3"/><path d="M3 12c-1 3 1 5 0 8"/>',
};

/** The NATO-style frame, 36 × 24, with each modifier drawn inside or under it. */
const ROLE_MODIFIERS: Record<string, string> = {
  infantry: '<path d="M4 4l28 16M32 4L4 20"/>',
  armour: '<rect x="9" y="8" width="18" height="8" rx="4"/>',
  recon: '<path d="M4 20L32 4"/>',
  anti_armour: '<path d="M4 20L18 4l14 16"/>',
  supply: '<path d="M4 16h28"/>',
  wheeled: '<circle cx="12" cy="27" r="1.6"/><circle cx="24" cy="27" r="1.6"/>',
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
  return svg(36, 30, `<rect x="2" y="2" width="32" height="20"/>${parts.join("")}`);
}

/** A side view (x along the hull, y down) drawn from the type's numbers. */
function placeholderSilhouette(t: UnitType, units: UnitCatalog): string {
  const hull = units.hull(t.id);
  if (!hull) {
    // One standing figure per slot, up to six.
    const count = Math.min(6, units.slots(t.id).length);
    const figures = Array.from({ length: count }, (_, k) => {
      const x = 4 + k * 6;
      return `<circle cx="${x}" cy="7" r="1.8"/><path d="M${x} 9v6M${x} 15l-2 5M${x} 15l2 5M${x - 2} 11h4"/>`;
    });
    return svg(4 + count * 6, 24, figures.join(""));
  }
  // 4 px a metre, the hull on the ground line; the first turret mount as a
  // turret block on the roof over its pivot, its muzzle's reach as the gun.
  const s = 4;
  const [hx, , hz] = hull.half_extents_m;
  const w = 2 * hx * s;
  const roof = 40 - 2 * hz * s;
  const turret = units.turret(t.id);
  const parts = [`<rect x="4" y="${n(roof)}" width="${n(w)}" height="${n(40 - roof)}" rx="2"/>`];
  let reach = 2 * hx;
  if (turret >= 0) {
    const m = t.mounts[turret];
    const cx = 4 + (hx + m.pivot_m[0]) * s;
    parts.push(`<rect x="${n(cx - 6)}" y="${n(roof - 5)}" width="12" height="5" rx="2"/>`);
    if (m.muzzle_m) {
      const gun = roof - 2.5;
      parts.push(`<path d="M${n(cx)} ${n(gun)}H${n(cx + m.muzzle_m[0] * s)}"/>`);
      reach = Math.max(reach, hx + m.pivot_m[0] + m.muzzle_m[0]);
    }
  }
  parts.push(`<path d="M4 41h${n(w)}"/>`);
  return svg(Math.ceil(8 + reach * s), 44, parts.join(""));
}

/** Every generated icon file, by path under `assets/icons/`. */
export function iconFiles(
  weapons: Record<string, { icon: string }>,
  units: UnitCatalog,
): Map<string, string> {
  const files = new Map<string, string>();
  for (const { icon } of Object.values(weapons)) files.set(`weapons/${icon}.svg`, weaponIcon(icon));
  for (const [role, r] of Object.entries(units.view.roles))
    files.set(`roles/${role}.svg`, roleSymbol(role, r.symbol));
  for (const t of units.view.units)
    files.set(`units/${t.id}-placeholder.svg`, placeholderSilhouette(t, units));
  return files;
}

/** The icon files a unit type shows: its silhouette and its first role's symbol. */
export function unitIcons(t: Pick<UnitType, "id" | "roles">): { silhouette: string; role: string } {
  return { silhouette: `units/${t.id}-placeholder.svg`, role: `roles/${t.roles[0]}.svg` };
}
