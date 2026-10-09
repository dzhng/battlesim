/** Player readouts, from observation only (U02, U03):
 *  - an info panel off every unit on a leader line (`infoPanel.tsx`): its
 *    name, a row per weapon, then its states (`panelRows.ts`); an own
 *    unit's in cyan, with rounds left and running timers; an enemy's or a
 *    contact's in the enemy red, only what the side knows of it;
 *  - the command bar: the selection's commands and its fire policy. */
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { useSessionCatalog } from "../catalog/context";
import {
  useCallback,
  useId,
  useImperativeHandle,
  useRef,
  useState,
  type ReactNode,
  type Ref,
} from "react";
import { createPortal } from "react-dom";
import type { PresentedContact } from "./contactPresentation";
import type { IdentifiedView, OwnUnitView } from "../sim/observation";
import type { useUnitControl } from "../input/useUnitControl";
import type { CommandMode, PointerPick } from "../input/pointerIntent";
import { CommandBindings } from "../input/commandBindings";
import { reach, type ReachCommand } from "../input/commandReach";
import { hudIcon, stateIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "./icons";
import { gameHud } from "./hudTheme";
import { layoutReadoutDetails, type DetailCard } from "./readoutDetails";
import { eyePosition, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { clamp, vec3 } from "math";
import type { Box2 } from "math/shapes";
import { InfoPanel, PanelCallout, type PanelOwner } from "./infoPanel";
import { contactPanel, enemyPanel, ownPanel, type PanelRules } from "./panelRows";

/** World point → page CSS pixel, or null when behind the eye. */
export type Project = (x: number, y: number, z: number) => [number, number] | null;
type Point3 = readonly [number, number, number];
export type ReadoutHover = Pick<PointerPick, "x" | "y" | "unit" | "enemy">;
const _readout_eye = vec3.create(),
  _readout_anchor = vec3.create();

/** The name a unit goes by in the panel, the log and on the map: its
 *  type's name, never a callsign. */
export function unitName(units: UnitCatalog, u: Pick<OwnUnitView, "kind">): string {
  return units.type(u.kind).name;
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
  place(
    project: Project,
    camera: Camera3DParams,
    drawn?: DrawnAnchors,
    clock?: number | null,
    expanded?: boolean,
    hover?: ReadoutHover | null,
  ): void;
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
  opacity?: number;
  retiring?: boolean;
}

/** A unit panel's anchor: this high over its unit, about its head. */
const HEAD_M = 2;

/** Observation-only panels, positioned each frame beside their units.
 *  Stack downward in screen order, upward when the bottom fills. If neither
 *  direction fits, hide a panel that cannot fit; selected panels paint above
 *  the others. Far views allow overlap in camera-depth order.
 *  Nudges ease on the presentation clock. */
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
  contacts?: readonly PresentedContact[];
  /** The published tick, which a contact's "ago" counts from. */
  tick?: number;
  rules: PanelRules;
  selected: readonly number[];
  handle: Ref<ReadoutLayerHandle>;
}) {
  const { units } = useSessionCatalog();
  const layer = useRef<HTMLDivElement>(null);
  const nodes = useRef(new Map<string, HTMLDivElement>());
  const leaders = useRef(new Map<string, SVGPathElement>());
  // Each panel's drawn nudge from its natural spot, and the clock it was
  // drawn at.
  const nudges = useRef(new Map<string, Nudge>());
  const lastClock = useRef<number | null>(null);
  const hoveredCard = useRef<string | null>(null);
  const paintOrder = useRef(new Map<string, number>());
  const compactBoxes = useRef(new Map<string, ReadoutRect>());
  const callouts: Callout[] = [
    ...own.map((u): Callout => {
      const picked = selected.includes(u.id);
      return {
        key: `own-${u.id}`,
        owner: "own",
        id: u.id,
        at: u.position,
        selected: picked,
        content: <InfoPanel panel={ownPanel(units, u, own, rules)} />,
      };
    }),
    ...identified.map(
      (e): Callout => ({
        key: `enemy-${e.id}`,
        owner: "enemy",
        id: e.id,
        at: e.position,
        selected: false,
        content: <InfoPanel panel={enemyPanel(units, e.kind, rules)} />,
      }),
    ),
    ...contacts.map(
      (c): Callout => ({
        key: `contact-${c.id}`,
        opacity: c.opacity,
        retiring: c.retiring,
        owner: "contact",
        id: c.id,
        at: [c.center[0], c.center[1], 0],
        selected: false,
        content: <InfoPanel panel={contactPanel(units, c, tick, rules)} />,
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
        (a, b) => (paintOrder.current.get(b.key) ?? 0) - (paintOrder.current.get(a.key) ?? 0),
      );
      for (const c of priority) {
        if (c.retiring) continue;
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
    place(project, camera, drawn = {}, clock = null, expanded = false, hover = null) {
      const contains = (r: ReadoutRect | null | undefined) =>
        !!r && !!hover && hover.x >= r.x0 && hover.x <= r.x1 && hover.y >= r.y0 && hover.y <= r.y1;
      // Retain the original hit area when expansion moves the hovered card.
      const previous = hoveredCard.current;
      const held =
        previous &&
        calloutsRef.current.some((c) => c.key === previous && !c.retiring) &&
        (contains(compactBoxes.current.get(previous)) || contains(visibleBox(previous)));
      const card =
        hover &&
        [...calloutsRef.current]
          .sort(
            (a, b) => (paintOrder.current.get(b.key) ?? 0) - (paintOrder.current.get(a.key) ?? 0),
          )
          .find((c) => !c.retiring && contains(visibleBox(c.key)));
      hoveredCard.current = hover
        ? held
          ? previous
          : (card?.key ??
            (hover.unit !== null
              ? `own-${hover.unit}`
              : hover.enemy != null
                ? `enemy-${hover.enemy}`
                : null))
        : null;
      eyePosition(_readout_eye, camera);
      const overlap = camera.distance > 1000;
      // Each panel's anchor, in page pixels.
      const anchored: {
        id: string;
        node: HTMLDivElement;
        x: number;
        y: number;
        distanceSq: number;
        selected: boolean;
        opacity: number;
      }[] = [];
      for (const c of calloutsRef.current) {
        const node = nodes.current.get(c.key);
        if (!node) continue;
        const hovering = c.key === hoveredCard.current;
        const p: Point3 =
          c.owner === "own"
            ? (drawn.own?.get(c.id) ?? c.at)
            : c.owner === "enemy"
              ? (drawn.enemies?.get(c.id) ?? c.at)
              : [c.at[0], c.at[1], drawn.ground?.(c.at[0], c.at[1]) ?? 0];
        // A panel hangs off a unit in view; one whose anchor is off screen hides.
        const q = project(p[0], p[1], p[2] + (c.owner === "contact" ? 0 : HEAD_M));
        const edge = gameHud.panel_edge_hide_px;
        // Hover overrides edge hiding; keep its card inside the viewport.
        const at: [number, number] | null =
          hovering && q
            ? [clamp(q[0], 0, window.innerWidth), clamp(q[1], 0, window.innerHeight)]
            : q &&
                q[0] >= edge &&
                q[1] >= edge &&
                q[0] <= window.innerWidth - edge &&
                q[1] <= window.innerHeight - edge
              ? q
              : null;
        node.style.display = at ? "flex" : "none";
        if (at)
          anchored.push({
            id: c.key,
            node,
            selected: c.selected,
            opacity: c.opacity ?? 1,
            x: at[0],
            y: at[1],
            distanceSq: vec3.squaredDistance(
              _readout_eye,
              vec3.set(_readout_anchor, p[0], p[1], p[2]),
            ),
          });
        else {
          leaders.current.get(c.key)?.setAttribute("d", "");
          nudges.current.delete(c.key);
        }
      }
      const zoom = expanded ? "default" : "compressed";
      if (layer.current && layer.current.dataset.zoom !== zoom) layer.current.dataset.zoom = zoom;
      // Measure detail separately, then always lay out the compact footprints.
      const fullSizes = new Map<string, { w: number; h: number }>();
      const detailing = expanded || hoveredCard.current !== null;
      if (detailing) {
        for (const a of anchored) a.node.dataset.zoom = "default";
        for (const a of anchored)
          fullSizes.set(a.id, { w: a.node.offsetWidth, h: a.node.offsetHeight });
      }
      for (const a of anchored) a.node.dataset.zoom = "compressed";
      const situated: { b: (typeof anchored)[number]; box: ReadoutRect }[] = [];
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
      const gap = gameHud.panel_gap_px;
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
        for (let direction = 1; !overlap && direction >= -1; direction -= 2) {
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
        const n = easeNudge(overlap ? undefined : nudges.current.get(b.id), want, dt);
        nudges.current.set(b.id, n);
        const box = {
          x0: natural.x0 + n.dx,
          x1: natural.x1 + n.dx,
          y0: natural.y0 + n.dy,
          y1: natural.y1 + n.dy,
        };
        b.node.style.transform = `translate(${box.x0}px, ${box.y0}px)`;
        situated.push({ b, box });
      }
      const viewport: Box2 = [EDGE_PX, EDGE_PX, right, window.innerHeight - EDGE_PX];
      compactBoxes.current.clear();
      for (const { b, box } of situated) compactBoxes.current.set(b.id, box);
      let details: ReturnType<typeof layoutReadoutDetails> | null = null;
      if (detailing && !overlap) {
        const detailCards: DetailCard[] = situated.map(({ b, box }) => {
          const size = fullSizes.get(b.id)!;
          return {
            id: b.id,
            distanceSq: b.distanceSq,
            compact: [box.x0, box.y0, box.x1, box.y1],
            full: [box.x0, box.y0, box.x0 + size.w, box.y0 + size.h],
          };
        });
        const obstacles: Box2[] = [
          ...panels.map((r): Box2 => [r.left, r.top, r.right, r.bottom]),
          ...callouts.map((b): Box2 => [b.x - 6, b.y - 6, b.x + 6, b.y + 6]),
        ];
        details = layoutReadoutDetails(
          detailCards,
          obstacles,
          viewport,
          gap,
          expanded,
          hoveredCard.current,
        );
      }
      paintOrder.current.clear();
      const near = [...situated].sort((a, b) => a.b.distanceSq - b.b.distanceSq);
      for (const [i, { b }] of near.entries()) {
        const order =
          b.id === hoveredCard.current
            ? near.length + 1
            : overlap
              ? near.length - i
              : Number(b.selected);
        paintOrder.current.set(b.id, order);
        b.node.parentElement!.style.zIndex = String(order);
      }
      const drawnBoxes = new Map<string, ReadoutRect>();
      for (const { b, box: compact } of situated) {
        const detail = details?.get(b.id);
        let box = detail
          ? { x0: detail.box[0], y0: detail.box[1], x1: detail.box[2], y1: detail.box[3] }
          : compact;
        b.node.dataset.zoom = (overlap ? expanded || b.id === hoveredCard.current : detail?.full)
          ? "default"
          : "compressed";
        if (overlap && b.node.dataset.zoom === "default") {
          const size = fullSizes.get(b.id)!;
          const x0 = clamp(box.x0, EDGE_PX, right - size.w);
          const y0 = clamp(box.y0, EDGE_PX, window.innerHeight - EDGE_PX - size.h);
          box = { x0, y0, x1: x0 + size.w, y1: y0 + size.h };
        }
        drawnBoxes.set(b.id, box);
        b.node.style.transform = `translate(${box.x0}px, ${box.y0}px)`;
        const group = b.node.parentElement!;
        if (b.id === hoveredCard.current) group.dataset.hovered = "true";
        else delete group.dataset.hovered;
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
      for (const { b } of situated) {
        const box = drawnBoxes.get(b.id)!;
        const behind =
          overlap &&
          near.some(
            ({ b: other }) =>
              paintOrder.current.get(other.id)! > paintOrder.current.get(b.id)! &&
              overlaps(box, drawnBoxes.get(other.id)!),
          );
        const inFront =
          overlap &&
          near.some(
            ({ b: other }) =>
              paintOrder.current.get(other.id)! < paintOrder.current.get(b.id)! &&
              overlaps(box, drawnBoxes.get(other.id)!),
          );
        const softened = behind && b.id !== hoveredCard.current;
        const group = b.node.parentElement!;
        if (inFront) group.dataset.overlapping = "true";
        else delete group.dataset.overlapping;
        group.style.opacity = String(b.opacity * (softened ? 0.65 : 1));
        group.style.filter = softened ? "blur(0.6px)" : "";
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
    <div ref={layer} className="ro-layer" data-testid="readouts" data-zoom="compressed">
      {callouts.map((c) => (
        <div
          key={c.key}
          className={`ro-callout${c.selected ? " ro-selected" : ""}`}
          style={{ opacity: c.opacity }}
          data-retiring={c.retiring || undefined}
        >
          <PanelCallout
            ref={bind(nodes.current, c.key)}
            owner={c.owner}
            selected={c.selected}
            data-zoom="compressed"
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

interface CommandProps {
  icon: string;
  label: string;
  pressed?: boolean;
  disabled?: boolean;
  onClick: () => void;
}

function CommandButton({
  icon,
  label,
  pressed,
  disabled,
  onClick,
  hintHost,
}: CommandProps & { hintHost?: HTMLElement | null }) {
  const id = useId();
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const [hintCenter, setHintCenter] = useState(0);
  const alignHint = (button: HTMLButtonElement) => {
    if (!hintHost) return;
    const owner = button.getBoundingClientRect();
    // The empty hint host is hidden; its fixed layout parent still has bounds.
    const lower = hintHost.parentElement!.getBoundingClientRect();
    setHintCenter(owner.left + owner.width / 2 - lower.left);
  };
  const shown = (hovered || focused) && !dismissed;
  const tooltip = shown ? (
    <span
      className="ro-cmd-tooltip"
      role="tooltip"
      id={id}
      style={hintHost ? { left: hintCenter } : undefined}
    >
      {label}
    </span>
  ) : null;
  return (
    <button
      type="button"
      className="ro-cmd"
      aria-label={label}
      aria-describedby={shown ? id : undefined}
      aria-pressed={pressed}
      disabled={disabled}
      onClick={onClick}
      onMouseEnter={(event) => {
        alignHint(event.currentTarget);
        setHovered(true);
        setDismissed(false);
      }}
      onMouseLeave={() => setHovered(false)}
      onFocus={(event) => {
        alignHint(event.currentTarget);
        setFocused(true);
        setDismissed(false);
      }}
      onBlur={() => setFocused(false)}
      onKeyDown={(event) => {
        if (event.key === "Escape") setDismissed(true);
      }}
    >
      <Icon path={icon} className="ro-cmd-icon" />
      {hintHost ? createPortal(tooltip, hintHost) : tooltip}
    </button>
  );
}

/** The selection's commands and fire policy, with unavailable actions disabled.
 *  Keys are optional shortcuts, named from the one binding table.
 *  Nothing selected, no bar. */
export function CommandBar({
  control,
  hintHost,
}: {
  hintHost?: HTMLElement | null;
  control: Pick<
    ReturnType<typeof useUnitControl>,
    | "mode"
    | "setMode"
    | "selectedUnits"
    | "stop"
    | "togglePolicy"
    | "toggleDeployment"
    | "exitBuilding"
  > & { refund?: () => void };
}) {
  const { units } = useSessionCatalog();
  const selected = control.selectedUnits;
  const n = selected.length;
  const command = (c: CommandProps) => <CommandButton {...c} hintHost={hintHost} />;
  const key = (command: keyof typeof CommandBindings) => `(${CommandBindings[command].label})`;
  // Capability actions apply to eligible units; movement applies to everyone.
  const [armed, deployers, inside] = (["attack", "deploy", "exit_building"] as ReachCommand[]).map(
    (c) => reach(c, selected, units),
  );
  const hold = n > 0 && selected.every((u) => u.engagement === "return_fire_only");
  const packing = deployers.every((u) => u.deployment?.target === "deployed");
  /** Movement stays available; an unsupported targeted attack is disabled. */
  const mode = (m: CommandMode, icon: string, label: string, available = n > 0) =>
    command({
      icon,
      label,
      pressed: control.mode === m,
      disabled: !available,
      onClick: () => control.setMode(m),
    });
  return (
    <div className="ro-commands" role="toolbar" aria-label="Commands">
      {mode("attack_move", hudIcon("attack_move"), `Attack-move ${key("attack_move")}`)}
      {mode("reverse_move", hudIcon("reverse"), `Reverse ${key("reverse_move")}`)}
      {mode(
        "attack_ground",
        hudIcon("attack_ground"),
        `Attack ground ${key("attack_ground")}`,
        armed.length > 0,
      )}
      {mode("fast_move", hudIcon("fast_move"), "Fast move (double right-click)")}
      {command({
        icon: hudIcon("stop"),
        label: `Stop ${key("stop")}`,
        disabled: n === 0,
        onClick: control.stop,
      })}
      {command({
        icon: hudIcon(hold ? "hold_fire" : "fire_at_will"),
        label: `${hold ? "Return fire only" : "Fire at will"} ${key("toggle_fire_policy")}`,
        pressed: hold,
        disabled: n === 0,
        onClick: control.togglePolicy,
      })}
      {command({
        icon: stateIcon(deployers.length > 0 && packing ? "pack" : "deploy"),
        label: `${deployers.length > 0 && packing ? "Pack" : "Deploy"} ${key("toggle_deployment")}`,
        disabled: deployers.length === 0,
        onClick: control.toggleDeployment,
      })}
      {control.refund &&
        command({
          icon: stateIcon("withdrawing"),
          label: "Refund · Return to base",
          disabled: n === 0,
          onClick: control.refund,
        })}
      {command({
        icon: hudIcon("leave_building"),
        label: "Leave building",
        disabled: inside.length === 0,
        onClick: control.exitBuilding,
      })}
    </div>
  );
}
