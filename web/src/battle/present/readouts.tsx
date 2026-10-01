/** Player readouts, from observation only (U02, U03):
 *  - an info panel off every unit on a leader line (`infoPanel.tsx`): its
 *    name, a row per weapon, then its states (`panelRows.ts`); an own
 *    unit's in cyan, with rounds left and running timers; an enemy's or a
 *    contact's in the enemy red, only what the side knows of it;
 *  - the unit card: the selection's panels, the same component, at any zoom;
 *  - the command bar: the selection's commands and its fire policy. */
import { useCallback, useImperativeHandle, useRef, type ReactNode, type Ref } from "react";
import type { ContactView, IdentifiedView, OwnUnitView } from "../sim/observation";
import type { CommandMode, useUnitControl } from "../input/useUnitControl";
import { CommandBindings, FacingBinding } from "../input/commandBindings";
import { reach, type ReachCommand } from "../input/commandReach";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { hudIcon, stateIcon, unitIcons } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";
import { villageHud } from "./hudTheme";
import { InfoPanel, PanelCallout, type PanelOwner } from "./infoPanel";
import { contactPanel, enemyPanel, ownPanel, type PanelRules } from "./panelRows";

/** World point → page CSS pixel, or null when behind the eye. */
export type Project = (x: number, y: number, z: number) => [number, number] | null;
type Point3 = readonly [number, number, number];

/** Above this camera distance, panels show only for selected units (every
 *  other own panel, and every enemy's and contact's, hides), each in its
 *  compact far form: the name over one line of icons and counts. */
const PANELS_FAR_M = 700;

/** The name a unit goes by in the panel, the log and on the map: its
 *  type's name, never a callsign. */
export function unitName(u: Pick<OwnUnitView, "kind">): string {
  return UNITS.type(u.kind).name;
}

/** Page-pixel box. */
export interface ReadoutRect {
  x0: number;
  x1: number;
  y0: number;
  y1: number;
}
const overlaps = (a: ReadoutRect, b: ReadoutRect) =>
  a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1;
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
interface DrawnAnchors {
  /** Own units' and identified enemies' drawn (interpolated) positions, so
   *  panels move with meshes; the published ones stand in for any missing. */
  own?: ReadonlyMap<number, Point3>;
  enemies?: ReadonlyMap<number, Point3>;
  /** The ground height a contact's panel hangs from; 0 without one. */
  ground?: (x: number, y: number) => number;
}

export interface ReadoutLayerHandle {
  pick(
    x: number,
    y: number,
  ): { unit: number | null; enemy: number | null; contact: number | null } | null;
  inRect(unit: number, rect: ReadoutRect): boolean;
  /** Re-anchor every panel; call once per animation frame. `clock` is that
   *  frame's presentation clock in seconds, which eases a panel's nudge
   *  (null snaps it). */
  place(project: Project, distance: number, drawn?: DrawnAnchors, clock?: number | null): void;
}

/** One panel: whose, where its leader starts and what it says. */
interface Callout {
  key: string;
  owner: PanelOwner;
  id: number;
  /** The published anchor point, before the drawn position replaces it. */
  at: Point3;
  selected: boolean;
  content: ReactNode;
}

/** A unit panel's anchor: this high over its unit, about its head. */
const HEAD_M = 2;

/** Observation-only panels, positioned each frame beside their units.
 *  Stack downward in screen order, upward when the bottom fills. If neither
 *  direction fits, hide a panel that cannot fit; selected panels paint above
 *  the others. Nudges ease on the presentation clock. */
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
  rules: PanelRules;
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
    ...contacts
      .filter((c) => c.primaryLabel)
      .map(
        (c): Callout => ({
          key: `contact-${c.id}`,
          owner: "contact",
          id: c.id,
          at: [c.center[0], c.center[1], 0],
          selected: false,
          content: <InfoPanel panel={contactPanel(c, tick, rules)} />,
        }),
      ),
  ];
  const calloutsRef = useRef(callouts);
  calloutsRef.current = callouts;
  const visibleBox = (key: string): ReadoutRect | null => {
    const node = nodes.current.get(key);
    if (!node || node.style.display === "none") return null;
    const r = node.getBoundingClientRect();
    return { x0: r.left, x1: r.right, y0: r.top, y1: r.bottom };
  };
  useImperativeHandle(handle, () => ({
    pick(x, y) {
      const priority = [...calloutsRef.current].sort(
        (a, b) => Number(b.selected) - Number(a.selected),
      );
      for (const c of priority) {
        const r = visibleBox(c.key);
        if (r && x >= r.x0 && x <= r.x1 && y >= r.y0 && y <= r.y1)
          return {
            unit: c.owner === "own" ? c.id : null,
            enemy: c.owner === "enemy" ? c.id : null,
            contact: c.owner === "contact" ? c.id : null,
          };
      }
      return null;
    },
    inRect(unit, rect) {
      const r = visibleBox(`own-${unit}`);
      return !!r && overlaps(r, rect);
    },
    place(project, distance, drawn = {}, clock = null) {
      // Far out, every panel takes its compact form (before sizes are read).
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
        // A panel hangs off a unit in view; one whose anchor is off screen hides.
        const q = shown ? project(p[0], p[1], p[2] + (c.owner === "contact" ? 0 : HEAD_M)) : null;
        const at =
          q &&
          q[0] >= villageHud.panel_edge_hide_px &&
          q[1] >= villageHud.panel_edge_hide_px &&
          q[0] <= window.innerWidth - villageHud.panel_edge_hide_px &&
          q[1] <= window.innerHeight - villageHud.panel_edge_hide_px
            ? q
            : null;
        node.style.display = at ? "flex" : "none";
        if (at) anchored.push({ id: c.key, node, x: at[0], y: at[1] });
        else {
          leaders.current.get(c.key)?.setAttribute("d", "");
          nudges.current.delete(c.key);
        }
      }
      const zoom =
        anchored.length > villageHud.panel_compress_above
          ? "compressed"
          : distance < PANELS_FAR_M
            ? "default"
            : "far";
      if (layer.current && layer.current.dataset.zoom !== zoom) layer.current.dataset.zoom = zoom;
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
      // On screen, and never under a panel: below the top edge, then the
      // shortest way out of a panel, right of a side panel, below a top
      // plate, above a bottom bar; never up off the top of the screen.
      const clear = (box: ReadoutRect): ReadoutRect => {
        if (box.y0 < EDGE_PX) box = shift(box, EDGE_PX - box.y0);
        for (const r of panels) {
          if (overlaps({ x0: r.left, x1: r.right, y0: r.top, y1: r.bottom }, box)) {
            const right = r.right + gap - box.x0;
            const down = r.bottom + gap - box.y0;
            const up = r.top - gap - box.y1;
            const dy = Math.abs(down) < Math.abs(up) || box.y0 + up < 0 ? down : up;
            box =
              right <= Math.abs(dy)
                ? { ...box, x0: box.x0 + right, x1: box.x1 + right }
                : shift(box, dy);
          }
        }
        return box;
      };
      const placed: ReadoutRect[] = panels.map((r) => ({
        x0: r.left,
        x1: r.right,
        y0: r.top,
        y1: r.bottom,
      }));
      // Pad obstacles so collision checks and placement use the same edge,
      // without subtracting the gap again and rounding back into a collision.
      const hit = (box: ReadoutRect) =>
        placed.find((o) =>
          overlaps({ x0: o.x0 - gap, x1: o.x1 + gap, y0: o.y0 - gap, y1: o.y1 + gap }, box),
        );
      const shift = (box: ReadoutRect, dy: number): ReadoutRect => ({
        ...box,
        y0: box.y0 + dy,
        y1: box.y1 + dy,
      });
      const right = window.innerWidth - EDGE_PX;
      // Highest unit first; keep unit anchors clear as well as panels.
      const callouts = [...boxes].sort((m, n) => m.y - n.y);
      for (const b of callouts) placed.push({ x0: b.x - 6, x1: b.x + 6, y0: b.y - 6, y1: b.y + 6 });
      for (const b of callouts) {
        const left = b.x + CALLOUT_SIDE_PX + b.w > right;
        const x0 = left ? b.x - CALLOUT_SIDE_PX - b.w : b.x + CALLOUT_SIDE_PX;
        const y1 = b.y - CALLOUT_RISE_PX;
        const natural = { x0, x1: x0 + b.w, y0: y1 - b.h, y1 };
        let target = clear(natural);
        // Both walks move strictly past each hit, so neither can cycle.
        for (let direction = 1; direction >= -1; direction -= 2) {
          let candidate = target;
          for (let o = hit(candidate); o; o = hit(candidate)) {
            const edge = direction > 0 ? o.y1 + gap : o.y0 - gap;
            candidate = {
              ...candidate,
              y0: direction > 0 ? edge : edge - b.h,
              y1: direction > 0 ? edge + b.h : edge,
            };
          }
          if (candidate.y0 >= EDGE_PX && candidate.y1 <= window.innerHeight - EDGE_PX) {
            target = candidate;
            break;
          }
        }
        if (
          target.x0 < EDGE_PX ||
          target.x1 > right ||
          target.y0 < EDGE_PX ||
          target.y1 > window.innerHeight - EDGE_PX
        ) {
          b.node.style.display = "none";
          leaders.current.get(b.id)?.setAttribute("d", "");
          nudges.current.delete(b.id);
          continue;
        }
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
  return (
    <div ref={layer} className="ro-layer" data-testid="readouts">
      {callouts.map((c) => (
        <div key={c.key} className={`ro-callout${c.selected ? " ro-selected" : ""}`}>
          <PanelCallout
            ref={bind(nodes.current, c.key)}
            owner={c.owner}
            selected={c.selected}
            data-unit={c.owner === "own" ? c.id : undefined}
            data-enemy={c.owner === "enemy" ? c.id : undefined}
            data-contact={c.owner === "contact" ? c.id : undefined}
          >
            {c.content}
          </PanelCallout>
          <svg className={`ro-leaders${c.owner !== "own" ? " ro-enemy" : ""}`} aria-hidden="true">
            <path
              ref={bind(leaders.current, c.key)}
              className={`ro-leader ro-${c.owner}${c.selected ? " ro-selected" : ""}`}
            />
          </svg>
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
  rules: PanelRules;
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
        <Icon path={role} className="hud-role" />
        <Icon path={silhouette} className="hud-silhouette" />
      </span>
      <InfoPanel panel={ownPanel(u, own, rules)} />
    </div>
  );
}

interface CommandProps {
  icon: string;
  name: string;
  keyHint?: string;
  label: string;
  pressed?: boolean;
  /** How many of the selection's `of` units it reaches, for a command only
   *  some units can carry out. */
  reach?: number;
  onClick: () => void;
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
}: CommandProps & { of: number }) {
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
      <Icon path={icon} className="ro-cmd-icon" />
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
export function CommandBar({
  control,
}: {
  control: Pick<
    ReturnType<typeof useUnitControl>,
    | "mode"
    | "setMode"
    | "selectedUnits"
    | "stop"
    | "togglePolicy"
    | "toggleDeployment"
    | "exitBuilding"
  >;
}) {
  const selected = control.selectedUnits;
  const n = selected.length;
  if (n === 0) return null;
  const command = (c: CommandProps) => <CommandButton {...c} of={n} />;
  const key = (command: keyof typeof CommandBindings) => `(${CommandBindings[command].label})`;
  // The union of the selection's capabilities: shown when any unit can.
  const [armed, deployers, squads, inside] = (
    ["attack", "deploy", "garrison", "exit_building"] as ReachCommand[]
  ).map((c) => reach(c, selected, UNITS));
  const hold = selected.every((u) => u.engagement === "return_fire_only");
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
    reaches?.length === 0
      ? null
      : command({
          icon,
          name,
          keyHint,
          label,
          pressed: control.mode === m,
          reach: reaches?.length,
          onClick: () => control.setMode(m),
        });
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
      {command({
        icon: hudIcon("stop"),
        name: "Stop",
        keyHint: chip("stop"),
        label: `Stop ${key("stop")}`,
        onClick: control.stop,
      })}
      {command({
        icon: hudIcon(hold ? "hold_fire" : "fire_at_will"),
        name: hold ? "Return fire" : "Fire at will",
        keyHint: chip("toggle_fire_policy"),
        label: `${hold ? "Return fire only" : "Fire at will"} ${key("toggle_fire_policy")}`,
        pressed: hold,
        onClick: control.togglePolicy,
      })}
      {deployers.length > 0 &&
        command({
          icon: stateIcon(packing ? "pack" : "deploy"),
          name: packing ? "Pack" : "Deploy",
          keyHint: chip("toggle_deployment"),
          label: `${packing ? "Pack" : "Deploy"} ${key("toggle_deployment")}`,
          reach: deployers.length,
          onClick: control.toggleDeployment,
        })}
      {inside.length > 0 &&
        command({
          icon: hudIcon("leave_building"),
          name: "Leave building",
          label: "Leave building",
          reach: inside.length,
          onClick: control.exitBuilding,
        })}
    </div>
  );
}
