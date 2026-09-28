/** What each info panel says, derived from the side's observation alone
 *  (pure, so it is tested without a page). Every unit state lives in a
 *  panel as an icon and a short word; the ground keeps only selection and
 *  movement marks.
 *  - An own unit's state rows (`ownStateRows`): deployment, a building,
 *    suppression, waiting, a truck's stock and its supplying, and a unit's
 *    supply in a set-up truck's reach. Its weapon rows are `readouts.tsx`'s.
 *  - An identified enemy's panel (`enemyPanel`): its type's name and weapon
 *    types, never a count, health or anything else the side can't know.
 *  - A contact's panel (`contactPanel`): a last sighting's name and weapons
 *    as identified, or UNKNOWN and what was heard, and how long ago. */
import { stateIcon, type StateIcon } from "@packages/scene-assets/src/icons";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { ContactView, OwnUnitView } from "../sim/observation";

/** The rule blocks the panels read (the scenario's). */
export interface PanelRules {
  tick_hz: number;
  /** Each weapon row's display name and icon. */
  weapons: Record<string, { name: string; icon?: string }>;
  /** The level at which a squad is pinned. */
  suppression: { collapse_level: number };
  /** A supply vehicle's reach. */
  service: { radius_m: number };
}

/** One state row: an icon (in a ring when the state has a timer or level)
 *  and a short word. */
export interface StateRow {
  /** Which state: the row's key and its `data-state`. */
  state: string;
  /** Under `assets/icons/`. */
  icon: string;
  word: string;
  /** A timer or level in [0, 1], drawn as the ring filling round the icon;
   *  null for a state that just holds. */
  progress: number | null;
}

const row = (state: string, icon: StateIcon, word: string, progress: number | null = null) => ({
  state,
  icon: stateIcon(icon),
  word,
  progress,
});

/** Suppression under this (a rounded 0%) shows nothing. */
const SUPPRESSION_SHOWN = 0.005;

/** The deployment row of any unit that deploys in place, from its published
 *  deployment alone (never its kind): the ring fills as the change
 *  completes. Packed and staying packed says nothing. */
function deploymentRow(d: OwnUnitView["deployment"]): StateRow | null {
  if (!d) return null;
  if (d.target === "deployed")
    return d.progress >= 1
      ? row("deployed", "deployed", "DEPLOYED")
      : row("deploying", "deploy", "DEPLOYING", d.progress);
  return d.progress > 0 ? row("packing", "pack", "PACKING", 1 - d.progress) : null;
}

/** A unit's supply row while a set-up truck has it in reach: RESUPPLYING,
 *  SUPPLY FULL or CANNOT SUPPLY (moving, firing, or the truck can't pay).
 *  Nothing out of reach or before the truck is set up. */
function serviceRow(service: string): StateRow | null {
  switch (service) {
    case "serving":
      return row("resupplying", "resupply", "RESUPPLYING");
    case "full":
      return row("supply_full", "supply_full", "SUPPLY FULL");
    case "moving":
    case "firing":
    case "no_stock":
      return row("cannot_supply", "supply_blocked", "CANNOT SUPPLY");
    default:
      return null;
  }
}

const GARRISON_ROWS: Record<string, (progress: number) => StateRow> = {
  entering: (p) => row("entering", "building", "ENTERING", p),
  waiting_for_room: () => row("no_room", "building", "NO ROOM"),
  inside: () => row("in_building", "building", "IN BUILDING"),
  exiting: (p) => row("leaving", "building", "LEAVING", p),
};

/** A supply vehicle is supplying while it stands set up and a unit in its
 *  reach is being served. */
function supplying(truck: OwnUnitView, own: readonly OwnUnitView[], rules: PanelRules): boolean {
  if (truck.deployment?.progress !== 1 || truck.state !== "idle") return false;
  const [x, y] = truck.position;
  return own.some(
    (u) =>
      u.service === "serving" &&
      Math.hypot(u.position[0] - x, u.position[1] - y) <= rules.service.radius_m,
  );
}

/** Every state row of an own unit, in panel order. `own` is the side's own
 *  units, for a truck's supplying. */
export function ownStateRows(
  u: OwnUnitView,
  own: readonly OwnUnitView[],
  rules: PanelRules,
): StateRow[] {
  const rows: (StateRow | null)[] = [deploymentRow(u.deployment)];
  if (u.garrison) rows.push(GARRISON_ROWS[u.garrison.phase]?.(u.garrison.progress) ?? null);
  if (u.suppression >= SUPPRESSION_SHOWN)
    rows.push(
      u.suppression >= rules.suppression.collapse_level
        ? row("pinned", "suppressed", "PINNED", u.suppression)
        : row("suppressed", "suppressed", "SUPPRESSED", u.suppression),
    );
  if (u.state === "waiting") rows.push(row("waiting", "waiting", "WAITING"));
  if (u.stock !== null) {
    rows.push(row(u.stock > 0 ? "stock" : "stock_empty", "stock", `SUPPLY ${u.stock}`));
    if (supplying(u, own, rules)) rows.push(row("supplying", "resupply", "SUPPLYING"));
  }
  rows.push(serviceRow(u.service));
  return rows.filter((r): r is StateRow => r !== null);
}

/** One weapon type an enemy is known to carry: its label and icon. */
export interface WeaponTag {
  label: string;
  icon: string | null;
}

/** A mount's label: its one row's name, or the mount's name with every row
 *  it fires ("CANNON AP · HE"). */
function mountTag(rows: readonly string[], name: string, rules: PanelRules): WeaponTag {
  const rowName = (r: string) => rules.weapons[r]?.name ?? r;
  const label = rows.length === 1 ? rowName(rows[0]) : `${name} ${rows.map(rowName).join(" · ")}`;
  const icon = rules.weapons[rows[0]]?.icon;
  return { label: label.toUpperCase(), icon: icon ? `weapons/${icon}.svg` : null };
}

/** An enemy panel: a name, its weapon types and, for a contact, how long
 *  ago the evidence came. */
export interface EnemyPanel {
  name: string;
  /** A mark before the name (under `assets/icons/`): an unidentified
   *  report's; null for a known type. */
  mark: string | null;
  weapons: WeaponTag[];
  /** A contact's evidence row; null for an identified enemy. */
  evidence: StateRow | null;
}

/** An identified enemy's panel: its type's name and every mount's weapon
 *  types, from the catalog alone. The observation says nothing of its
 *  ammunition, health or timers, and neither does this. */
export function enemyPanel(
  kind: string,
  rules: PanelRules,
  units: UnitCatalog = UNITS,
): Omit<EnemyPanel, "evidence"> {
  const t = units.type(kind);
  return {
    name: t.name.toUpperCase(),
    mark: null,
    weapons: t.mounts.map((m) => mountTag(m.weapons, m.name, rules)),
  };
}

/** The weapon types a firing report heard: each heard row under the first
 *  catalog mount that fires it, named as that mount would be, once. */
export function heardWeapons(
  heard: readonly string[],
  rules: PanelRules,
  units: UnitCatalog = UNITS,
): WeaponTag[] {
  const mounts = units.view.units.flatMap((t) => t.mounts);
  const tags = new Map<string, WeaponTag>();
  for (const r of heard) {
    const m = mounts.find((m) => m.weapons.includes(r));
    const tag = m ? mountTag(m.weapons, m.name, rules) : mountTag([r], r, rules);
    tags.set(tag.label, tag);
  }
  return [...tags.values()];
}

/** Whole seconds since `tick` at the published `now`. */
const since = (now: number, tick: number, rules: PanelRules) =>
  Math.max(0, Math.floor((now - tick) / rules.tick_hz));

/** A contact's panel at the published tick `now`: a last sighting's name and
 *  weapons as identified and LAST SEEN n s AGO; a firing report's UNKNOWN,
 *  what was heard, and HEARD n s AGO. */
export function contactPanel(
  c: Pick<ContactView, "source" | "kind" | "heard" | "evidenceTick">,
  now: number,
  rules: PanelRules,
  units: UnitCatalog = UNITS,
): EnemyPanel {
  const ago = since(now, c.evidenceTick, rules);
  if (c.source === "last_seen" && c.kind)
    return {
      ...enemyPanel(c.kind, rules, units),
      evidence: row("last_seen", "last_seen", `LAST SEEN ${ago} s AGO`),
    };
  return {
    name: "UNKNOWN",
    mark: stateIcon("unknown"),
    weapons: heardWeapons(c.heard, rules, units),
    evidence: row("heard", "heard", `HEARD ${ago} s AGO`),
  };
}
