/** Player readouts, from observation only (U02, U03):
 *  - an info panel off every unit on a leader line (`infoPanel.tsx`): its
 *    name, a row per weapon, then its states (`panelRows.ts`); an own
 *    unit's in cyan, with rounds left and running timers; an ally's the
 *    same panel in pale ice on a broken leader, naming its owner, and held
 *    open while it is selected (to read: it takes no order); an enemy's or a
 *    contact's in the enemy red, only what the side knows of it. */
import { highAir, type UnitCatalog } from "@packages/scene-assets/src/units";
import { useSessionCatalog } from "../catalog/context";
import { useCallback, useImperativeHandle, useRef, type ReactNode, type Ref } from "react";
import type { PresentedContact } from "./contactPresentation";
import {
  aboardCarrier,
  passengersOf,
  type IdentifiedView,
  type OwnUnitView,
} from "../sim/observationView";
import type { PointerPick } from "../input/pointerIntent";
import { gameHud } from "./hudTheme";
import { layoutReadoutDetails, type DetailCard, type DetailPlacement } from "./readoutDetails";
import {
  layoutReadouts,
  leaderOf,
  overlaps,
  readoutPaintOrder,
  READOUT_ANCHOR_PX,
  type ReadoutRect,
} from "./readoutLayout";
import { eyePosition, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { clamp, vec3 } from "math";
import type { Box2 } from "math/shapes";
import {
  InfoPanel,
  PanelCallout,
  PassengerCards,
  sideTone,
  type PanelOwner,
  type PassengerCard,
} from "./infoPanel";
import { contactPanel, enemyPanel, ownPanel, type Panel, type PanelRules } from "./panelRows";

/** The element the high-air models' canvas lies in, between the two card
 *  planes: a battle view draws its high-air models there when the page has
 *  one (the shared renderer's high-air target), and with the world otherwise. */
export const HIGH_PLANE_ATTR = "data-high-plane";

/** A camera this close to the ground is watching the battle, not commanding
 *  it: cards are whole from `CARD_FADE_START_M` of camera distance out and
 *  fade away linearly to nothing at `CARD_FADE_END_M`, so a close shot is
 *  clean film. */
export const CARD_FADE_START_M = 90;
export const CARD_FADE_END_M = 48;

/** The cards' opacity at camera distance `cameraDistance`. */
export function cardZoomOpacity(cameraDistance: number): number {
  if (cameraDistance >= CARD_FADE_START_M) return 1;
  if (cameraDistance <= CARD_FADE_END_M) return 0;
  return (cameraDistance - CARD_FADE_END_M) / (CARD_FADE_START_M - CARD_FADE_END_M);
}

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

const NO_ALLIES: ReadonlyMap<number, string> = new Map();
/** A panel of one of the side's own units, the player's or an ally's. */
const sideUnit = (owner: PanelOwner) => owner === "own" || owner === "ally";

/** The ring a leader line starts from, on its unit. */
const LEADER_RING_PX = 2;
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
  /** The ground height a ground contact's panel hangs from; 0 without one.
   *  An aloft contact's hangs at its reported height. */
  ground?: (x: number, y: number) => number;
}

/** Where `c`'s panel hangs: its unit as the frame drew it (a ghost where it
 *  stands), or a ground contact on the drawn ground. */
function anchorOf(c: Callout, drawn: DrawnAnchors): Point3 {
  if (c.ghost) return c.at;
  if (sideUnit(c.owner)) return drawn.own?.get(c.id) ?? c.at;
  if (c.owner === "enemy") return drawn.enemies?.get(c.id) ?? c.at;
  return c.onGround ? [c.at[0], c.at[1], drawn.ground?.(c.at[0], c.at[1]) ?? 0] : c.at;
}

export interface GhostReadout {
  unit: OwnUnitView;
  position: Point3;
}

export interface ReadoutLayerHandle {
  pick(
    x: number,
    y: number,
  ): { unit: number | null; enemy: number | null; contact: number | null } | null;
  inRect(unit: number, rect: ReadoutRect): boolean;
  /** Where own `unit`'s card is drawn, or its carrier's for a rider; null
   *  while neither shows. */
  cardBox(unit: number): ReadoutRect | null;
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
  /** Hang from the drawn ground under `at` (a contact not aloft), not its z. */
  onGround?: boolean;
  selected: boolean;
  /** Its unit flies in the high-air layer: its card is laid out in the high
   *  plane, above the ground and low-air cards and the high-air models. */
  highAir: boolean;
  content: ReactNode;
  /** The squads a carrier holds, their compact cards stacked under its own. */
  passengers?: PassengerCard[];
  opacity?: number;
  retiring?: boolean;
  ghost?: boolean;
}

/** A carrier's underline lies this far below its own panel, in the gap
 *  above its riders' cards (`.ro-aboard`'s top margin). */
const RIDER_RULE_PX = 3;
/** The box a callout's leader underlines, drawn at `box`: the callout's,
 *  or for a carrier's card hanging above its unit, its own panel's, so the
 *  rule runs above its riders' cards and the line names the carrier. */
function underlined(
  b: { node: HTMLElement; y: number; riders: boolean },
  box: ReadoutRect,
): ReadoutRect {
  const body = b.node.firstElementChild as HTMLElement | null;
  if (!b.riders || !body || box.y1 > b.y) return box;
  return { ...box, y1: box.y0 + body.offsetTop + body.offsetHeight + RIDER_RULE_PX };
}

/** A unit panel's anchor: this high over its unit, about its head. */
const HEAD_M = 2;

/** Observation-only panels, positioned each frame beside their units by
 *  `layoutReadouts`, in two planes back to front: the ground and low-air
 *  cards; the high-air models' canvas (`HIGH_PLANE_ATTR`, which the battle
 *  view draws into); the high-air cards. In each plane cards make room for
 *  each other on short, uncrossed leaders, and a selected or hovered panel
 *  paints above the others. A card near the viewport's edge hides. Far
 *  views allow overlap in camera-depth order. Nudges ease on the
 *  presentation clock. A squad riding a carrier has no callout of its own:
 *  its compact card stacks under the carrier's, inside that callout, and
 *  picks the carrier. */
export function ReadoutLayer({
  own,
  identified = [],
  contacts = [],
  tick = 0,
  rules,
  selected,
  handle,
  ghosts = [],
  allies = NO_ALLIES,
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
  ghosts?: readonly GhostReadout[];
  /** Each of `own` an ally owns, with its panel's owner line (`allyOwners`). */
  allies?: ReadonlyMap<number, string>;
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
  const zoomOpacity = useRef(1);
  /** A unit on the side's panel, naming its owner when it is an ally's. */
  const sidePanel = (u: OwnUnitView): Panel => {
    const panel = ownPanel(units, u, own, rules);
    const owner = allies.get(u.id);
    return owner === undefined ? panel : { ...panel, owner };
  };
  const callouts: Callout[] = [
    ...ghosts.map(
      (g): Callout => ({
        key: `ghost-${g.unit.id}`,
        owner: "own",
        ghost: true,
        id: -g.unit.id - 1,
        at: g.position,
        selected: false,
        highAir: highAir(units.type(g.unit.kind)),
        content: <InfoPanel panel={ownPanel(units, g.unit, own, rules)} zoom="far" />,
      }),
    ),
    ...own
      // A squad riding a carrier has its card stacked under the carrier's.
      .filter((u) => !aboardCarrier(u))
      .map((u): Callout => {
        const panel = sidePanel(u);
        const passengers = passengersOf(own, u.id).map((p) => ({ id: p.id, panel: sidePanel(p) }));
        return {
          key: `own-${u.id}`,
          owner: sideTone(panel),
          id: u.id,
          at: u.position,
          selected: selected.includes(u.id),
          highAir: highAir(units.type(u.kind)),
          content: <InfoPanel panel={panel} />,
          ...(passengers.length ? { passengers } : {}),
        };
      }),
    ...identified.map(
      (e): Callout => ({
        key: `enemy-${e.id}`,
        owner: "enemy",
        id: e.id,
        at: e.position,
        selected: false,
        highAir: highAir(units.type(e.kind)),
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
        at: c.center,
        onGround: !c.aloft,
        selected: false,
        highAir: c.layer === "high_air",
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
  /** The card under `hover`, else the unit or enemy it points at. The card
   *  hovered last frame keeps hold while the pointer is in its compact or
   *  drawn box, so expanding it never moves it out from under the pointer. */
  const hoveredKey = (hover: ReadoutHover | null): string | null => {
    const contains = (r: ReadoutRect | null | undefined) =>
      !!r && !!hover && hover.x >= r.x0 && hover.x <= r.x1 && hover.y >= r.y0 && hover.y <= r.y1;
    const previous = hoveredCard.current;
    const held =
      previous &&
      calloutsRef.current.some((c) => c.key === previous && !c.retiring && !c.ghost) &&
      (contains(compactBoxes.current.get(previous)) || contains(visibleBox(previous)));
    const card =
      hover &&
      [...calloutsRef.current]
        .sort((a, b) => (paintOrder.current.get(b.key) ?? 0) - (paintOrder.current.get(a.key) ?? 0))
        .find((c) => !c.retiring && !c.ghost && contains(visibleBox(c.key)));
    return hover
      ? held
        ? previous
        : (card?.key ??
          (hover.unit !== null
            ? `own-${hover.unit}`
            : hover.enemy != null
              ? `enemy-${hover.enemy}`
              : null))
      : null;
  };
  /** The card shown open: the hovered one, else a selected ally's, held
   *  open as a hovered one is, since selecting it is how the player reads it. */
  const openKey = (hover: ReadoutHover | null): string | null =>
    hoveredKey(hover) ??
    calloutsRef.current.find((c) => c.owner === "ally" && c.selected)?.key ??
    null;
  /** A panel off the layout this frame: no leader, and no nudge to ease from. */
  const unplace = (key: string) => {
    leaders.current.get(key)?.setAttribute("d", "");
    nudges.current.delete(key);
  };
  /** In a far view, where cards may overlap: a card another paints over is
   *  softened (unless hovered), and one painted over another is marked. */
  const shadeOverlaps = (
    cards: readonly { id: string; node: HTMLDivElement; distanceSq: number; opacity: number }[],
    drawnBoxes: ReadonlyMap<string, ReadoutRect>,
    overlap: boolean,
  ) => {
    const near = [...cards].sort((a, b) => a.distanceSq - b.distanceSq);
    for (const b of cards) {
      const box = drawnBoxes.get(b.id)!;
      const behind =
        overlap &&
        near.some(
          (other) =>
            paintOrder.current.get(other.id)! > paintOrder.current.get(b.id)! &&
            overlaps(box, drawnBoxes.get(other.id)!),
        );
      const inFront =
        overlap &&
        near.some(
          (other) =>
            paintOrder.current.get(other.id)! < paintOrder.current.get(b.id)! &&
            overlaps(box, drawnBoxes.get(other.id)!),
        );
      const softened = behind && b.id !== hoveredCard.current;
      const group = b.node.parentElement!;
      if (inFront) group.dataset.overlapping = "true";
      else delete group.dataset.overlapping;
      // A close camera fades every card out (`cardZoomOpacity`); a faded card takes no clicks.
      const opacity = b.opacity * (softened ? 0.65 : 1) * zoomOpacity.current;
      group.style.opacity = String(opacity);
      group.style.filter = softened ? "blur(0.6px)" : "";
      group.style.pointerEvents = opacity <= 0 ? "none" : "";
    }
  };
  useImperativeHandle(handle, () => ({
    pick(x, y) {
      // Cards faded out by a close camera take no clicks.
      if (zoomOpacity.current <= 0) return null;
      const priority = [...calloutsRef.current].sort(
        (a, b) => (paintOrder.current.get(b.key) ?? 0) - (paintOrder.current.get(a.key) ?? 0),
      );
      for (const c of priority) {
        if (c.retiring || c.ghost) continue;
        // A rider's card lies on its carrier's, so it picks the carrier.
        const r = visibleBox(c.key);
        if (r && x >= r.x0 && x <= r.x1 && y >= r.y0 && y <= r.y1)
          return {
            unit: sideUnit(c.owner) ? c.id : null,
            enemy: c.owner === "enemy" ? c.id : null,
            contact: c.owner === "contact" ? c.id : null,
          };
      }
      return null;
    },
    inRect(unit, rect) {
      if (zoomOpacity.current <= 0) return false;
      const r = visibleBox(`own-${unit}`);
      return !!r && overlaps(r, rect);
    },
    cardBox(unit) {
      const carrier = calloutsRef.current.find((c) => c.passengers?.some((p) => p.id === unit));
      return visibleBox(carrier?.key ?? `own-${unit}`);
    },
    place(project, camera, drawn = {}, clock = null, expanded = false, hover = null) {
      zoomOpacity.current = cardZoomOpacity(camera.distance);
      hoveredCard.current = openKey(hover);
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
        highAir: boolean;
        opacity: number;
        /** It carries riders' cards under its own panel. */
        riders: boolean;
      }[] = [];
      for (const c of calloutsRef.current) {
        const node = nodes.current.get(c.key);
        if (!node) continue;
        const p = anchorOf(c, drawn);
        // A panel hangs off a unit in front of the eye; the layout hides one
        // near the viewport's edge.
        const at = project(p[0], p[1], p[2] + (c.owner === "contact" ? 0 : HEAD_M));
        node.style.display = at ? "flex" : "none";
        if (at)
          anchored.push({
            id: c.key,
            node,
            selected: c.selected,
            riders: !!c.passengers,
            highAir: c.highAir,
            opacity: c.opacity ?? 1,
            x: at[0],
            y: at[1],
            distanceSq: vec3.squaredDistance(
              _readout_eye,
              vec3.set(_readout_anchor, p[0], p[1], p[2]),
            ),
          });
        else unplace(c.key);
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
      const panels = [...document.querySelectorAll("[data-occludes-readouts]")].map((e) => {
        const r = e.getBoundingClientRect();
        return { x0: r.left, x1: r.right, y0: r.top, y1: r.bottom };
      });
      const callouts = anchored.map((a) => ({
        ...a,
        w: a.node.offsetWidth,
        h: a.node.offsetHeight,
      }));
      const gap = gameHud.panel_gap_px;
      const edge = gameHud.panel_edge_hide_px;
      const right = window.innerWidth - edge;
      const bottom = window.innerHeight - edge;
      const layout = layoutReadouts(
        callouts,
        panels,
        { width: window.innerWidth, height: window.innerHeight },
        { gap, edge },
        overlap,
      );
      for (const b of callouts) {
        const at = layout.get(b.id);
        if (!at) {
          b.node.style.display = "none";
          unplace(b.id);
          continue;
        }
        const { natural, box: target } = at;
        // This one is drawn eased toward its target.
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
      const viewport: Box2 = [edge, edge, right, bottom];
      compactBoxes.current.clear();
      for (const { b, box } of situated) compactBoxes.current.set(b.id, box);
      const details = new Map<string, DetailPlacement>();
      // Each plane's cards open into their full detail among themselves.
      if (detailing && !overlap)
        for (const high of [false, true]) {
          const detailCards: DetailCard[] = situated
            .filter(({ b }) => b.highAir === high)
            .map(({ b, box }) => {
              const size = fullSizes.get(b.id)!;
              return {
                id: b.id,
                anchor: [b.x, b.y],
                distanceSq: b.distanceSq,
                compact: [box.x0, box.y0, box.x1, box.y1],
                full: [box.x0, box.y0, box.x0 + size.w, box.y0 + size.h],
              };
            });
          const obstacles: Box2[] = [
            ...panels.map((r): Box2 => [r.x0, r.y0, r.x1, r.y1]),
            ...callouts
              .filter((b) => b.highAir === high)
              .map(
                (b): Box2 => [
                  b.x - READOUT_ANCHOR_PX,
                  b.y - READOUT_ANCHOR_PX,
                  b.x + READOUT_ANCHOR_PX,
                  b.y + READOUT_ANCHOR_PX,
                ],
              ),
          ];
          const plane = layoutReadoutDetails(
            detailCards,
            obstacles,
            viewport,
            gap,
            expanded,
            hoveredCard.current,
          );
          for (const [id, at] of plane) details.set(id, at);
        }
      paintOrder.current = readoutPaintOrder(
        situated.map(({ b }) => b),
        hoveredCard.current,
        overlap,
      );
      for (const { b } of situated)
        b.node.parentElement!.style.zIndex = String(paintOrder.current.get(b.id));
      const drawnBoxes = new Map<string, ReadoutRect>();
      for (const { b, box: compact } of situated) {
        const detail = details.get(b.id);
        let box = detail
          ? { x0: detail.box[0], y0: detail.box[1], x1: detail.box[2], y1: detail.box[3] }
          : compact;
        // In a far view every card opens where it stands, off the edge.
        b.node.dataset.zoom = (overlap ? expanded || b.id === hoveredCard.current : detail?.full)
          ? "default"
          : "compressed";
        if (overlap && b.node.dataset.zoom === "default") {
          const size = fullSizes.get(b.id)!;
          const x0 = clamp(box.x0, edge, right - size.w);
          const y0 = clamp(box.y0, edge, bottom - size.h);
          box = { x0, y0, x1: x0 + size.w, y1: y0 + size.h };
        }
        drawnBoxes.set(b.id, box);
        b.node.style.transform = `translate(${box.x0}px, ${box.y0}px)`;
        const group = b.node.parentElement!;
        if (b.id === hoveredCard.current) group.dataset.hovered = "true";
        else delete group.dataset.hovered;
        // The leader: a small ring on the unit, a line off it to the
        // callout's near corner, then along its edge nearest the unit.
        const [from, corner, foot] = leaderOf(b, underlined(b, box));
        const r = LEADER_RING_PX;
        const xy = (p: [number, number]) => `${p[0].toFixed(1)} ${p[1].toFixed(1)}`;
        leaders.current
          .get(b.id)
          ?.setAttribute(
            "d",
            `M ${b.x - r} ${b.y.toFixed(1)} a ${r} ${r} 0 1 0 ${2 * r} 0 a ${r} ${r} 0 1 0 ${-2 * r} 0 ` +
              `M ${xy(from)} L ${xy(corner)} L ${xy(foot)}`,
          );
      }
      shadeOverlaps(
        situated.map(({ b }) => b),
        drawnBoxes,
        overlap,
      );
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
  const card = (c: Callout) => (
    <div
      key={c.key}
      className={`ro-callout${c.selected ? " ro-selected" : ""}`}
      style={{ opacity: c.opacity }}
      data-retiring={c.retiring || undefined}
      data-ghost={c.ghost || undefined}
    >
      <PanelCallout
        ref={bind(nodes.current, c.key)}
        owner={c.owner}
        selected={c.selected}
        data-zoom="compressed"
        data-unit={sideUnit(c.owner) && !c.ghost ? c.id : undefined}
        data-enemy={c.owner === "enemy" ? c.id : undefined}
        data-contact={c.owner === "contact" ? c.id : undefined}
      >
        {c.content}
        {c.passengers && <PassengerCards cards={c.passengers} />}
      </PanelCallout>
      <svg
        className={`ro-leaders${c.owner === "enemy" || c.owner === "contact" ? " ro-enemy" : c.owner === "ally" ? " ro-ally" : ""}`}
        aria-hidden="true"
      >
        <path
          ref={bind(leaders.current, c.key)}
          className={`ro-leader ro-${c.owner}${c.selected ? " ro-selected" : ""}`}
        />
      </svg>
    </div>
  );
  return (
    <div ref={layer} className="ro-layer" data-testid="readouts" data-zoom="compressed">
      <div className="ro-plane">{callouts.filter((c) => !c.highAir).map(card)}</div>
      <div className="ro-high-models" {...{ [HIGH_PLANE_ATTR]: "" }} />
      <div className="ro-plane" data-high-air>
        {callouts.filter((c) => c.highAir).map(card)}
      </div>
    </div>
  );
}
