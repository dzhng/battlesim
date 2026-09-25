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

/** Every supply service state in player words. */
export const SERVICE_TEXT: Record<string, string> = {
  out_of_range: "no supply vehicle in reach",
  source_not_deployed: "supply vehicle not set up yet",
  moving: "waiting: must stand still",
  firing: "waiting: fired this moment",
  serving: "being served",
  no_stock: "waiting: the truck cannot pay for the next item",
  full: "nothing missing",
  garrisoned: "in a building: no replacements",
};

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

export interface ReadoutLayerHandle {
  /** Re-anchor every cluster; call once per animation frame. `positions`
   *  are the drawn (interpolated) unit positions, so rings move with meshes. */
  place(project: Project, distance: number, positions?: ReadonlyMap<number, Point3>): void;
}

/** Ring clusters for own units, positioned by the viewport each frame. */
export function ReadoutLayer({
  observation,
  selected,
  handle,
}: {
  observation: ObservationView | null;
  selected: readonly number[];
  handle: Ref<ReadoutLayerHandle>;
}) {
  const nodes = useRef(new Map<number, HTMLDivElement>());
  const units = useRef<OwnUnitView[]>([]);
  units.current = observation?.own ?? [];
  const selectedRef = useRef(selected);
  selectedRef.current = selected;
  useImperativeHandle(handle, () => ({
    place(project, distance, positions) {
      for (const u of units.current) {
        const node = nodes.current.get(u.id);
        if (!node) continue;
        // Zoomed out, rings stay only for the selection; the panel keeps all.
        const shown = distance < RINGS_FAR_M || selectedRef.current.includes(u.id);
        const p = positions?.get(u.id) ?? u.position;
        const at = shown ? project(p[0], p[1], p[2] + 3) : null;
        node.style.display = at ? "flex" : "none";
        // A fixed screen gap above the unit, so the cluster never sits on it.
        if (at)
          node.style.transform = `translate(${at[0]}px, ${at[1] - 22}px) translate(-50%, -100%)`;
      }
    },
  }));
  const bind = useCallback(
    (id: number) => (el: HTMLDivElement | null) => {
      if (el) nodes.current.set(id, el);
      else nodes.current.delete(id);
    },
    [],
  );
  return (
    <div className="ro-layer" data-testid="readouts">
      {(observation?.own ?? []).map((u) =>
        u.mounts.length ||
        (u.deployment && u.deployment.progress > 0 && u.deployment.progress < 1) ? (
          <div
            key={u.id}
            ref={bind(u.id)}
            className={`ro-unit${selected.includes(u.id) ? " ro-selected" : ""}`}
            data-unit={u.id}
          >
            {u.mounts.map((m) => (
              <MountRing key={m.mount} unit={u} mount={m} />
            ))}
            {u.deployment && u.deployment.progress > 0 && u.deployment.progress < 1 && (
              <DeploymentRing unit={u} />
            )}
          </div>
        ) : null,
      )}
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
            `supply: ${SERVICE_TEXT[u.service] ?? u.service}`}
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

/** Every village action and the fire policy; keys are optional shortcuts. */
export function CommandBar(p: CommandBarProps) {
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
      {modeButton("attack_move", "Attack-move (A)")}
      {modeButton("attack_ground", "Attack ground (G)")}
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
        Stop (S)
      </button>
      <button type="button" aria-pressed={hold} onClick={p.onTogglePolicy} disabled={!any}>
        {hold ? "Return fire only (E)" : "Fire at will (E)"}
      </button>
      <button type="button" onClick={() => p.onDeploy(true)} disabled={!trucks}>
        Deploy
      </button>
      <button type="button" onClick={() => p.onDeploy(false)} disabled={!trucks}>
        Pack
      </button>
      <button type="button" onClick={p.onExit} disabled={!inside}>
        Leave building
      </button>
    </div>
  );
}
