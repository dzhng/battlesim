/** Player readouts, from observation only (U02, U03):
 *  - an info panel off every unit on a leader line (`infoPanel.tsx`): its
 *    name, a row per weapon, then its states (`panelRows.ts`); an own
 *    unit's in cyan, with rounds left and running timers; an enemy's or a
 *    contact's in the enemy red, only what the side knows of it;
 *  - the unit card: the selection's panels, the same component, at any zoom;
 *  - the command bar: the selection's commands and its fire policy. */
import { useCallback, useImperativeHandle, useRef, type ReactNode, type Ref } from "react";
import type { ContactView, IdentifiedView, OwnUnitView } from "../sim/observation";
import type { CommandMode } from "../input/useUnitControl";
import { CommandBindings, FacingBinding } from "../input/commandBindings";
import { reach, type ReachCommand } from "../input/commandReach";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { hudIcon, stateIcon, unitIcons } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";
import { villageHud } from "./hudTheme";
import { InfoPanel } from "./infoPanel";
import { contactPanel, enemyPanel, ownPanel, type PanelRules } from "./panelRows";

export type Project = (x: number, y: number, z: number) => [number, number] | null;
type Point3 = readonly [number, number, number];

/** The rule blocks the readouts read (the scenario's): each weapon row's
 *  display name and icon, and what the state rows read (`PanelRules`). A
 *  unit's type (its name, mounts, full strength) is the unit catalog's. */
export type ReadoutRules = PanelRules;

/** Above this camera distance, panels show only for selected units (every
 *  other own panel, and every enemy's and contact's, hides), each in its
 *  compact far form: the name over one line of icons and counts. */
export const PANELS_FAR_M = 700;

/** The name a unit goes by in the panel, the log and on the map: its
 *  type's name, never a callsign. */
export function unitName(u: Pick<OwnUnitView, "kind">): string {
  return UNITS.type(u.kind).name;
}

/** Page-pixel box. */
interface Box {
  x0: number;
  x1: number;
  y0: number;
  y1: number;
}
const overlaps = (a: Box, b: Box) => a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1;
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

/** Where the frame drew what the panels hang off. */
export interface DrawnAnchors {
  /** Own units' and identified enemies' drawn (interpolated) positions, so
   *  panels move with meshes; the published ones stand in for any missing. */
  own?: ReadonlyMap<number, Point3>;
  enemies?: ReadonlyMap<number, Point3>;
  /** The ground height a contact's panel hangs from; 0 without one. */
  ground?: (x: number, y: number) => number;
}

export interface ReadoutLayerHandle {
  /** Re-anchor every panel; call once per animation frame. `clock` is that
   *  frame's presentation clock in seconds, which eases a panel's nudge
   *  (null snaps it). */
  place(project: Project, distance: number, drawn?: DrawnAnchors, clock?: number | null): void;
}

/** Who a panel belongs to: its tone (own cyan, enemy red) follows. */
type Owner = "own" | "enemy" | "contact";

/** One panel: whose, where its leader starts and what it says. */
interface Callout {
  key: string;
  owner: Owner;
  id: number;
  /** The published anchor point, before the drawn position replaces it. */
  at: Point3;
  /** A contact's area radius: its leader starts at the area's border. */
  radius?: number;
  selected: boolean;
  content: ReactNode;
}

/** A unit panel's anchor: this high over its unit, about its head. */
const HEAD_M = 2;

/** Where a panel's leader starts on screen: a unit's head over `p`, or for
 *  an area of `radius` round `p` on the ground, its rightmost point, so the
 *  leader leaves the area from its border and the panel hangs clear of it. */
function anchor(project: Project, p: Point3, radius?: number): [number, number] | null {
  if (!radius) return project(p[0], p[1], p[2] + HEAD_M);
  let best: [number, number] | null = null;
  for (let k = 0; k < 8; k++) {
    const a = (k * Math.PI) / 4;
    const q = project(p[0] + radius * Math.cos(a), p[1] + radius * Math.sin(a), p[2]);
    if (q && (!best || q[0] > best[0])) best = q;
  }
  return best;
}

/** Every unit's info panel: it floats up and to the side of its unit,
 *  joined to it by a thin leader line that runs under the panel, with no box
 *  behind it. Every own unit has one (`InfoPanel`: its name, weapons, then
 *  states), in the callouts' cyan; every identified enemy and contact has
 *  one in the enemy red. A destination carries no text: its marker and route say whose
 *  it is. Positioned by the viewport each frame. Nothing is placed under an
 *  element marked `data-occludes-readouts` (a panel or bar): it moves the
 *  shortest way out. Nothing overprints: a panel that would cover another
 *  rises above it, easing there on the presentation clock rather than
 *  jumping a row. */
export function ReadoutLayer({
  own,
  identified = [],
  contacts = [],
  tick = 0,
  rules,
  selected,
  handle,
}: {
  /** The observation's own units, identified enemies and contacts: all a
   *  panel reads. Only these are props, not the whole observation, whose
   *  rounds in flight would swell React's development measure of each
   *  commit's changed props. */
  own: readonly OwnUnitView[];
  identified?: readonly IdentifiedView[];
  contacts?: readonly ContactView[];
  /** The published tick, which a contact's "ago" counts from. */
  tick?: number;
  rules: ReadoutRules;
  selected: readonly number[];
  handle: Ref<ReadoutLayerHandle>;
}) {
  const layer = useRef<HTMLDivElement>(null);
  const nodes = useRef(new Map<string, HTMLDivElement>());
  const leaders = useRef(new Map<string, SVGPathElement>());
  // Each panel's drawn nudge from its natural spot, and the clock it was
  // drawn at.
  const nudges = useRef(new Map<string, Nudge>());
  const lastClock = useRef<number | null>(null);
  const callouts: Callout[] = [
    ...own.map((u): Callout => {
      const picked = selected.includes(u.id);
      return {
        key: `own-${u.id}`,
        owner: "own",
        id: u.id,
        at: u.position,
        selected: picked,
        content: <InfoPanel panel={ownPanel(u, own, rules)} />,
      };
    }),
    ...identified.map(
      (e): Callout => ({
        key: `enemy-${e.id}`,
        owner: "enemy",
        id: e.id,
        at: e.position,
        selected: false,
        content: <InfoPanel panel={enemyPanel(e.kind, rules)} />,
      }),
    ),
    ...contacts.map(
      (c): Callout => ({
        key: `contact-${c.id}`,
        owner: "contact",
        id: c.id,
        at: [c.center[0], c.center[1], 0],
        radius: c.radius,
        selected: false,
        content: <InfoPanel panel={contactPanel(c, tick, rules)} />,
      }),
    ),
  ];
  const calloutsRef = useRef(callouts);
  calloutsRef.current = callouts;
  useImperativeHandle(handle, () => ({
    place(project, distance, drawn = {}, clock = null) {
      // Far out, every panel takes its compact form (before sizes are read).
      const zoom = distance < PANELS_FAR_M ? "default" : "far";
      if (layer.current && layer.current.dataset.zoom !== zoom) layer.current.dataset.zoom = zoom;
      // Each panel's anchor, in page pixels.
      const anchored: { id: string; node: HTMLDivElement; x: number; y: number }[] = [];
      for (const c of calloutsRef.current) {
        const node = nodes.current.get(c.key);
        if (!node) continue;
        // Zoomed out, panels stay only for the selection.
        const shown = distance < PANELS_FAR_M || c.selected;
        const p: Point3 =
          c.owner === "own"
            ? (drawn.own?.get(c.id) ?? c.at)
            : c.owner === "enemy"
              ? (drawn.enemies?.get(c.id) ?? c.at)
              : [c.at[0], c.at[1], drawn.ground?.(c.at[0], c.at[1]) ?? 0];
        const at = shown ? anchor(project, p, c.radius) : null;
        node.style.display = at ? "flex" : "none";
        if (at) anchored.push({ id: c.key, node, x: at[0], y: at[1] });
        else {
          leaders.current.get(c.key)?.setAttribute("d", "");
          nudges.current.delete(c.key);
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
      const gap = villageHud.panel_gap_px;
      // Never under a panel: move the shortest way out of it, right of a
      // side panel, below a top bar, above a bottom bar.
      const clear = (box: Box): Box => {
        for (const r of panels) {
          if (overlaps({ x0: r.left, x1: r.right, y0: r.top, y1: r.bottom }, box)) {
            const right = r.right + gap - box.x0;
            const down = r.bottom + gap - box.y0;
            const up = r.top - gap - box.y1;
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
      // A callout hits whatever lies within the gap of it, so stacked
      // callouts keep the gap between them, not merely don't overlap.
      const hit = (box: Box) =>
        placed.find((o) =>
          overlaps(o, { x0: box.x0 - gap, x1: box.x1 + gap, y0: box.y0 - gap, y1: box.y1 + gap }),
        );
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
          target = shift(target, o.y0 - gap - target.y1);
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
    <E extends Element>(map: Map<string, E>, key: string) =>
      (el: E | null) => {
        if (el) map.set(key, el);
        else map.delete(key);
      },
    [],
  );
  const leaderLines = (enemy: boolean) => (
    <svg className={`ro-leaders${enemy ? " ro-enemy" : ""}`} aria-hidden="true">
      {callouts
        .filter((c) => (c.owner !== "own") === enemy)
        .map((c) => (
          <path
            key={c.key}
            ref={bind(leaders.current, c.key)}
            className={`ro-leader ro-${c.owner}${c.selected ? " ro-selected" : ""}`}
          />
        ))}
    </svg>
  );
  return (
    <div ref={layer} className="ro-layer" data-testid="readouts">
      {leaderLines(false)}
      {leaderLines(true)}
      {callouts.map((c) => (
        <div
          key={c.key}
          ref={bind(nodes.current, c.key)}
          className={`ro-unit ro-${c.owner}${c.selected ? " ro-selected" : ""}`}
          data-owner={c.owner}
          data-unit={c.owner === "own" ? c.id : undefined}
          data-enemy={c.owner === "enemy" ? c.id : undefined}
          data-contact={c.owner === "contact" ? c.id : undefined}
        >
          {c.content}
        </div>
      ))}
    </div>
  );
}

/** The unit card: the selection's info panels, drawn by the callouts' own
 *  component. One unit: its portrait (role symbol and silhouette) beside its
 *  panel. A group: each unit's panel in its far form. No selection, no card.
 *  `own` is the side's own units (a truck's supplying reads them). */
export function SelectionCard({
  units,
  own,
  rules,
}: {
  units: readonly OwnUnitView[];
  own: readonly OwnUnitView[];
  rules: ReadoutRules;
}) {
  if (units.length === 0) return null;
  if (units.length > 1)
    return (
      <div className="hud-card hud-group" data-testid="selection-card">
        {units.map((u) => (
          <div key={u.id} data-unit={u.id}>
            <InfoPanel panel={ownPanel(u, own, rules)} zoom="far" />
          </div>
        ))}
      </div>
    );
  const [u] = units;
  const { silhouette, role } = unitIcons(UNITS.type(u.kind));
  return (
    <div className="hud-card" data-testid="selection-card" data-unit={u.id}>
      <span className="hud-portrait">
        <Icon path={role} className="ro-icon hud-role" />
        <Icon path={silhouette} className="ro-icon hud-silhouette" />
      </span>
      <InfoPanel panel={ownPanel(u, own, rules)} />
    </div>
  );
}

export interface CommandBarProps {
  mode: CommandMode;
  setMode: (mode: CommandMode) => void;
  /** The selection; the bar shows only the commands it can carry out. */
  selected: readonly OwnUnitView[];
  onStop: () => void;
  onTogglePolicy: () => void;
  /** Deploy the selection's units that deploy, or pack them. */
  onToggleDeployment: () => void;
  onExit: () => void;
}

/** One command: its icon, its short name and the key on a chip. The
 *  accessible name (and the tooltip) is the full wording, every alternative
 *  gesture included. A command only part of the selection can carry out
 *  shows how many it reaches ("1/2"). */
function CommandButton({
  icon,
  name,
  keyHint,
  label,
  pressed,
  reach: reaches,
  of,
  onClick,
}: {
  icon: string;
  name: string;
  keyHint?: string;
  label: string;
  pressed?: boolean;
  /** How many of the selection's `of` units it reaches, for a command only
   *  some units can carry out. */
  reach?: number;
  of: number;
  onClick: () => void;
}) {
  const partial = reaches !== undefined && reaches < of;
  return (
    <button
      type="button"
      className="ro-cmd"
      aria-label={label}
      title={partial ? `${label}: ${reaches} of ${of} selected` : label}
      aria-pressed={pressed}
      data-reach={partial ? `${reaches}/${of}` : undefined}
      onClick={onClick}
    >
      <Icon path={icon} className="ro-icon ro-cmd-icon" />
      <span className="ro-cmd-name" aria-hidden="true">
        {name}
      </span>
      {partial && (
        <span className="ro-cmd-reach" aria-hidden="true">
          {reaches}/{of}
        </span>
      )}
      {keyHint && <kbd aria-hidden="true">{keyHint}</kbd>}
    </button>
  );
}

/** The key chip for a binding: its first key ("X or Ctrl+right-click" → X). */
const chip = (command: keyof typeof CommandBindings) =>
  CommandBindings[command].label.split(/,| or /)[0].replace("Backspace", "BKSP");

/** The selection's commands, only those it can carry out, and the fire
 *  policy; keys are optional shortcuts, named from the one binding table.
 *  Nothing selected, no bar. */
export function CommandBar(p: CommandBarProps) {
  const n = p.selected.length;
  if (n === 0) return null;
  const key = (command: keyof typeof CommandBindings) => `(${CommandBindings[command].label})`;
  // The union of the selection's capabilities: shown when any unit can.
  const [armed, deployers, squads, inside] = (
    ["attack", "deploy", "garrison", "exit_building"] as ReachCommand[]
  ).map((command) => reach(command, p.selected, UNITS));
  const hold = p.selected.every((u) => u.engagement === "return_fire_only");
  const packing = deployers.every((u) => u.deployment?.target === "deployed");
  /** A mode's tile, when the selection can carry it out (`reaches` > 0). */
  const mode = (
    m: CommandMode,
    icon: string,
    name: string,
    keyHint: string,
    label: string,
    reaches?: readonly unknown[],
  ) =>
    reaches?.length === 0 ? null : (
      <CommandButton
        icon={icon}
        name={name}
        keyHint={keyHint}
        label={label}
        pressed={p.mode === m}
        reach={reaches?.length}
        of={n}
        onClick={() => p.setMode(m)}
      />
    );
  return (
    <div className="ro-commands" role="toolbar" aria-label="Commands">
      {mode(
        "move",
        hudIcon("move"),
        "Move",
        "RMB",
        `Move (right-click; ${FacingBinding.label.toLowerCase()} faces)`,
      )}
      {mode(
        "attack_move",
        hudIcon("attack_move"),
        "Attack-move",
        chip("attack_move"),
        `Attack-move ${key("attack_move")}`,
        armed,
      )}
      {mode(
        "reverse_move",
        hudIcon("reverse"),
        "Reverse",
        chip("reverse_move"),
        `Reverse ${key("reverse_move")}`,
      )}
      {mode(
        "attack_ground",
        hudIcon("attack_ground"),
        "Attack ground",
        chip("attack_ground"),
        `Attack ground ${key("attack_ground")}`,
        armed,
      )}
      {mode(
        "fast_move",
        hudIcon("fast_move"),
        "Fast move",
        "2×RMB",
        "Fast move (double right-click)",
      )}
      {mode(
        "garrison",
        stateIcon("building"),
        "Garrison",
        "RMB",
        "Garrison (right-click a building)",
        squads,
      )}
      <CommandButton
        icon={hudIcon("stop")}
        name="Stop"
        keyHint={chip("stop")}
        label={`Stop ${key("stop")}`}
        of={n}
        onClick={p.onStop}
      />
      <CommandButton
        icon={hudIcon(hold ? "hold_fire" : "fire_at_will")}
        name={hold ? "Return fire" : "Fire at will"}
        keyHint={chip("toggle_fire_policy")}
        label={`${hold ? "Return fire only" : "Fire at will"} ${key("toggle_fire_policy")}`}
        pressed={hold}
        of={n}
        onClick={p.onTogglePolicy}
      />
      {deployers.length > 0 && (
        <CommandButton
          icon={stateIcon(packing ? "pack" : "deploy")}
          name={packing ? "Pack" : "Deploy"}
          keyHint={chip("toggle_deployment")}
          label={`${packing ? "Pack" : "Deploy"} ${key("toggle_deployment")}`}
          reach={deployers.length}
          of={n}
          onClick={p.onToggleDeployment}
        />
      )}
      {inside.length > 0 && (
        <CommandButton
          icon={hudIcon("leave_building")}
          name="Leave building"
          label="Leave building"
          reach={inside.length}
          of={n}
          onClick={p.onExit}
        />
      )}
    </div>
  );
}
