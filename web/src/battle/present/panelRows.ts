/** What each info panel says, derived from the side's observation alone
 *  (pure, so it is tested without a page). Every panel, own or enemy, is the
 *  same three sections in the same order: its NAME, a row per WEAPON, then
 *  its STATES. Every unit state lives in a panel as an icon and a short
 *  word; the ground keeps only selection and movement marks.
 *  - An own unit's panel (`ownPanel`): its type's name, each weapon with
 *    its rounds left and live timers, and its state rows: deployment, a
 *    building, concealment, suppression, waiting, a truck's stock and its supplying, and a
 *    unit's supply in a set-up truck's reach.
 *  - An identified enemy's panel (`enemyPanel`): its type's name and weapon
 *    types, never a count, health or anything else the side can't know.
 *  - A contact's panel (`contactPanel`): its remembered type's name and weapons,
 *    or UNKNOWN and what was heard, and how long ago. */
import { stateIcon, weaponIcon, type StateIcon } from "@packages/scene-assets/src/icons";
import type { MountRow, UnitCatalog } from "@packages/scene-assets/src/units";
import type { ContactView, MountView, OwnUnitView } from "../sim/observation";

/** The rule blocks the panels read (the scenario's). */
export interface PanelRules {
  tick_hz: number;
  /** Each weapon row's label, icon and full load. The catalog gives every
   *  row its label and icon (`contract::labels`), so a row reads them as
   *  they are and never stands its id in for either. */
  weapons: Record<string, { name: string; icon: string; ammo?: number | "unlimited" }>;
  /** A supply vehicle's reach. */
  service: { radius_m: number };
}

/** Every state row a panel can show: its mark, its word (`n` fills a
 *  number in), whether it is lasting, and its tone. The one list of them, so
 *  the panel workbench can show every one.
 *  - `lasting`: it describes the unit steadily (set up, in a building, its
 *    stock); a panel lists its lasting rows first, then the temporary ones
 *    (something happening now or passing).
 *  - `tone`: warning states, including pinned, draw in the shared warning
 *    colour. */
export const STATE_ROWS = {
  deployed: { icon: "deployed", word: () => "DEPLOYED", lasting: true },
  in_building: { icon: "building", word: () => "IN BUILDING", lasting: true },
  hidden: { icon: "hidden", word: () => "HIDDEN", lasting: true },
  stock: { icon: "stock", word: (n: number) => `SUPPLY ${n}`, lasting: true },
  stock_empty: { icon: "stock", word: (n: number) => `SUPPLY ${n}`, lasting: true, tone: "warn" },
  withdrawing: { icon: "withdrawing", word: () => "RETURNING TO BASE" },
  deploying: { icon: "deploy", word: () => "DEPLOYING" },
  packing: { icon: "pack", word: () => "PACKING" },
  entering: { icon: "building", word: () => "ENTERING" },
  leaving: { icon: "building", word: () => "LEAVING" },
  suppressed: { icon: "suppressed", word: () => "SUPPRESSED", tone: "warn" },
  pinned: { icon: "pinned", word: () => "PINNED", tone: "warn" },
  waiting: { icon: "waiting", word: () => "WAITING" },
  route_blocked: { icon: "route_blocked", word: () => "ROUTE BLOCKED", tone: "warn" },
  supplying: { icon: "resupply", word: () => "SUPPLYING" },
  resupplying: { icon: "resupply", word: () => "RESUPPLYING" },
  last_seen: { icon: "last_seen", word: (n: number) => `LAST SEEN ${n} s AGO` },
  heard: { icon: "heard", word: (n: number) => `HEARD ${n} s AGO` },
} satisfies Record<string, StateRowKind>;

interface StateRowKind {
  icon: StateIcon;
  word: (n: number) => string;
  lasting?: boolean;
  tone?: StateTone;
}

type StateKind = keyof typeof STATE_ROWS;
/** A warning row's shared HUD colour role. */
type StateTone = "warn";

/** One state row: an icon and a short word, with its timer or amount. */
export interface StateRow {
  /** Which state: the row's key and its `data-state`. */
  state: StateKind;
  /** Under `assets/icons/`. */
  icon: string;
  word: string;
  /** A running timer in [0, 1], drawn as a ring filling round the icon;
   *  null for a state with no timer. */
  progress: number | null;
  /** A counted amount against its full amount, in [0, 1], drawn as pips;
   *  null for a state that counts nothing. */
  fill: number | null;
  lasting: boolean;
  tone: StateTone | null;
}

const row = (
  state: StateKind,
  {
    progress = null,
    fill = null,
    n = 0,
  }: { progress?: number | null; fill?: number | null; n?: number } = {},
): StateRow => {
  const s: StateRowKind = STATE_ROWS[state];
  return {
    state,
    icon: stateIcon(s.icon),
    word: s.word(n),
    progress,
    fill,
    lasting: !!s.lasting,
    tone: s.tone ?? null,
  };
};

/** How many of a row's five pips an amount lights: any amount at all shows
 *  one, none shows none, a full one all five. */
export const PIPS = 5;
export function pipsLit(fill: number): number {
  if (!(fill > 0)) return 0;
  return Math.min(PIPS, Math.max(1, Math.round(fill * PIPS)));
}

/** The deployment row of any unit that deploys in place, from its published
 *  deployment alone (never its kind): the progress fills as the change
 *  completes. Packed and staying packed says nothing. */
function deploymentRow(d: OwnUnitView["deployment"]): StateRow | null {
  if (!d) return null;
  if (d.target === "deployed")
    return d.progress >= 1 ? row("deployed") : row("deploying", { progress: d.progress });
  return d.progress > 0 ? row("packing", { progress: 1 - d.progress }) : null;
}

/** Only active service is shown; full and unavailable states stay silent. */
function serviceRow(service: string): StateRow | null {
  return service === "serving" ? row("resupplying") : null;
}

const GARRISON_ROWS: Record<string, (progress: number) => StateRow> = {
  entering: (p) => row("entering", { progress: p }),
  inside: () => row("in_building"),
  exiting: (p) => row("leaving", { progress: p }),
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

/** Every state row of an own unit, in panel order: lasting rows, then
 *  temporary ones. `own` is the side's own units, for a truck's supplying.
 *  Suppression is the published tier's word alone (SUPPRESSED, PINNED):
 *  the sim owns the thresholds, and the level under them stays hidden. */
export function ownStateRows(
  units: UnitCatalog,
  u: OwnUnitView,
  own: readonly OwnUnitView[],
  rules: PanelRules,
): StateRow[] {
  const rows: (StateRow | null)[] = [deploymentRow(u.deployment)];
  if (u.withdrawing) rows.push(row("withdrawing"));
  if (u.garrison) rows.push(GARRISON_ROWS[u.garrison.phase]?.(u.garrison.progress) ?? null);
  if (u.concealed) rows.push(row("hidden"));
  if (u.suppression !== "none") rows.push(row(u.suppression));
  // The published move state: `waiting` yields to vehicle traffic in the
  // way; `route_blocked` has no known route (the order is kept and retried).
  if (u.state === "waiting") rows.push(row("waiting"));
  if (u.state === "route_blocked") rows.push(row("route_blocked"));
  if (u.stock !== null) {
    const full = units.type(u.kind).capabilities?.supply?.stock;
    rows.push(
      row(u.stock > 0 ? "stock" : "stock_empty", {
        n: u.stock,
        fill: full ? Math.min(1, u.stock / full) : null,
      }),
    );
    if (supplying(u, own, rules)) rows.push(row("supplying"));
  }
  rows.push(serviceRow(u.service));
  const shown = rows.filter((r): r is StateRow => r !== null);
  return [...shown.filter((r) => r.lasting), ...shown.filter((r) => !r.lasting)];
}

/** One ammunition kind a weapon fires, as its row names it. */
interface AmmoKind {
  /** The kind's label on a mount of several kinds ("AP SHELL"); null on a mount
   *  of one, whose row's name is the kind's. */
  label: string | null;
  /** Rounds left: a number, null for unlimited; undefined where the side
   *  can't know (an enemy's). */
  count?: number | null;
  /** The kind loaded or being loaded: the one the next shot fires. */
  loaded: boolean;
}

/** An own weapon's live state; an enemy's weapon has none the side knows. */
interface WeaponLive {
  /** Why it is or isn't firing (`infoPanel.tsx` `REASON_MARK` says which
   *  reasons the panel marks). */
  reason: string;
  /** Running timers in [0, 1]; null when complete or not running. */
  aim: number | null;
  reload: number | null;
  guiding: boolean;
  /** Defensive interception cooldown uses the same timer ring. */
  cooldown?: number | null;
}

/** One weapon row: its icon, its name and each kind it fires. The same
 *  row, own or enemy; an own row adds its counts and live state. */
export interface WeaponRow {
  key: string;
  /** Under `assets/icons/`: the loaded (or first) kind's. */
  icon: string | null;
  /** Its one kind's label, or the mount's label for a mount of several. */
  name: string;
  kinds: AmmoKind[];
  live: WeaponLive | null;
  /** The loaded kind's rounds left against its full load, in [0, 1], drawn
   *  as pips; null for unlimited rounds, or an enemy's. */
  fill: number | null;
}

/** Which timers a mount shows: none once complete (U03). */
export function mountTimers(mount: MountView): { aim: number | null; reload: number | null } {
  return {
    aim: mount.target && mount.aim < 1 ? mount.aim : null,
    reload: mount.loaded === null && mount.reload > 0 ? mount.reload : null,
  };
}

const rowIcon = (rules: PanelRules, row: string | undefined) =>
  row === undefined ? null : weaponIcon(rules.weapons[row].icon);

/** A mount's row: one kind's label, or the mount's label over each kind.
 *  `loaded` is the kind the next shot fires; -1 where the side can't know. */
function mountRow(
  key: string,
  rows: readonly string[],
  mountName: string,
  rules: PanelRules,
  loaded: number,
  counts?: readonly (number | null)[],
): Omit<WeaponRow, "live" | "fill"> {
  const rowName = (r: string) => rules.weapons[r].name.toUpperCase();
  const single = rows.length === 1;
  return {
    key,
    icon: rowIcon(rules, rows[loaded] ?? rows[0]),
    name: single ? rowName(rows[0]) : mountName.toUpperCase(),
    kinds: rows.map((r, k) => ({
      label: single ? null : rowName(r),
      ...(counts ? { count: counts[k] === undefined ? 0 : counts[k] } : {}),
      loaded: k === loaded,
    })),
  };
}

/** An own mount's row: every kind's rounds left, the loaded kind marked,
 *  and its live timers and reason. */
function ownWeaponRow(mounts: readonly MountRow[], m: MountView, rules: PanelRules): WeaponRow {
  const mount = mounts[m.mount];
  const rows = mount.weapons;
  const loaded =
    m.loaded ??
    m.reloading ??
    Math.max(
      0,
      m.ammo.findIndex((n) => n === null || n > 0),
    );
  const { aim, reload } = mountTimers(m);
  return {
    ...mountRow(String(m.mount), rows, mount.name, rules, loaded, m.ammo),
    name: equipmentName(mounts, m.mount, rules),
    live: { reason: m.reason, aim, reload, guiding: m.guiding },
    fill: ammoFill(m.ammo[loaded], rules.weapons[rows[loaded]]?.ammo),
  };
}

/** Distinguish physical guns by their authored labels. A lone gun uses its
 *  weapon's label; twins use their mounts' labels, or stable ordinals when
 *  those repeat. */
function equipmentName(mounts: readonly MountRow[], index: number, rules: PanelRules): string {
  const mount = mounts[index];
  const short =
    mount.weapons.length === 1
      ? rules.weapons[mount.weapons[0]].name.toUpperCase()
      : mount.name.toUpperCase();
  const peers = mounts.flatMap((m, i) =>
    m.weapons.length === mount.weapons.length && m.weapons.every((r, k) => r === mount.weapons[k])
      ? [i]
      : [],
  );
  if (peers.length === 1) return short;
  if (new Set(peers.map((i) => mounts[i].name.toUpperCase())).size === peers.length)
    return mount.name.toUpperCase();
  return `${short} ${peers.indexOf(index) + 1}`;
}

/** The equipment rows shared by floating panels and the army deck's detail.
 *  Without own readiness, expose equipment alone, never ammo or timers. */
export function weaponRows(
  mounts: readonly MountRow[],
  rules: PanelRules,
  readiness?: readonly MountView[],
  protection?: { name: string; icon: string; capacity: number },
  protectionReadiness?: OwnUnitView["protection"],
): WeaponRow[] {
  const rows = readiness
    ? readiness.map((m) => ownWeaponRow(mounts, m, rules))
    : mounts.map((m, k) => ({
        ...mountRow(String(k), m.weapons, m.name, rules, -1),
        name: equipmentName(mounts, k, rules),
        live: null,
        fill: null,
      }));
  if (protection)
    rows.push({
      key: "protection",
      icon: weaponIcon(protection.icon),
      name: protection.name.toUpperCase(),
      kinds: [
        {
          label: null,
          ...(protectionReadiness ? { count: protectionReadiness.charges } : {}),
          loaded: !!protectionReadiness,
        },
      ],
      live: protectionReadiness
        ? {
            reason: protectionReadiness.charges === 0 ? "no_ammo" : "ready",
            aim: null,
            reload: null,
            guiding: false,
            cooldown: protectionReadiness.cooldown,
          }
        : null,
      fill: protectionReadiness ? protectionReadiness.charges / protection.capacity : null,
    });
  return rows;
}

/** Rounds left against a full load; null when either is unlimited or
 *  unknown. */
function ammoFill(n: number | null | undefined, full: number | "unlimited" | undefined) {
  return typeof n === "number" && typeof full === "number" && full > 0
    ? Math.min(1, n / full)
    : null;
}

/** A row's kinds and counts as words: "AP 20 · HE 15", "∞", "AP · HE" on
 *  an enemy's; "" for an enemy's one-kind row. */
export function weaponCounts(w: Pick<WeaponRow, "kinds">): string {
  const count = (k: AmmoKind) =>
    k.count === undefined ? "" : k.count === null ? "∞" : String(k.count);
  return w.kinds.map((k) => [k.label, count(k)].filter(Boolean).join(" ")).join(" · ");
}

/** A row's words as one line: "MAIN GUN AP SHELL · HE SHELL", or with
 *  counts "MAIN GUN AP SHELL 20 · HE SHELL 15", "RIFLE ∞". */
export function weaponLabel(w: Pick<WeaponRow, "name" | "kinds">): string {
  return [w.name, weaponCounts(w)].filter(Boolean).join(" ");
}

/** Strength in [0, 1]: a vehicle's hit points, or a squad's soldiers' health
 *  against the full squad (each slot's soldier kind), so losses show as well
 *  as wounds. A squad a battle started tougher than its type reads full. */
export function unitStrength(
  units: UnitCatalog,
  u: Pick<OwnUnitView, "kind" | "hp" | "memberHp">,
): number {
  const hull = units.hull(u.kind);
  if (hull) return u.hp / hull.hp;
  const full = units.slots(u.kind).reduce((sum, kind) => sum + units.soldier(kind).hp, 0);
  return Math.min(1, u.memberHp.reduce((a, b) => a + b, 0) / (full || 1));
}

/** One info panel: NAME (with its strength), then WEAPONS, then STATES. */
export interface Panel {
  name: string;
  /** Its strength against full, in [0, 1], drawn as pips on the name line;
   *  null where the side can't know it (an enemy's). */
  strength: number | null;
  /** Living members, including wounded soldiers; absent without known personnel. */
  personnel?: number;
  /** A mark before the name (under `assets/icons/`): an unidentified
   *  report's; null for a known type. */
  mark: string | null;
  weapons: WeaponRow[];
  states: StateRow[];
}

/** An own unit's panel. `own` is the side's own units (a truck's
 *  supplying reads them). */
export function ownPanel(
  units: UnitCatalog,
  u: OwnUnitView,
  own: readonly OwnUnitView[],
  rules: PanelRules,
): Panel {
  const t = units.type(u.kind);
  return {
    name: t.name.toUpperCase(),
    strength: unitStrength(units, u),
    personnel: u.memberHp.length ? u.memberHp.filter((hp) => hp > 0).length : undefined,
    mark: null,
    weapons: weaponRows(t.mounts, rules, u.mounts, t.capabilities.active_protection, u.protection),
    states: ownStateRows(units, u, own, rules),
  };
}

/** An identified enemy's panel: its type's name and every mount's weapon
 *  types, from the catalog alone. The observation says nothing of its
 *  ammunition, health or timers, and neither does this. */
export function enemyPanel(units: UnitCatalog, kind: string, rules: PanelRules): Panel {
  const t = units.type(kind);
  return {
    name: t.name.toUpperCase(),
    strength: null,
    mark: null,
    weapons: weaponRows(t.mounts, rules, undefined, t.capabilities.active_protection),
    states: [],
  };
}

/** The weapon types a firing report heard: each heard row under the first
 *  catalog mount that fires it, named as that mount would be, once. */
export function heardWeapons(
  units: UnitCatalog,
  heard: readonly string[],
  rules: PanelRules,
): WeaponRow[] {
  const mounts = units.view.units.flatMap((t) => t.mounts);
  const tags = new Map<string, WeaponRow>();
  for (const r of heard) {
    const m = mounts.find((m) => m.weapons.includes(r));
    const tag = m ? mountRow("", m.weapons, m.name, rules, -1) : mountRow("", [r], r, rules, -1);
    const label = weaponLabel(tag);
    tags.set(label, { ...tag, key: label, live: null, fill: null });
  }
  return [...tags.values()];
}

/** Whole seconds since `tick` at the published `now`. */
const since = (now: number, tick: number, rules: PanelRules) =>
  Math.max(0, Math.floor((now - tick) / rules.tick_hz));

/** Remembered type or heard weapons, with the latest evidence's age. */
export function contactPanel(
  units: UnitCatalog,
  c: Pick<ContactView, "source" | "kind" | "heard" | "evidenceTick">,
  now: number,
  rules: PanelRules,
): Panel {
  const ago = since(now, c.evidenceTick, rules);
  if (c.kind)
    return {
      ...enemyPanel(units, c.kind, rules),
      states: [row(c.source === "last_seen" ? "last_seen" : "heard", { n: ago })],
    };
  return {
    name: "UNKNOWN",
    strength: null,
    mark: stateIcon("unknown"),
    weapons: heardWeapons(units, c.heard, rules),
    states: [row("heard", { n: ago })],
  };
}
