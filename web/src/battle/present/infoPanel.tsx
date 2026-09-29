/** One unit's info panel as drawn: NAME (an own unit's with its strength's
 *  pips), then a row per WEAPON, then its STATES (`panelRows.ts` says what
 *  each holds). Own and enemy panels are the same markup; the owner's tone
 *  (own cyan, enemy red) comes from the callout or card round it. Each row is its icon, bare and centred in a 16 px slot,
 *  its words, then a right-hand column of counts. A running timer is a ring
 *  filling round the icon (only while it runs); an amount counted against a
 *  full one is five pips at the row's end. The battle's callouts, the unit
 *  card and the panel workbench draw panels only through this. Every item of a row
 *  (icon, words, counts, marks, pips) sits on the row's one centre line. */
import { glyphIcon, hudIcon, stateIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";
import {
  PIPS,
  pipsLit,
  weaponCounts,
  type Panel,
  type StateRow,
  type WeaponRow,
} from "./panelRows";

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

/** The mark a panel's weapon row carries for its reason, in the warning
 *  colour: why the weapon can't fire (a generated icon). Null says nothing:
 *  plain progress (its pips say it), guiding (its own mark), no target it
 *  can hurt (every idle weapon's reason), out of range (the player's to see,
 *  and the ruler's), and no facing slot (a garrison always has someone
 *  facing; not the player's to act on). One line per reason. */
export const REASON_MARK: Record<string, string | null> = {
  firing: null,
  aiming: null,
  reloading: null,
  guiding: null,
  no_compatible_target: null,
  holding_fire: hudIcon("hold_fire"),
  out_of_range: null,
  blocked_trajectory: hudIcon("blocked_shot"),
  friendly_in_line: hudIcon("friendly_in_line"),
  turret_traversing: hudIcon("turret"),
  moving_stationary_weapon: hudIcon("must_stop"),
  out_of_ammo: hudIcon("no_ammo"),
  tracking_last_sighting: stateIcon("last_seen"),
  no_own_sight: hudIcon("no_sight"),
  no_facing_slot: null,
  changing_position: stateIcon("building"),
};

/** The zoom a panel is drawn for: far keeps the name and each row's icon
 *  and count on one line. The battle sets it on the callout layer. */
export type PanelZoom = "default" | "far";

/** A running timer; `kind` names it for its form and the scenes
 *  (`ro-aim`, `ro-reload`, `ro-progress`). */
interface Timer {
  kind: "aim" | "reload" | "progress";
  value: number;
}

/** Ring radii round a 16 px icon slot: a reload's dashed ring outside, an
 *  aim's solid one just inside it; any other timer on the aim's. */
const RING_R = { aim: 7.4, progress: 7.4, reload: 9 } as const;

function arc(r: number, fraction: number): string {
  // A sliver still reads as "started".
  const f = Math.min(Math.max(fraction, 0.04), 0.9999);
  const a = f * Math.PI * 2 - Math.PI / 2;
  const [x, y] = [Math.cos(a) * r, Math.sin(a) * r];
  return `M 0 ${-r} A ${r} ${r} 0 ${f > 0.5 ? 1 : 0} 1 ${x.toFixed(2)} ${y.toFixed(2)}`;
}

/** An amount as five pips, the lit ones its share of the full amount. With
 *  no amount, `reserve` keeps their room empty, so an unlimited weapon's ∞
 *  stands in the same count column as its neighbours' numbers. */
function Pips({ fill, reserve = false }: { fill: number | null; reserve?: boolean }) {
  if (fill === null)
    return reserve ? <span className="ro-pips ro-pips-none" aria-hidden="true" /> : null;
  const lit = pipsLit(fill);
  return (
    <span className="ro-pips" data-lit={lit} aria-hidden="true">
      {Array.from({ length: PIPS }, (_, k) => (
        <i key={k} data-on={k < lit ? "" : undefined} />
      ))}
    </span>
  );
}

/** A row's icon, centred in its 16 px slot, and round it a ring filling
 *  for each timer while one runs (none otherwise). */
function Mark({
  icon,
  timers,
  className,
}: {
  icon: string | null;
  timers: Timer[];
  className: string;
}) {
  return (
    <span className={`ro-mark ${className}`}>
      {timers.length > 0 && (
        <svg className="ro-ring" viewBox="-11 -11 22 22" aria-hidden="true">
          {timers.map((t) => (
            <g key={t.kind} className={`ro-timer ro-${t.kind}`}>
              <circle r={RING_R[t.kind]} className="ro-track" />
              <path d={arc(RING_R[t.kind], t.value)} className="ro-arc" />
            </g>
          ))}
        </svg>
      )}
      {icon && <Icon path={icon} className="ro-icon ro-mark-icon" />}
    </span>
  );
}

/** A weapon's kinds and counts: "∞" (drawn), "8", "AP 20 · HE 15" (the
 *  loaded kind bright), or on an enemy's just the kinds. The unit card shows
 *  counts with it too. */
export function WeaponCounts({ w }: { w: Pick<WeaponRow, "kinds"> }) {
  if (w.kinds.length === 1 && w.kinds[0].label === null && w.kinds[0].count === undefined)
    return null;
  return (
    <span className="ro-kinds">
      {w.kinds.map((k, i) => (
        <span key={i} className="ro-kind" data-loaded={k.loaded ? "" : undefined}>
          {i > 0 && <span className="ro-sep">·</span>}
          {k.label && <span className="ro-kind-label">{k.label}</span>}
          {k.count === null ? (
            <span className="ro-ammo ro-unlimited" title="unlimited">
              <Icon path={glyphIcon("unlimited")} className="ro-icon" />
            </span>
          ) : k.count !== undefined ? (
            <span className="ro-ammo">{k.count}</span>
          ) : null}
        </span>
      ))}
    </span>
  );
}

function WeaponRowView({ w }: { w: WeaponRow }) {
  const live = w.live;
  const timers: Timer[] = [];
  if (live?.reload != null) timers.push({ kind: "reload", value: live.reload });
  if (live?.aim != null) timers.push({ kind: "aim", value: live.aim });
  const mark = live ? REASON_MARK[live.reason] : null;
  return (
    <div
      className="ro-row ro-weapon"
      data-reason={live?.reason}
      data-aim={live ? (live.aim ?? "") : undefined}
      data-reload={live ? (live.reload ?? "") : undefined}
      data-ammo={live ? weaponCounts(w) : undefined}
    >
      <Mark icon={w.icon} timers={timers} className="ro-weapon-mark" />
      <span className="ro-word">{w.name}</span>
      {live?.guiding && <Icon path={hudIcon("guiding")} className="ro-icon ro-guide" />}
      {mark && <Icon path={mark} className="ro-icon ro-badge" />}
      <WeaponCounts w={w} />
      <Pips fill={w.fill} reserve={!!live} />
    </div>
  );
}

function StateRowView({ row }: { row: StateRow }) {
  const timers: Timer[] =
    row.progress !== null && row.progress > 0 ? [{ kind: "progress", value: row.progress }] : [];
  return (
    <div
      className="ro-row ro-state"
      data-state={row.state}
      data-tone={row.tone ?? undefined}
      data-progress={row.progress ?? ""}
    >
      <Mark icon={row.icon} timers={timers} className="ro-state-mark" />
      <span className="ro-word ro-state-word">{row.word}</span>
      <Pips fill={row.fill} />
    </div>
  );
}

/** One panel: NAME (with its strength's pips), WEAPONS, STATES, each
 *  section only when it has rows.
 *  `zoom` is the workbench's; the battle sets it on the layer round it. */
export function InfoPanel({ panel, zoom }: { panel: Panel; zoom?: PanelZoom }) {
  return (
    <div className="ro-body" data-zoom={zoom}>
      <span className="ro-name">
        {panel.mark && <Icon path={panel.mark} className="ro-icon ro-name-icon" />}
        <span className="ro-name-word">{panel.name}</span>
        <Pips fill={panel.strength} />
      </span>
      {panel.weapons.length > 0 && (
        <div className="ro-section ro-weapons">
          {panel.weapons.map((w) => (
            <WeaponRowView key={w.key} w={w} />
          ))}
        </div>
      )}
      {panel.states.length > 0 && (
        <div className="ro-section ro-states">
          {panel.states.map((r) => (
            <StateRowView key={r.state} row={r} />
          ))}
        </div>
      )}
    </div>
  );
}
