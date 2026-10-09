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
import type { ReactNode, Ref } from "react";
import { Icon } from "./icons";
import {
  PIPS,
  pipsLit,
  weaponCounts,
  type Panel,
  type StateRow,
  type WeaponRow,
} from "./panelRows";

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

/** Floating panels use compressed or default detail on their layer.
 *  Cards and specimens can compact distinct icons with the far mode. */
export type PanelZoom = "default" | "far" | "compressed";

/** The activity currently represented by the single progress ring. */
interface Timer {
  kind: "aim" | "reload" | "cooldown" | "progress";
  value: number;
}

const RING_R = 9;

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

/** One progress ring round the icon, whatever activity it represents. */
function Mark({
  icon,
  timer,
  className,
}: {
  icon: string | null;
  timer: Timer | null;
  className: string;
}) {
  return (
    <span className={`ro-mark ${className}`}>
      {timer && (
        <svg className={`ro-ring ro-${timer.kind}`} viewBox="-11 -11 22 22" aria-hidden="true">
          <circle r={RING_R} className="ro-track" />
          <path d={arc(RING_R, timer.value)} className="ro-arc" />
        </svg>
      )}
      {icon && <Icon path={icon} className="ro-mark-icon" />}
    </span>
  );
}

/** A weapon's kinds and counts: "∞" (drawn), "8", "AP 20 · HE 15" (the
 *  loaded kind bright), or on an enemy's just the kinds. */
function WeaponCounts({ w }: { w: Pick<WeaponRow, "kinds"> }) {
  if (w.kinds.length === 1 && w.kinds[0].label === null && w.kinds[0].count === undefined)
    return null;
  return (
    <span className="ro-kinds">
      <span className="ro-count-separator" aria-hidden="true">
        :
      </span>
      {w.kinds.map((k, i) => (
        <span key={i} className="ro-kind" data-loaded={k.loaded ? "" : undefined}>
          {i > 0 && <span className="ro-sep">·</span>}
          {k.label && <span className="ro-kind-label">{k.label}</span>}
          {k.count === null ? (
            <span className="ro-ammo ro-unlimited" title="unlimited">
              <Icon path={glyphIcon("unlimited")} />
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
  const timer: Timer | null =
    live?.cooldown != null
      ? { kind: "cooldown", value: live.cooldown }
      : live?.aim != null
        ? { kind: "aim", value: live.aim }
        : live?.reload != null
          ? { kind: "reload", value: live.reload }
          : null;
  const mark = live ? REASON_MARK[live.reason] : null;
  return (
    <div
      className="ro-row ro-weapon"
      data-reason={live?.reason}
      data-aim={live ? (live.aim ?? "") : undefined}
      data-reload={live ? (live.reload ?? "") : undefined}
      data-cooldown={live ? (live.cooldown ?? "") : undefined}
      data-ammo={live ? weaponCounts(w) : undefined}
    >
      <Mark icon={w.icon} timer={timer} className="ro-weapon-mark" />
      <span className="ro-word">{w.name}</span>
      {live?.guiding && <Icon path={hudIcon("guiding")} className="ro-guide" />}
      {mark && <Icon path={mark} className="ro-badge" title={live?.reason.replaceAll("_", " ")} />}
      <WeaponCounts w={w} />
      <Pips fill={w.fill} reserve={!!live} />
    </div>
  );
}

/** The compact card's one active progress track. When several things are
 * waiting, the largest fraction finishes first, so the bar moves straight to
 * the next item as each one completes. */
function loadingProgress(panel: Panel): number | null {
  const values = [
    ...panel.states
      .filter((row) => row.state === "deploying" || row.state === "packing")
      .map((row) => row.progress),
    ...panel.weapons.flatMap((weapon) => {
      if (!weapon.live) return [];
      if (weapon.live.reason === "aiming") return [weapon.live.aim];
      if (weapon.live.reason === "reloading") return [weapon.live.reload];
      return [];
    }),
  ].filter((value): value is number => value !== null && value !== undefined);
  return values.length ? Math.max(...values) : null;
}

function StateRowView({ row }: { row: StateRow }) {
  const timer: Timer | null =
    row.progress !== null && row.progress > 0 ? { kind: "progress", value: row.progress } : null;
  return (
    <div
      className="ro-row ro-state"
      data-state={row.state}
      data-tone={row.tone ?? undefined}
      data-progress={row.progress ?? ""}
    >
      <Mark icon={row.icon} timer={timer} className="ro-state-mark" />
      <span className="ro-word ro-state-word">{row.word}</span>
      <Pips fill={row.fill} />
    </div>
  );
}

/** One panel: NAME (with its strength's pips), WEAPONS, STATES, each
 *  section only when it has rows.
 *  `zoom` is the workbench's; the battle sets it on the layer round it. */
export function InfoPanel({ panel, zoom }: { panel: Panel; zoom?: PanelZoom }) {
  const loading = zoom === "compressed" ? loadingProgress(panel) : null;
  const statusIcons = [
    panel.weapons.some((weapon) => weapon.kinds.some((kind) => kind.count === 0))
      ? { path: hudIcon("no_ammo"), title: "Out of ammo", negative: true }
      : null,
    panel.states.some((row) => row.state === "pinned")
      ? { path: stateIcon("pinned"), title: "Pinned", negative: true }
      : null,
    panel.states.some((row) => row.state === "hidden")
      ? { path: stateIcon("hidden"), title: "Hidden", negative: false }
      : null,
  ].filter((icon): icon is { path: string; title: string; negative: boolean } => icon !== null);
  return (
    <div className="ro-body" data-zoom={zoom}>
      <span className="ro-name">
        {panel.mark && <Icon path={panel.mark} className="ro-name-icon" />}
        <span className="ro-name-word">{panel.name}</span>
        {statusIcons.length > 0 && (
          <span className="ro-status-icons" aria-label="unit status">
            {statusIcons.map((icon, index) => (
              <Icon
                key={`${icon.path}-${index}`}
                path={icon.path}
                className={`ro-status-icon${icon.negative ? " ro-status-negative" : ""}`}
                title={icon.title}
              />
            ))}
          </span>
        )}
        <Pips fill={panel.strength} />
      </span>
      {panel.weapons.length > 0 && (
        <div
          className="ro-section ro-weapons"
          data-repeat-icons={
            new Set(panel.weapons.map((w) => w.icon)).size < panel.weapons.length || undefined
          }
        >
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
      {loading !== null && (
        <div
          className="ro-loading"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={1}
          aria-valuenow={loading}
        >
          <span style={{ width: `${Math.round(loading * 100)}%` }} />
        </div>
      )}
    </div>
  );
}

/** Who a panel belongs to: its tone (own cyan, enemy and contact red)
 *  follows. */
export type PanelOwner = "own" | "enemy" | "contact";

/** The callout round a panel, in its owner's tone, brighter when selected:
 *  the battle's callouts and the panel workbench frame panels only through
 *  this. */
export function PanelCallout({
  owner,
  selected,
  children,
  ...rest
}: {
  owner: PanelOwner;
  selected: boolean;
  children: ReactNode;
  ref?: Ref<HTMLDivElement>;
} & Record<`data-${string}`, string | number | undefined>) {
  return (
    <div
      className={`ro-unit ro-${owner}${selected ? " ro-selected" : ""}`}
      data-owner={owner}
      {...rest}
    >
      {children}
    </div>
  );
}
