/** Player readouts of own units' readiness (U02, U03), from observation only:
 *  - holo callouts off each unit on a leader line: per weapon, a
 *    reload ring round an aim ring beside the rounds left (∞ when
 *    unlimited); completed timers vanish; a guidance mark while guiding; a
 *    separate deployment square;
 *  - the selected-unit panel, which keeps every detail at any zoom;
 *  - the command bar, exposing every village action and the fire policy.
 *  Enemies never get readouts: only own units carry readiness. */
import { useCallback, useImperativeHandle, useRef, type Ref } from "react";
import type { MountView, OwnUnitView } from "../sim/observation";
import type { CommandMode } from "../input/useUnitControl";
import { CommandBindings, FacingBinding } from "../input/commandBindings";
import { reach, type ReachCommand } from "../input/commandReach";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { unitIcons } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";

type Project = (x: number, y: number, z: number) => [number, number] | null;
type Point3 = readonly [number, number, number];

/** The rule block the readouts read (the scenario's): each weapon row's
 *  display name and icon. A unit's type (its name, mounts, full strength)
 *  is the unit catalog's. */
export interface ReadoutRules {
  weapons: Record<string, { name: string; icon?: string }>;
}

/** A unit type's silhouette and role symbol, as the unit card shows them. */
function UnitIcons({ kind }: { kind: string }) {
  const { silhouette, role } = unitIcons(UNITS.type(kind));
  return (
    <span className="ro-unit-icons">
      <Icon path={role} className="ro-icon ro-role" />
      <Icon path={silhouette} className="ro-icon ro-silhouette" />
    </span>
  );
}

/** The icon of a mount's loaded (or first) weapon row. */
function WeaponIcon({
  unit,
  mount,
  rules,
}: {
  unit: OwnUnitView;
  mount: MountView;
  rules: ReadoutRules;
}) {
  const rows = mountWeapons(unit, mount);
  const icon = rules.weapons[rows[mount.loaded ?? 0] ?? rows[0]]?.icon;
  return icon ? <Icon path={`weapons/${icon}.svg`} /> : null;
}
/** Above this camera distance, rings show only for selected units. */
export const RINGS_FAR_M = 700;

/** Reasons the ring itself shows as a badge: anything that is not plain progress. */
const QUIET = new Set(["firing", "aiming", "reloading", "guiding"]);

/** Every sim action reason in player words; none names a hidden obstacle. */
export const REASON_TEXT: Record<string, string> = {
  firing: "firing",
  no_compatible_target: "no target it can hurt",
  holding_fire: "holding fire (return fire only)",
  out_of_range: "out of range",
  blocked_trajectory: "no clear shot",
  friendly_in_line: "friendly vehicle in the way",
  aiming: "aiming",
  reloading: "reloading",
  turret_traversing: "turning turret",
  moving_stationary_weapon: "must stop to use",
  out_of_ammo: "out of ammunition",
  tracking_last_sighting: "tracking last sighting",
  guiding: "guiding a missile",
  no_own_sight: "needs its own sight of the target",
  no_facing_slot: "no firing position facing the target",
  changing_position: "entering or leaving a building",
};

/** A short glyph per reason, so no state is told apart by colour alone. */
const REASON_GLYPH: Record<string, string> = {
  firing: "✹",
  aiming: "◎",
  reloading: "↻",
  turret_traversing: "⟳",
  guiding: "⌖",
  tracking_last_sighting: "?",
  holding_fire: "✋",
  out_of_range: "↔",
  blocked_trajectory: "▦",
  no_compatible_target: "⊘",
  friendly_in_line: "⚠",
  moving_stationary_weapon: "⏸",
  out_of_ammo: "∅",
  no_own_sight: "◉",
  no_facing_slot: "⊟",
  changing_position: "⇄",
};

/** Every supply service state in player words (for a waiting state, why). */
export const SERVICE_TEXT: Record<string, string> = {
  out_of_range: "no supply vehicle in reach",
  source_not_deployed: "supply vehicle not set up yet",
  moving: "must stand still",
  firing: "fired this moment",
  serving: "being served",
  no_stock: "the truck cannot pay for the next item",
  full: "nothing missing",
  garrisoned: "in a building: no replacements",
};

/** Service states in which a unit in a truck's reach waits to be served (the
 *  broken ring on the map). */
export const SERVICE_WAITING: ReadonlySet<string> = new Set([
  "moving",
  "firing",
  "no_stock",
  "garrisoned",
  "source_not_deployed",
]);

/** A unit's supply state in words: "waiting for supply: <why>" under the
 *  broken ring, else the state itself. */
export function serviceText(u: Pick<OwnUnitView, "service">): string {
  const words = SERVICE_TEXT[u.service] ?? u.service;
  return SERVICE_WAITING.has(u.service) ? `waiting for supply: ${words}` : words;
}

/** Each garrison phase in player words. */
export const GARRISON_PHASE_TEXT: Record<string, string> = {
  entering: "entering",
  waiting_for_room: "no room: waiting",
  inside: "inside",
  exiting: "leaving",
};

/** The name a unit goes by in the panel, the log and on the map: its
 *  type's name, never a callsign. */
export function unitName(u: Pick<OwnUnitView, "kind">): string {
  return UNITS.type(u.kind).name;
}

/** Strength in [0, 1]: a vehicle's hit points, or a squad's soldiers' health
 *  against the full squad (each slot's soldier kind), so losses show as well
 *  as wounds. */
export function unitStrength(u: OwnUnitView): number {
  const hull = UNITS.hull(u.kind);
  if (hull) return u.hp / hull.hp;
  const full = UNITS.slots(u.kind).reduce((sum, kind) => sum + UNITS.soldier(kind).hp, 0);
  return u.memberHp.reduce((a, b) => a + b, 0) / (full || 1);
}

/** The garrison phase, with the timer while entering or leaving. */
export function garrisonText(u: OwnUnitView): string {
  const g = u.garrison;
  if (!g) return "outside";
  const timer = g.phase === "entering" || g.phase === "exiting";
  return `${GARRISON_PHASE_TEXT[g.phase] ?? g.phase}${timer ? ` ${(g.progress * 100).toFixed(0)}%` : ""}`;
}

/** A mount's weapon rows (its ammunition kinds), in order. */
function mountWeapons(unit: Pick<OwnUnitView, "kind">, mount: MountView): readonly string[] {
  return UNITS.type(unit.kind).mounts[mount.mount]?.weapons ?? [];
}

export function weaponName(unit: OwnUnitView, mount: MountView): string {
  return UNITS.type(unit.kind).mounts[mount.mount]?.name ?? `weapon ${mount.mount + 1}`;
}

/** The short caption under a mount's ring: its one weapon row's name, or
 *  for a mount of several rows (a cannon's AP and HE) the mount's own. */
export function mountCaption(unit: OwnUnitView, mount: MountView, rules: ReadoutRules): string {
  const rows = mountWeapons(unit, mount);
  const name =
    rows.length === 1 ? (rules.weapons[rows[0]]?.name ?? rows[0]) : weaponName(unit, mount);
  return name.toUpperCase();
}

/** The rounds shown inside a mount's ring: the loaded (or next) kind's count. */
export function ringAmmo(unit: OwnUnitView, mount: MountView, rules: ReadoutRules): string {
  const kinds = mountWeapons(unit, mount);
  const k = mount.loaded ?? mount.reloading ?? mount.ammo.findIndex((n) => n === null || n > 0);
  const n = mount.ammo[Math.max(0, k)];
  const count = n === null ? "∞" : String(n ?? 0);
  const label = kinds.length > 1 ? (rules.weapons[kinds[Math.max(0, k)]]?.name ?? "") : "";
  return label ? `${label}${count}` : count;
}

/** Which timers a mount shows: none once complete (U03). */
export function ringTimers(mount: MountView): { aim: number | null; reload: number | null } {
  return {
    aim: mount.target && mount.aim < 1 ? mount.aim : null,
    reload: mount.loaded === null && mount.reload > 0 ? mount.reload : null,
  };
}

function arc(r: number, fraction: number): string {
  // A sliver still reads as "started".
  const f = Math.min(Math.max(fraction, 0.04), 0.9999);
  const a = f * Math.PI * 2 - Math.PI / 2;
  const [x, y] = [Math.cos(a) * r, Math.sin(a) * r];
  return `M 0 ${-r} A ${r} ${r} 0 ${f > 0.5 ? 1 : 0} 1 ${x.toFixed(2)} ${y.toFixed(2)}`;
}

/** One weapon's line in a callout: its ring (reload round it, aim inside it,
 *  in its heart the glyph of why it can't fire or the guidance mark), the
 *  rounds left and, on a selected unit, the weapon's caption. */
function MountRing({
  unit,
  mount,
  rules,
}: {
  unit: OwnUnitView;
  mount: MountView;
  rules: ReadoutRules;
}) {
  const { aim, reload } = ringTimers(mount);
  const blocked = !QUIET.has(mount.reason);
  return (
    <div className="ro-mount">
      <svg
        className="ro-ring"
        viewBox="-12 -12 60 24"
        data-reason={mount.reason}
        data-aim={aim ?? ""}
        data-reload={reload ?? ""}
      >
        <circle r={9} className="ro-track" />
        {reload !== null && <path d={arc(9, reload)} className="ro-reload" />}
        {aim !== null && <path d={arc(5.5, aim)} className="ro-aim" />}
        {mount.guiding && (
          <text className="ro-guide" y={3.5}>
            ⌖
          </text>
        )}
        {blocked && (
          <text className="ro-badge" y={3.5}>
            <title>{REASON_TEXT[mount.reason] ?? mount.reason}</title>
            {REASON_GLYPH[mount.reason] ?? "!"}
          </text>
        )}
        <text className="ro-ammo" x={16} y={4}>
          {ringAmmo(unit, mount, rules)}
        </text>
      </svg>
      <span className="ro-caption">{mountCaption(unit, mount, rules)}</span>
    </div>
  );
}

function DeploymentRing({ unit }: { unit: OwnUnitView }) {
  const d = unit.deployment!;
  const done = d.progress >= 1;
  return (
    <div className="ro-mount">
      <svg className="ro-ring ro-deploy" viewBox="-12 -12 60 24" data-deploy={d.progress}>
        <rect x={-9} y={-9} width={18} height={18} className="ro-track" />
        {!done && d.progress > 0 && <path d={arc(6, d.progress)} className="ro-deploy-arc" />}
        <text className="ro-ammo" x={16} y={4}>
          {done ? "✓" : d.target === "deployed" ? "▲" : "▼"}
        </text>
      </svg>
      <span className="ro-caption">SETUP</span>
    </div>
  );
}

/** Page-pixel box. */
interface Box {
  x0: number;
  x1: number;
  y0: number;
  y1: number;
}
const overlaps = (a: Box, b: Box) => a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1;
/** Space kept between a tag and a panel or another tag. */
const TAG_GAP_PX = 4;
/** Where a callout sits from its unit's anchor: its near bottom corner this
 *  far to the side and up, so the leader line rises off the unit. */
const CALLOUT_SIDE_PX = 30;
const CALLOUT_RISE_PX = 34;
/** The ring a leader line starts from, on its unit. */
const LEADER_RING_PX = 2;
/** Kept clear of the viewport's edge; a callout that would cross the right
 *  edge hangs to the unit's left instead. */
const EDGE_PX = 8;
/** A callout eases to a new nudge (another readout to clear, a bar) with
 *  this time constant, on the presentation clock: about 95% of the way in
 *  150 ms. */
const NUDGE_TAU_S = 0.05;

type Nudge = { dx: number; dy: number };

/** A callout's drawn nudge after the presentation clock steps `dt` seconds:
 *  eased from `had` toward `want`; snapped when there was none, or when the
 *  clock is held or rewound (a paused battle's callouts still clear each
 *  other as the camera moves, and a held-clock capture is the settled
 *  layout). */
export function easeNudge(had: Nudge | undefined, want: Nudge, dt: number): Nudge {
  if (!had || !(dt > 0)) return want;
  const k = 1 - Math.exp(-dt / NUDGE_TAU_S);
  return { dx: had.dx + (want.dx - had.dx) * k, dy: had.dy + (want.dy - had.dy) * k };
}

export interface ReadoutLayerHandle {
  /** Re-anchor every callout; call once per animation frame. `positions` are
   *  the drawn (interpolated) unit positions, so callouts move with meshes;
   *  `clock` is that frame's presentation clock in seconds, which eases a
   *  callout's nudge (null snaps it). */
  place(
    project: Project,
    distance: number,
    positions?: ReadonlyMap<number, Point3>,
    clock?: number | null,
  ): void;
}

/** Holo callouts for own units: each unit's weapon readout floats
 *  up and to the side of it, joined to it by a thin leader line that runs
 *  under the readout, with no box behind it; a selected unit's also carries
 *  its name and weapon captions. A destination carries no text: its marker
 *  and route say whose it is. Positioned by the viewport each frame. Nothing
 *  is placed under an element marked `data-occludes-readouts` (a panel or
 *  bar): it moves the shortest way out. Nothing overprints: a callout that
 *  would cover another rises above it, easing there on the presentation
 *  clock rather than jumping a row. */
export function ReadoutLayer({
  own,
  rules,
  selected,
  handle,
}: {
  /** The observation's own units: all a readout reads. Only these are the
   *  prop, not the whole observation, whose rounds in flight would swell
   *  React's development measure of each commit's changed props. */
  own: readonly OwnUnitView[];
  rules: ReadoutRules;
  selected: readonly number[];
  handle: Ref<ReadoutLayerHandle>;
}) {
  const nodes = useRef(new Map<number, HTMLDivElement>());
  const leaders = useRef(new Map<number, SVGPathElement>());
  // Each callout's drawn nudge from its natural spot, and the clock it was
  // drawn at.
  const nudges = useRef(new Map<number, Nudge>());
  const lastClock = useRef<number | null>(null);
  const units = useRef<readonly OwnUnitView[]>([]);
  units.current = own;
  const selectedRef = useRef(selected);
  selectedRef.current = selected;
  useImperativeHandle(handle, () => ({
    place(project, distance, positions, clock = null) {
      // Each callout's unit anchor, in page pixels.
      const anchored: { id: number; node: HTMLDivElement; x: number; y: number }[] = [];
      for (const u of units.current) {
        const picked = selectedRef.current.includes(u.id);
        const node = nodes.current.get(u.id);
        if (node) {
          // Zoomed out, callouts stay only for the selection; the panel keeps all.
          const shown = distance < RINGS_FAR_M || picked;
          const p = positions?.get(u.id) ?? u.position;
          const at = shown ? project(p[0], p[1], p[2] + 2) : null;
          node.style.display = at ? "flex" : "none";
          if (at) anchored.push({ id: u.id, node, x: at[0], y: at[1] });
          else {
            leaders.current.get(u.id)?.setAttribute("d", "");
            nudges.current.delete(u.id);
          }
        }
      }
      // The presentation clock's step, which eases each callout's nudge.
      const dt = clock === null || lastClock.current === null ? 0 : clock - lastClock.current;
      lastClock.current = clock;
      // One layout read for the frame: the panels to keep clear of, then sizes.
      const panels = [...document.querySelectorAll("[data-occludes-readouts]")].map((e) =>
        e.getBoundingClientRect(),
      );
      const boxes = anchored.map((a) => ({
        ...a,
        w: a.node.offsetWidth,
        h: a.node.offsetHeight,
      }));
      // Never under a panel: move the shortest way out of it, right of a
      // side panel, below a top bar, above a bottom bar.
      const clear = (box: Box): Box => {
        for (const r of panels) {
          if (overlaps({ x0: r.left, x1: r.right, y0: r.top, y1: r.bottom }, box)) {
            const right = r.right + TAG_GAP_PX - box.x0;
            const down = r.bottom + TAG_GAP_PX - box.y0;
            const up = r.top - TAG_GAP_PX - box.y1;
            const dy = Math.abs(down) < Math.abs(up) ? down : up;
            box =
              right <= Math.abs(dy)
                ? { ...box, x0: box.x0 + right, x1: box.x1 + right }
                : shift(box, dy);
          }
        }
        return box;
      };
      const placed: Box[] = [];
      const hit = (box: Box) => placed.find((o) => overlaps(o, box));
      const shift = (box: Box, dy: number): Box => ({ ...box, y0: box.y0 + dy, y1: box.y1 + dy });
      const right = window.innerWidth - EDGE_PX;
      // Callouts, lowest unit first: one that would cover another rises above
      // it. Each unit's own anchor is kept clear too, so no callout sits on a
      // unit.
      const callouts = [...boxes].sort((m, n) => n.y - m.y);
      for (const b of callouts) placed.push({ x0: b.x - 6, x1: b.x + 6, y0: b.y - 6, y1: b.y + 6 });
      for (const b of callouts) {
        const left = b.x + CALLOUT_SIDE_PX + b.w > right;
        const x0 = left ? b.x - CALLOUT_SIDE_PX - b.w : b.x + CALLOUT_SIDE_PX;
        const y1 = b.y - CALLOUT_RISE_PX;
        const natural = { x0, x1: x0 + b.w, y0: y1 - b.h, y1 };
        let target = clear(natural);
        for (let o = hit(target); o; o = hit(target))
          target = shift(target, o.y0 - TAG_GAP_PX - target.y1);
        target = clear(target);
        // Layout holds the target, so the next callout clears where this one
        // is going; this one is drawn eased toward it.
        placed.push(target);
        const want = { dx: target.x0 - natural.x0, dy: target.y0 - natural.y0 };
        const n = easeNudge(nudges.current.get(b.id), want, dt);
        nudges.current.set(b.id, n);
        const box = {
          x0: natural.x0 + n.dx,
          x1: natural.x1 + n.dx,
          y0: natural.y0 + n.dy,
          y1: natural.y1 + n.dy,
        };
        b.node.style.transform = `translate(${box.x0}px, ${box.y0}px)`;
        // The leader: a small ring on the unit, a line off it to the
        // callout's near bottom corner, then along the callout's foot.
        const [near, far] = box.x0 >= b.x ? [box.x0, box.x1] : [box.x1, box.x0];
        const [x, y] = [b.x.toFixed(1), b.y.toFixed(1)];
        const r = LEADER_RING_PX;
        leaders.current
          .get(b.id)
          ?.setAttribute(
            "d",
            `M ${b.x - r} ${y} a ${r} ${r} 0 1 0 ${2 * r} 0 a ${r} ${r} 0 1 0 ${-2 * r} 0 ` +
              `M ${x} ${y} L ${near.toFixed(1)} ${box.y1.toFixed(1)} L ${far.toFixed(1)} ${box.y1.toFixed(1)}`,
          );
      }
    },
  }));
  const bind = useCallback(
    <E extends Element>(map: Map<number, E>, id: number) =>
      (el: E | null) => {
        if (el) map.set(id, el);
        else map.delete(id);
      },
    [],
  );
  const callout = (u: OwnUnitView) => {
    const setup = !!u.deployment && u.deployment.progress > 0 && u.deployment.progress < 1;
    return u.mounts.length > 0 || setup || selected.includes(u.id);
  };
  return (
    <div className="ro-layer" data-testid="readouts">
      <svg className="ro-leaders" aria-hidden="true">
        {own.filter(callout).map((u) => (
          <path
            key={u.id}
            ref={bind(leaders.current, u.id)}
            className={selected.includes(u.id) ? "ro-leader ro-selected" : "ro-leader"}
          />
        ))}
      </svg>
      {own.filter(callout).map((u) => {
        const picked = selected.includes(u.id);
        const setup = !!u.deployment && u.deployment.progress > 0 && u.deployment.progress < 1;
        return (
          <div
            key={u.id}
            ref={bind(nodes.current, u.id)}
            className={`ro-unit${picked ? " ro-selected" : ""}`}
            data-unit={u.id}
          >
            {picked && <span className="ro-name">{unitName(u)}</span>}
            {u.mounts.map((m) => (
              <MountRing key={m.mount} unit={u} mount={m} rules={rules} />
            ))}
            {setup && <DeploymentRing unit={u} />}
          </div>
        );
      })}
    </div>
  );
}

/** The unit card, at any zoom. One unit: every detail (policy, set-up,
 *  strength and pinning, building and supply state, and each weapon). A
 *  group: a count, then one compact row a unit (name, strength, each
 *  weapon's state glyph and rounds), so the card never overflows. */
export function SelectionPanel({
  units,
  rules,
}: {
  units: readonly OwnUnitView[];
  rules: ReadoutRules;
}) {
  if (units.length === 0) return <div className="lab-hint">No unit selected</div>;
  if (units.length > 1)
    return (
      <div className="ro-panel ro-group" data-testid="selection-panel">
        <div className="ro-group-head">{units.length} units selected</div>
        {units.map((u) => (
          <div key={u.id} className="ro-group-row" data-unit={u.id}>
            <strong>
              <Icon path={unitIcons(UNITS.type(u.kind)).role} className="ro-icon ro-role" />
              <Icon
                path={unitIcons(UNITS.type(u.kind)).silhouette}
                className="ro-icon ro-silhouette"
              />
              {unitName(u)}
            </strong>
            <meter
              min={0}
              max={1}
              low={0.35}
              high={0.7}
              optimum={1}
              value={unitStrength(u)}
              aria-label="strength"
            />
            <span className="ro-group-arms">
              {u.mounts.map((m) => (
                <span
                  key={m.mount}
                  title={`${weaponName(u, m)}: ${REASON_TEXT[m.reason] ?? m.reason}`}
                >
                  <span className="ro-glyph">{REASON_GLYPH[m.reason] ?? "·"}</span>
                  {ringAmmo(u, m, rules)}
                </span>
              ))}
            </span>
          </div>
        ))}
      </div>
    );
  return (
    <div className="ro-panel" data-testid="selection-panel">
      {units.map((u) => (
        <div key={u.id} className="ro-panel-unit" data-unit={u.id}>
          <div className="ro-unit-head">
            <UnitIcons kind={u.kind} />
            <div>
              <strong>{unitName(u)}</strong>
              <div className="ro-unit-state">
                {u.engagement === "fire_at_will" ? "fire at will" : "return fire only"}
                {u.deployment && ` · ${deploymentText(u)}`}
              </div>
            </div>
          </div>
          <UnitCondition unit={u} />
          {u.mounts.map((m) => (
            <div key={m.mount} className="ro-panel-mount" data-reason={m.reason}>
              <span className="ro-glyph">{REASON_GLYPH[m.reason] ?? "·"}</span>
              <WeaponIcon unit={u} mount={m} rules={rules} />
              <span>
                {weaponName(u, m)}: {REASON_TEXT[m.reason] ?? m.reason}
                {m.guiding && m.reason !== "guiding" && " · guiding a missile"} ·{" "}
                {ammoText(u, m, rules)}
                {timersText(m)}
              </span>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}

/** Strength, pinning (infantry), building and supply state. */
function UnitCondition({ unit: u }: { unit: OwnUnitView }) {
  const strength = unitStrength(u);
  const infantry = u.members.length > 0;
  return (
    <>
      <div className="lab-bar">
        <span>{infantry ? `${u.members.length} soldiers` : `${u.hp.toFixed(0)} hp`}</span>
        <meter min={0} max={1} low={0.35} high={0.7} optimum={1} value={strength} />
        <span>{(strength * 100).toFixed(0)}%</span>
      </div>
      {infantry && (
        <div className="lab-bar">
          <span>pinned</span>
          <meter min={0} max={1} low={0.3} high={0.6} optimum={0} value={u.suppression} />
          <span>{(u.suppression * 100).toFixed(0)}%</span>
        </div>
      )}
      {(u.garrison || (u.stock === null && u.service !== "full")) && (
        <div className="lab-hint" data-testid={`condition-${u.id}`}>
          {u.garrison && `building: ${garrisonText(u)}`}
          {u.garrison && u.stock === null && u.service !== "full" && " · "}
          {u.stock === null &&
            u.service !== "full" &&
            (SERVICE_WAITING.has(u.service) ? serviceText(u) : `supply: ${serviceText(u)}`)}
        </div>
      )}
    </>
  );
}

function deploymentText(u: OwnUnitView): string {
  const d = u.deployment!;
  if (d.progress >= 1) return "deployed";
  if (d.progress <= 0 && d.target === "packed") return "packed";
  return `${d.target === "deployed" ? "deploying" : "packing"} ${Math.round(d.progress * 100)}%`;
}

function ammoText(u: OwnUnitView, m: MountView, rules: ReadoutRules): string {
  const kinds = mountWeapons(u, m);
  return m.ammo
    .map((n, k) => {
      const label = kinds.length > 1 ? `${rules.weapons[kinds[k]]?.name ?? kinds[k]} ` : "";
      const loaded = m.loaded === k && kinds.length > 1 ? " (loaded)" : "";
      return `${label}${n === null ? "∞" : n}${loaded}`;
    })
    .join(", ");
}

/** Timer progress exactly as published (seconds would assume the nominal
 *  rate, which suppression slows). */
function timersText(m: MountView): string {
  const { aim, reload } = ringTimers(m);
  const parts = [];
  if (aim !== null) parts.push(`aim ${Math.floor(aim * 100)}%`);
  if (reload !== null) parts.push(`reload ${Math.floor(reload * 100)}%`);
  return parts.length ? ` · ${parts.join(", ")}` : "";
}

export interface CommandBarProps {
  mode: CommandMode;
  setMode: (mode: CommandMode) => void;
  selected: readonly OwnUnitView[];
  onStop: () => void;
  onTogglePolicy: () => void;
  onDeploy: (deployed: boolean) => void;
  onExit: () => void;
}

/** One command: its glyph, its short name and the key on a chip. The
 *  accessible name (and the tooltip) is the full wording, every alternative
 *  gesture included. A command only part of the selection can carry out
 *  shows how many it reaches ("1/2"). */
function CommandButton({
  glyph,
  name,
  keyHint,
  label,
  pressed,
  reach: reaches,
  of,
  disabled,
  onClick,
}: {
  glyph: string;
  name: string;
  keyHint: string;
  label: string;
  pressed?: boolean;
  /** How many of the selection's `of` units it reaches, for a command only
   *  some units can carry out. */
  reach?: number;
  of?: number;
  disabled: boolean;
  onClick: () => void;
}) {
  const partial = reaches !== undefined && of !== undefined && reaches > 0 && reaches < of;
  return (
    <button
      type="button"
      className="ro-cmd"
      aria-label={label}
      title={partial ? `${label}: ${reaches} of ${of} selected` : label}
      aria-pressed={pressed}
      data-reach={partial ? `${reaches}/${of}` : undefined}
      onClick={onClick}
      disabled={disabled}
    >
      <span className="ro-cmd-glyph" aria-hidden="true">
        {glyph}
      </span>
      <span className="ro-cmd-name" aria-hidden="true">
        {name}
      </span>
      {partial && (
        <span className="ro-cmd-reach" aria-hidden="true">
          {reaches}/{of}
        </span>
      )}
      <kbd aria-hidden="true">{keyHint}</kbd>
    </button>
  );
}

/** The key chip for a binding: its first key ("X or Ctrl+right-click" → X). */
const chip = (command: keyof typeof CommandBindings) =>
  CommandBindings[command].label.split(/,| or /)[0].replace("Backspace", "⌫");

/** The modes whose order is an attack, reaching only armed units. */
const ATTACKS = new Set<CommandMode>(["attack_move", "attack_ground"]);

/** Every village action and the fire policy; keys are optional shortcuts,
 *  named from the one binding table. */
export function CommandBar(p: CommandBarProps) {
  const key = (command: keyof typeof CommandBindings) => `(${CommandBindings[command].label})`;
  const any = p.selected.length > 0;
  // The union of the selection's capabilities: lit when any unit can.
  const reached = (command: ReachCommand) => reach(command, p.selected, UNITS).length;
  const [armed, deployers, squads, inside] = [
    reached("attack"),
    reached("deploy"),
    reached("garrison"),
    reached("exit_building"),
  ];
  const reachOf = (m: CommandMode) =>
    m === "garrison" ? squads : ATTACKS.has(m) ? armed : undefined;
  const hold = any && p.selected.every((u) => u.engagement === "return_fire_only");
  const mode = (m: CommandMode, glyph: string, name: string, keyHint: string, label: string) => (
    <CommandButton
      glyph={glyph}
      name={name}
      keyHint={keyHint}
      label={label}
      pressed={p.mode === m}
      reach={reachOf(m)}
      of={p.selected.length}
      disabled={m === "garrison" || ATTACKS.has(m) ? !reachOf(m) : !any}
      onClick={() => p.setMode(m)}
    />
  );
  return (
    <div className="ro-commands" role="toolbar" aria-label="Commands">
      {mode(
        "move",
        "➤",
        "Move",
        "RMB",
        `Move (right-click; ${FacingBinding.label.toLowerCase()} faces)`,
      )}
      {mode(
        "attack_move",
        "⇶",
        "Attack-move",
        chip("attack_move"),
        `Attack-move ${key("attack_move")}`,
      )}
      {mode("reverse_move", "⇠", "Reverse", chip("reverse_move"), `Reverse ${key("reverse_move")}`)}
      {mode(
        "attack_ground",
        "⌖",
        "Attack ground",
        chip("attack_ground"),
        `Attack ground ${key("attack_ground")}`,
      )}
      {mode("fast_move", "»", "Fast move", "2×RMB", "Fast move (double right-click)")}
      {mode("garrison", "⌂", "Garrison", "RMB", "Garrison (right-click a building)")}
      <CommandButton
        glyph="■"
        name="Stop"
        keyHint={chip("stop")}
        label={`Stop ${key("stop")}`}
        disabled={!any}
        onClick={p.onStop}
      />
      <CommandButton
        glyph={hold ? "✋" : "✹"}
        name={hold ? "Return fire" : "Fire at will"}
        keyHint={chip("toggle_fire_policy")}
        label={`${hold ? "Return fire only" : "Fire at will"} ${key("toggle_fire_policy")}`}
        pressed={hold}
        disabled={!any}
        onClick={p.onTogglePolicy}
      />
      <CommandButton
        glyph="▲"
        name="Deploy"
        keyHint={chip("toggle_deployment")}
        label={`Deploy ${key("toggle_deployment")}`}
        reach={deployers}
        of={p.selected.length}
        disabled={!deployers}
        onClick={() => p.onDeploy(true)}
      />
      <CommandButton
        glyph="▼"
        name="Pack"
        keyHint={chip("toggle_deployment")}
        label={`Pack ${key("toggle_deployment")}`}
        reach={deployers}
        of={p.selected.length}
        disabled={!deployers}
        onClick={() => p.onDeploy(false)}
      />
      <CommandButton
        glyph="⇄"
        name="Leave building"
        keyHint="—"
        label="Leave building"
        reach={inside}
        of={p.selected.length}
        disabled={!inside}
        onClick={p.onExit}
      />
    </div>
  );
}
