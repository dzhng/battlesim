/** One unit's info panel as drawn: NAME, then a row per WEAPON, then its
 *  STATES (`panelRows.ts` says what each holds). Own and enemy panels are the
 *  same markup; the owner's tone (own cyan, enemy red) comes from the
 *  callout round it. The battle's callouts and the panel workbench draw
 *  panels only through this. */
import { glyphIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";
import { weaponLabel, type Panel, type StateRow, type WeaponRow } from "./panelRows";

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
export const REASON_GLYPH: Record<string, string> = {
  firing: "✹",
  aiming: "◎",
  reloading: "↻",
  turret_traversing: "⟳",
  guiding: "⌖",
  tracking_last_sighting: "?",
  holding_fire: "⊖",
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

/** The panel's layout variants, for the workbench to set side by side:
 *  - `line`: a weapon's icon bare, a state's in a thin ring; counts right
 *    after the name; every timer a thin bar under the row's words.
 *  - `ring`: every icon in a ring, and the ring is the timer (a weapon's
 *    reload dashed outside, its aim on the ring; a state's timer or level).
 *  - `ledger`: icons bare; counts in a right-hand column; every timer five
 *    pips at the row's end; a hairline between sections. */
export const PANEL_VARIANTS = ["line", "ring", "ledger"] as const;
export type PanelVariant = (typeof PANEL_VARIANTS)[number];

/** How a variant draws a row's mark and its timers. */
const LOOK: Record<
  PanelVariant,
  { ring: "states" | "all" | "none"; timer: "bar" | "ring" | "pips" }
> = {
  line: { ring: "states", timer: "bar" },
  ring: { ring: "all", timer: "ring" },
  ledger: { ring: "none", timer: "pips" },
};

/** The zoom a panel is drawn for: far keeps the name and each row's mark
 *  and count on one line. */
export type PanelZoom = "default" | "far";

function arc(r: number, fraction: number): string {
  // A sliver still reads as "started".
  const f = Math.min(Math.max(fraction, 0.04), 0.9999);
  const a = f * Math.PI * 2 - Math.PI / 2;
  const [x, y] = [Math.cos(a) * r, Math.sin(a) * r];
  return `M 0 ${-r} A ${r} ${r} 0 ${f > 0.5 ? 1 : 0} 1 ${x.toFixed(2)} ${y.toFixed(2)}`;
}

/** A timer drawn the variant's way; `kind` names it for its colour and the
 *  scenes (`ro-aim`, `ro-reload`, `ro-progress`). */
interface Timer {
  kind: "aim" | "reload" | "progress";
  value: number;
}

const PIPS = 5;

/** The timers under the row (`bar`) or at its end (`pips`). */
function Timers({ timers, look }: { timers: Timer[]; look: "bar" | "pips" }) {
  if (!timers.length) return null;
  if (look === "bar")
    return (
      <span className="ro-bars" aria-hidden="true">
        {timers.map((t) => (
          <span key={t.kind} className={`ro-bar ro-${t.kind}`}>
            <span style={{ width: `${Math.max(4, t.value * 100).toFixed(1)}%` }} />
          </span>
        ))}
      </span>
    );
  return (
    <>
      {timers.map((t) => (
        <span key={t.kind} className={`ro-pips ro-${t.kind}`} aria-hidden="true">
          {Array.from({ length: PIPS }, (_, k) => (
            <i key={k} data-on={t.value * PIPS > k + 0.001 ? "" : undefined} />
          ))}
        </span>
      ))}
    </>
  );
}

/** A row's mark: its icon centred in a 16 px slot, in a ring when the
 *  variant rings it, with the ring's timers when the ring is the timer. */
function Mark({
  icon,
  ring,
  timers,
  className,
}: {
  icon: string | null;
  ring: boolean;
  timers: Timer[];
  className: string;
}) {
  return (
    <span className={`ro-mark ${className}`}>
      {ring && (
        <svg className="ro-ring" viewBox="-8 -8 16 16" aria-hidden="true">
          <circle r={6.5} className="ro-track" />
          {timers.map((t) => (
            <path
              key={t.kind}
              d={arc(t.kind === "reload" ? 7.6 : 6.5, t.value)}
              className={`ro-arc ro-${t.kind}`}
            />
          ))}
        </svg>
      )}
      {icon && <Icon path={icon} className="ro-icon ro-mark-icon" />}
    </span>
  );
}

/** A weapon's kinds and counts: "∞", "8", "AP 20 · HE 15" (the loaded kind
 *  bright), or on an enemy's just the kinds. */
function Kinds({ w }: { w: WeaponRow }) {
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

function WeaponRowView({ w, variant }: { w: WeaponRow; variant: PanelVariant }) {
  const look = LOOK[variant];
  const live = w.live;
  const timers: Timer[] = [];
  if (live?.reload != null) timers.push({ kind: "reload", value: live.reload });
  if (live?.aim != null) timers.push({ kind: "aim", value: live.aim });
  const counts = weaponLabel({ ...w, name: "" }).trim();
  return (
    <div
      className="ro-row ro-weapon"
      data-reason={live?.reason}
      data-aim={live ? (live.aim ?? "") : undefined}
      data-reload={live ? (live.reload ?? "") : undefined}
      data-ammo={live ? counts : undefined}
    >
      <Mark
        icon={w.icon}
        ring={look.ring === "all"}
        timers={look.timer === "ring" ? timers : []}
        className="ro-weapon-mark"
      />
      <span className="ro-word">{w.name}</span>
      <Kinds w={w} />
      {live?.guiding && <span className="ro-guide">⌖</span>}
      {live?.blocked && (
        <span className="ro-badge" title={REASON_TEXT[live.reason] ?? live.reason}>
          {REASON_GLYPH[live.reason] ?? "!"}
        </span>
      )}
      {look.timer !== "ring" && <Timers timers={timers} look={look.timer} />}
    </div>
  );
}

function StateRowView({ row, variant }: { row: StateRow; variant: PanelVariant }) {
  const look = LOOK[variant];
  const timers: Timer[] =
    row.progress !== null && row.progress > 0 ? [{ kind: "progress", value: row.progress }] : [];
  return (
    <div
      className="ro-row ro-state"
      data-state={row.state}
      data-warn={row.warn ? "" : undefined}
      data-progress={row.progress ?? ""}
    >
      <Mark
        icon={row.icon}
        ring={look.ring !== "none"}
        timers={look.timer === "ring" ? timers : []}
        className="ro-state-mark"
      />
      <span className="ro-word ro-state-word">{row.word}</span>
      {look.timer !== "ring" && <Timers timers={timers} look={look.timer} />}
    </div>
  );
}

/** One panel: NAME, WEAPONS, STATES, each section only when it has rows. */
export function InfoPanel({
  panel,
  variant = "line",
  zoom = "default",
}: {
  panel: Panel;
  variant?: PanelVariant;
  zoom?: PanelZoom;
}) {
  return (
    <div className="ro-body" data-variant={variant} data-zoom={zoom}>
      <span className="ro-name">
        {panel.mark && <Icon path={panel.mark} className="ro-icon ro-name-icon" />}
        {panel.name}
      </span>
      {panel.weapons.length > 0 && (
        <div className="ro-section ro-weapons">
          {panel.weapons.map((w) => (
            <WeaponRowView key={w.key} w={w} variant={variant} />
          ))}
        </div>
      )}
      {panel.states.length > 0 && (
        <div className="ro-section ro-states">
          {panel.states.map((r) => (
            <StateRowView key={r.state} row={r} variant={variant} />
          ))}
        </div>
      )}
    </div>
  );
}
