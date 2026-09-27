/** Player readouts of own units' readiness (U02, U03), from observation only:
 *  - world-anchored rings above units: per weapon, an aim ring and a reload
 *    ring around the rounds left (∞ when unlimited); completed timers vanish;
 *    a guidance icon while guiding; a separate deployment square;
 *  - the selected-unit panel, which keeps every detail at any zoom;
 *  - the command bar, exposing every village action and the fire policy.
 *  Enemies never get readouts: only own units carry readiness. */
import { useCallback, useImperativeHandle, useRef, type Ref } from "react";
import village from "@fixtures/village.json";
import type { MountView, ObservationView, OwnUnitView } from "../sim/observation";
import type { CommandMode } from "../input/useUnitControl";
import { CommandBindings } from "../input/commandBindings";

type Project = (x: number, y: number, z: number) => [number, number] | null;
type Point3 = readonly [number, number, number];

const MOUNTS = village.mounts as Record<string, { name: string; weapons: string[] }[]>;
const SOLDIER_HP = village.health.soldier;
/** Full squad sizes and vehicle hit points, against which strength reads. */
const SQUAD_SIZE: Record<string, number> = {
  rifle: village.health.rifle_squad_size,
  recon: village.health.recon_squad_size,
  at: village.health.at_squad_size,
};
const VEHICLE_HP: Record<string, number> = {
  tank: village.health.tank,
  supply: village.health.supply,
  jeep: village.health.jeep,
};
/** Above this camera distance, rings show only for selected units. */
export const RINGS_FAR_M = 700;

/** Short captions under each ring, naming the weapon. */
const MOUNT_CAPTION: Record<string, string> = {
  cannon: "CANNON",
  HMG: "HMG",
  rifles: "RIFLES",
  "grenade launcher": "GREN",
  "ATGM launcher": "ATGM",
};
/** Reasons the ring itself shows as a badge: anything that is not plain progress. */
const QUIET = new Set(["firing", "aiming", "reloading", "guiding"]);

/** Short labels for weapon rows, as the rings print them. */
const KIND_LABEL: Record<string, string> = { tank_ap: "AP", tank_he: "HE" };

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

/** The name a unit goes by in the panel, the log and on the map. */
export function unitName(u: Pick<OwnUnitView, "kind" | "id">): string {
  return `${u.kind} #${u.id}`;
}

/** Strength in [0, 1]: a vehicle's hit points, or a squad's soldiers' health
 *  against the full squad, so losses show as well as wounds. */
export function unitStrength(u: OwnUnitView): number {
  if (u.members.length === 0) return u.hp / (VEHICLE_HP[u.kind] ?? u.hp);
  const full = (SQUAD_SIZE[u.kind] ?? u.memberHp.length) * SOLDIER_HP;
  return u.memberHp.reduce((a, b) => a + b, 0) / (full || 1);
}

/** The garrison phase, with the timer while entering or leaving. */
export function garrisonText(u: OwnUnitView): string {
  const g = u.garrison;
  if (!g) return "outside";
  const timer = g.phase === "entering" || g.phase === "exiting";
  return `${GARRISON_PHASE_TEXT[g.phase] ?? g.phase}${timer ? ` ${(g.progress * 100).toFixed(0)}%` : ""}`;
}

export function weaponName(unit: OwnUnitView, mount: MountView): string {
  return MOUNTS[unit.kind]?.[mount.mount]?.name ?? `weapon ${mount.mount + 1}`;
}

/** The rounds shown inside a mount's ring: the loaded (or next) kind's count. */
export function ringAmmo(unit: OwnUnitView, mount: MountView): string {
  const kinds = MOUNTS[unit.kind]?.[mount.mount]?.weapons ?? [];
  const k = mount.loaded ?? mount.reloading ?? mount.ammo.findIndex((n) => n === null || n > 0);
  const n = mount.ammo[Math.max(0, k)];
  const count = n === null ? "∞" : String(n ?? 0);
  const label = kinds.length > 1 ? (KIND_LABEL[kinds[Math.max(0, k)]] ?? "") : "";
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

function MountRing({ unit, mount }: { unit: OwnUnitView; mount: MountView }) {
  const { aim, reload } = ringTimers(mount);
  return (
    <div className="ro-mount">
      <svg
        className="ro-ring"
        viewBox="-17 -17 34 34"
        data-reason={mount.reason}
        data-aim={aim ?? ""}
        data-reload={reload ?? ""}
      >
        <circle r={15} className="ro-track" />
        {reload !== null && <path d={arc(15, reload)} className="ro-reload" />}
        {aim !== null && <path d={arc(11, aim)} className="ro-aim" />}
        <text className="ro-ammo" y={4}>
          {ringAmmo(unit, mount)}
        </text>
        {mount.guiding && (
          <g className="ro-guide">
            <circle cx={12} cy={-12} r={6} />
            <text x={12} y={-8.5}>
              ⌖
            </text>
          </g>
        )}
        {!QUIET.has(mount.reason) && (
          <g className="ro-badge">
            <circle cx={12} cy={12} r={6} />
            <text x={12} y={15.5}>
              {REASON_GLYPH[mount.reason] ?? "!"}
            </text>
          </g>
        )}
      </svg>
      <span className="ro-caption">{MOUNT_CAPTION[weaponName(unit, mount)] ?? ""}</span>
    </div>
  );
}

function DeploymentRing({ unit }: { unit: OwnUnitView }) {
  const d = unit.deployment!;
  const done = d.progress >= 1;
  return (
    <div className="ro-mount">
      <svg className="ro-ring ro-deploy" viewBox="-17 -17 34 34" data-deploy={d.progress}>
        <rect x={-14} y={-14} width={28} height={28} rx={4} className="ro-track" />
        {!done && d.progress > 0 && <path d={arc(12, d.progress)} className="ro-deploy-arc" />}
        <text className="ro-ammo" y={4}>
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
/** How far a destination name drops to sit below its ring instead of above. */
const GOAL_BELOW_PX = 20;

export interface ReadoutLayerHandle {
  /** Re-anchor every cluster and name tag; call once per animation frame.
   *  `positions` are the drawn (interpolated) unit positions, so rings move
   *  with meshes. */
  place(project: Project, distance: number, positions?: ReadonlyMap<number, Point3>): void;
}

/** Ring clusters for own units, and for each selected unit its name, above
 *  it and at its destination ring (so two routes starting close together
 *  still read apart), positioned by the viewport each frame. Nothing is
 *  placed under an element marked `data-occludes-readouts` (a panel): it
 *  slides clear to the right. Nothing overprints: a cluster that would cover
 *  another rises above it, and a destination name moves below its ring and
 *  then further down. */
export function ReadoutLayer({
  observation,
  selected,
  handle,
  groundZ,
}: {
  observation: ObservationView | null;
  selected: readonly number[];
  handle: Ref<ReadoutLayerHandle>;
  /** Height of the ground a destination ring lies on. */
  groundZ: (x: number, y: number) => number;
}) {
  const nodes = useRef(new Map<number, HTMLDivElement>());
  const goals = useRef(new Map<number, HTMLDivElement>());
  const units = useRef<OwnUnitView[]>([]);
  units.current = observation?.own ?? [];
  const selectedRef = useRef(selected);
  selectedRef.current = selected;
  const groundRef = useRef(groundZ);
  groundRef.current = groundZ;
  useImperativeHandle(handle, () => ({
    place(project, distance, positions) {
      // Where each cluster and tag is anchored: its bottom centre, in page pixels.
      const anchored: { node: HTMLDivElement; x: number; y: number; goal: boolean }[] = [];
      for (const u of units.current) {
        const picked = selectedRef.current.includes(u.id);
        const node = nodes.current.get(u.id);
        if (node) {
          // Zoomed out, rings stay only for the selection; the panel keeps all.
          const shown = distance < RINGS_FAR_M || picked;
          const p = positions?.get(u.id) ?? u.position;
          const at = shown ? project(p[0], p[1], p[2] + 3) : null;
          node.style.display = at ? "flex" : "none";
          // A fixed screen gap above the unit, so the cluster never sits on it.
          if (at) anchored.push({ node, x: at[0], y: at[1] - 22, goal: false });
        }
        const tag = goals.current.get(u.id);
        if (tag) {
          const g = u.goal;
          const at = picked && g ? project(g[0], g[1], groundRef.current(g[0], g[1])) : null;
          tag.style.display = at ? "block" : "none";
          // Just above the destination ring's far edge.
          if (at) anchored.push({ node: tag, x: at[0], y: at[1] - 10, goal: true });
        }
      }
      // One layout read for the frame: the panels to keep clear of, then sizes.
      const panels = [...document.querySelectorAll("[data-occludes-readouts]")].map((e) =>
        e.getBoundingClientRect(),
      );
      const boxes = anchored.map((a) => ({
        ...a,
        w: a.node.offsetWidth,
        h: a.node.offsetHeight,
      }));
      // Never under a panel: slide right of any panel it would sit beneath.
      const clear = (box: Box): Box => {
        for (const r of panels) {
          if (overlaps({ x0: r.left, x1: r.right, y0: r.top, y1: r.bottom }, box)) {
            const dx = r.right + TAG_GAP_PX - box.x0;
            box = { ...box, x0: box.x0 + dx, x1: box.x1 + dx };
          }
        }
        return box;
      };
      const placed: Box[] = [];
      const hit = (box: Box) => placed.find((o) => overlaps(o, box));
      const shift = (box: Box, dy: number): Box => ({ ...box, y0: box.y0 + dy, y1: box.y1 + dy });
      // Clusters, lowest first: one that would cover another rises above it.
      for (const b of boxes.filter((b) => !b.goal).sort((m, n) => n.y - m.y)) {
        let box = clear({ x0: b.x - b.w / 2, x1: b.x + b.w / 2, y0: b.y - b.h, y1: b.y });
        for (let o = hit(box); o; o = hit(box)) box = shift(box, o.y0 - TAG_GAP_PX - box.y1);
        box = clear(box);
        placed.push(box);
        b.node.style.transform = `translate(${box.x0}px, ${box.y0}px)`;
      }
      // Destination names: above the ring, else just below it, else stacked
      // further down, so no name covers another or a cluster.
      for (const b of boxes.filter((b) => b.goal).sort((m, n) => m.y - n.y)) {
        let box = clear({ x0: b.x - b.w / 2, x1: b.x + b.w / 2, y0: b.y - b.h, y1: b.y });
        if (hit(box)) box = shift(box, b.h + GOAL_BELOW_PX);
        for (let o = hit(box); o; o = hit(box)) box = shift(box, o.y1 + TAG_GAP_PX - box.y0);
        box = clear(box);
        placed.push(box);
        b.node.style.transform = `translate(${box.x0}px, ${box.y0}px)`;
      }
    },
  }));
  const bind = useCallback(
    (map: Map<number, HTMLDivElement>, id: number) => (el: HTMLDivElement | null) => {
      if (el) map.set(id, el);
      else map.delete(id);
    },
    [],
  );
  return (
    <div className="ro-layer" data-testid="readouts">
      {(observation?.own ?? []).map((u) => {
        const picked = selected.includes(u.id);
        const setup = !!u.deployment && u.deployment.progress > 0 && u.deployment.progress < 1;
        return u.mounts.length || setup || picked ? (
          <div
            key={u.id}
            ref={bind(nodes.current, u.id)}
            className={`ro-unit${picked ? " ro-selected" : ""}`}
            data-unit={u.id}
          >
            {picked && <span className="ro-name">{unitName(u)}</span>}
            {u.mounts.map((m) => (
              <MountRing key={m.mount} unit={u} mount={m} />
            ))}
            {setup && <DeploymentRing unit={u} />}
          </div>
        ) : null;
      })}
      {(observation?.own ?? [])
        .filter((u) => selected.includes(u.id) && u.goal)
        .map((u) => (
          <div
            key={`goal-${u.id}`}
            ref={bind(goals.current, u.id)}
            className="ro-goal"
            data-goal={u.id}
          >
            {unitName(u)}
          </div>
        ))}
    </div>
  );
}

/** Every detail for the selected units, at any zoom: policy, set-up,
 *  strength and pinning, building and supply state, and each weapon. */
export function SelectionPanel({ units }: { units: readonly OwnUnitView[] }) {
  if (units.length === 0) return <div className="lab-hint">No unit selected</div>;
  return (
    <div className="ro-panel" data-testid="selection-panel">
      {units.map((u) => (
        <div key={u.id} className="ro-panel-unit" data-unit={u.id}>
          <div>
            <strong>{unitName(u)}</strong> ·{" "}
            {u.engagement === "fire_at_will" ? "fire at will" : "return fire only"}
            {u.deployment && ` · ${deploymentText(u)}`}
          </div>
          <UnitCondition unit={u} />
          {u.mounts.map((m) => (
            <div key={m.mount} className="ro-panel-mount" data-reason={m.reason}>
              <span className="ro-glyph">{REASON_GLYPH[m.reason] ?? "·"}</span>
              <span>
                {weaponName(u, m)}: {REASON_TEXT[m.reason] ?? m.reason}
                {m.guiding && m.reason !== "guiding" && " · guiding a missile"} · {ammoText(u, m)}
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

function ammoText(u: OwnUnitView, m: MountView): string {
  const kinds = MOUNTS[u.kind]?.[m.mount]?.weapons ?? [];
  return m.ammo
    .map((n, k) => {
      const label = kinds.length > 1 ? `${KIND_LABEL[kinds[k]] ?? kinds[k]} ` : "";
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

/** Every village action and the fire policy; keys are optional shortcuts,
 *  named from the one binding table. */
export function CommandBar(p: CommandBarProps) {
  const key = (command: keyof typeof CommandBindings) => `(${CommandBindings[command].label})`;
  const any = p.selected.length > 0;
  const trucks = p.selected.some((u) => u.deployment);
  const inside = p.selected.some((u) => u.garrison);
  const infantry = p.selected.some((u) => u.members.length > 0);
  const hold = any && p.selected.every((u) => u.engagement === "return_fire_only");
  const modeButton = (mode: CommandMode, label: string) => (
    <button
      type="button"
      aria-pressed={p.mode === mode}
      onClick={() => p.setMode(mode)}
      disabled={!any}
    >
      {label}
    </button>
  );
  return (
    <div className="lab-row ro-commands" role="toolbar" aria-label="Commands">
      {modeButton("move", "Move (right-click)")}
      {modeButton("attack_move", `Attack-move ${key("attack_move")}`)}
      {modeButton("reverse_move", `Reverse ${key("reverse_move")}`)}
      {modeButton("attack_ground", `Attack ground ${key("attack_ground")}`)}
      {modeButton("fast_move", "Fast move (double right-click)")}
      <button
        type="button"
        aria-pressed={p.mode === "garrison"}
        onClick={() => p.setMode("garrison")}
        disabled={!infantry}
      >
        Garrison (right-click a building)
      </button>
      <button type="button" onClick={p.onStop} disabled={!any}>
        Stop {key("stop")}
      </button>
      <button type="button" aria-pressed={hold} onClick={p.onTogglePolicy} disabled={!any}>
        {hold ? "Return fire only" : "Fire at will"} {key("toggle_fire_policy")}
      </button>
      <button type="button" onClick={() => p.onDeploy(true)} disabled={!trucks}>
        Deploy {key("toggle_deployment")}
      </button>
      <button type="button" onClick={() => p.onDeploy(false)} disabled={!trucks}>
        Pack {key("toggle_deployment")}
      </button>
      <button type="button" onClick={p.onExit} disabled={!inside}>
        Leave building
      </button>
    </div>
  );
}
