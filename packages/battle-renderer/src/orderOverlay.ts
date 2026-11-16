// Order presentation (D2, D2+): the one Space view, drawn for each unit the
// reveal shows (`OrderView.reveal`: every own unit with Space held, the units
// of an order just given as it flashes and fades) at that opacity: remaining
// routes draped on the walkable surface, the final marker with its facing
// (Total War style), queued waypoints, a blocked route's mark, each
// soldier's resolved spot, a marker under each soldier's and vehicle's
// current position and a cover pip on each soldier's marker (the cover he
// has now at his current marker, the cover his spot gives at the final one).
// Apart from them, whatever the reveal, the selection's own markers under
// each selected unit: they are not order marks. Built only from the
// observing side's own-unit view.
//
// It is drawn as a holo-tactical projection: thin lines sized on screen
// (`OrderStyle.line_px` at the camera's target, thinned as the camera pulls
// out by the one stroke rule, `strokeWidth.ts`), no filled discs, all of it
// paint on the ground but for an aircraft's drop
// line, which stands in the air in the overlay (D18), glowing past its colour's
// full value by its role's `OrderStyle.glow`. Every ground unit's circle, under it or at
// its destination, is one marker, "the unit plus its facing" (`unitMarker`):
// a circle with a small filled arrowhead on its rim (the user's pick). A
// vehicle's lies under its hull; a squad's is the ring round its soldiers
// while it moves and its area ring while it holds, its arrowhead at the
// squad's heading or facing; the destination's points the final facing, and
// the route runs between the two circles' edges. Each soldier's marker is a
// plain ring holding his cover pip. A moving vehicle's travel shows as two
// chevrons behind it, against its facing on a reverse move, with a pulse
// marching along them. Colour carries meaning only where a player needs it:
// the cover tiers, a blocked route, and the selection.
import {
  concatMeshes,
  fadeAlpha,
  glowing,
  MeshBuilder,
  paintOnly,
  type Mesh,
  type Rgba,
} from "./mesh";
import { SOLDIER_R, unitCircle, destinationCircle, type UnitCircle } from "./unitCircle";
import {
  CHEVRON_M,
  along,
  circleMarker,
  crossMark,
  marchChevron,
  orderPen,
  pip,
  ring,
  routeBetween,
  unloadMark,
  type Pen,
} from "./orderPen";
import {
  currentAim,
  currentAttack,
  currentUnloads,
  ordersEnd,
  queuedLegs,
  warningRing,
  type OrderStep,
} from "./orderSteps";
import type { WorldMeshes } from "./scene";
import type { StrokeWidth } from "./strokeWidth";

type CoverTierName = "light" | "medium" | "heavy";

/** `presentation.overlay.orders`: every colour and width the orders draw. */
export interface OrderStyle {
  /** A route's width on screen at the camera's target. */
  line_px: number;
  /** The width of a marker's outline. */
  mark_px: number;
  /** A soldier's own markers (under him and at his spot) keep a finer
   *  weight: his outline, and a selected soldier's. */
  soldier_mark_px: number;
  soldier_line_px: number;
  /** Every route and marker, whatever the order's kind. */
  color: Rgba;
  /** Queued waypoints and the current markers are the colour at these
   *  alphas. */
  queued_alpha: number;
  current_alpha: number;
  /** A route the unit can't take: the HUD's one "can't" colour. */
  blocked: Rgba;
  /** An attack's line and reticle. */
  attack: Rgba;
  /** Half the side of an attack's reticle on its target. */
  reticle_m: number;
  /** A selected unit's own marker (its circle and arrowhead) and its
   *  soldiers' markers. */
  selected: Rgba;
  /** The marker of the unit under the pointer: the colour of its card. */
  hovered: Rgba;
  /** An ally's unit's marker: the HUD's allied colour (D24). */
  ally: Rgba;
  /** How far each role's paint glows past its colour's full value (up to 2,
   *  the paint's range): the order marks (and a blocked route), and the
   *  selection's. Tuned apart, since the tone mapper pulls each hue toward
   *  white at its own rate. */
  glow: { order: number; selected: number };

  /** The cover pips' paint glows this much past their colour's full value:
   *  a dark green pip on grass still reads. */
  cover_glow: number;
  /** Cover pips by tier (D2+). */
  cover: Record<CoverTierName, Rgba>;
  /** A cover pip's radius. */
  cover_pip_m: number;
  /** The pulse along a moving vehicle's travel chevrons, on the frame's
   *  presentation clock: cycles a second, and how far it dims them between
   *  beats (0 steady, 1 to nothing). */
  march: { cycles_per_s: number; amplitude: number };
  /** A squad's circles are drawn at this fraction of their radius. */
  area_draw_scale: number;
  /** How far a vehicle's marker circle (under it and at its destination)
   *  reaches past its hull's half-length (`OrderView.hullHalfLength`): the
   *  ring and its arrowhead peek out from under any hull, a jeep's or a
   *  tank's, from the kind's own footprint. */
  vehicle_marker_margin_m: number;
  /** How far a garrisoned squad's circle reaches past its building's
   *  corners (`OrderView.building`): the ring encloses the whole house. */
  building_marker_margin_m: number;
  /** How far a held building's outline (`heldOutline`) lies outside
   *  each part's walls, so the paint peeks out round the shell. */
  garrison_outline_margin_m: number;
  /** An aircraft's drop line (D18), dots rising from its marker's centre
   *  on the ground toward its airframe: a dot's size on screen, by the
   *  stroke rule like every mark's; the pitch from one dot to the next, in
   *  dot sizes; its opacity at the ground, whatever its marker's; the
   *  share of the height it holds that opacity to; and the share it has
   *  faded out by, short of the airframe, so it points up at it rather than
   *  tying on. Its colour is the marker's at full brightness, so it reads
   *  as the ring's glowing paint does. */
  drop_line_px: number;
  drop_line_spacing: number;
  drop_line_alpha: number;
  drop_line_hold: number;
  drop_line_reach: number;
}

interface MemberOrderMark {
  spot: readonly [number, number];
  coverNow: CoverTierName | null;
  coverThere: CoverTierName | null;
}

export interface OrderView {
  position: readonly [number, number, number];
  goal: readonly [number, number] | null;
  state: string;
  route: readonly (readonly [number, number])[];
  /** Its orders, the current one first (`OrderStep`). */
  steps: readonly OrderStep[];
  /** Living soldiers' positions (empty for a vehicle). */
  members: readonly (readonly [number, number, number])[];
  /** Each living soldier's resolved spot and cover, in `members` order. */
  memberOrders: readonly MemberOrderMark[];
  /** The bearing the unit ends its move at. */
  finalFacing: number;
  /** The bearing it faces now (its published yaw). */
  yaw: number;
  /** "reverse" on a reverse move (Q31). */
  direction: string | null;
  /** A squad's area round its anchor: the disc its soldiers take
   *  their spots and fight in, which only an order moves; null for a
   *  vehicle. */
  area: { anchor: readonly [number, number]; radius: number } | null;
  /** A vehicle's hull half-length (its footprint's, `physics`); 0 for a
   *  squad. Its marker circle is this plus `vehicle_marker_margin_m`. */
  hullHalfLength: number;
  /** The building a garrisoned squad holds (inside or leaving): its
   *  footprint's centre and half extents; null otherwise. The squad's circle
   *  encloses it, `building_marker_margin_m` past its corners. */
  building: { center: readonly [number, number]; half: readonly [number, number] } | null;
  /** Its circle is a house's ring another occupant's mark paints: the
   *  circle still meets its routes, but is not painted again. */
  ringShared?: boolean;
  /** It flies on wings: it never stops over a point, so it has no ground
   *  marker, drop line or destination circle. */
  fixedWing?: boolean;
  /** Any airborne unit, including rotor aircraft that keep their marker. */
  airborne?: boolean;
  /** How high its ghost stands over where its orders end while one shows
   *  (an aircraft's cruise height, Space held): a drop line joins the ghost
   *  to that end's marker, as one joins the airframe to its own. 0 or
   *  absent: no ghost, no line. */
  ghostAloft?: number;
  /** In the player's selection: the marker under it is `selected`'s. */
  selected?: boolean;
  /** Under the pointer (its model or its card): its marker shows as a
   *  selected one's does, in the style's `hovered` colour; selected, it keeps
   *  the selection's. */
  hovered?: boolean;
  /** Another player on the side owns it (an ally's): its marker is a
   *  broken ring with no facing, in the style's `ally` colour, and none of
   *  its orders show. */
  ally?: boolean;
  /** The opacity its order marks (the Space view) draw at, in [0, 1]: 0 or
   *  absent draws none, and a selected unit then shows only the selection's
   *  own markers. */
  reveal?: number;
}

interface OrderOverlayOptions {
  /** The stroke widths at the camera's zoom (`strokeWidth`): turns the
   *  style's pixel widths (`line_px`, `mark_px`, ...) into metres. */
  stroke: StrokeWidth;
}

/** Height of the walkable surface (bridge deck where one spans). */
export type SurfaceHeight = (x: number, y: number) => number;

/** How far outside a vehicle's marker its travel chevrons start. */
const CHEVRONS_GAP_M = 1;
type P2 = readonly [number, number];

/** An airframe this close to the ground under it draws no drop line. */
const DROP_MIN_M = 0.5;
/** The most dots a drop line draws: past it, a close camera spaces them wider. */
const DROP_MAX_DOTS = 48;

/** Where an aircraft is over the ground (D18): its marker on the ground
 *  under it (paint, in `paint`), and a dotted drop line rising from the
 *  marker's centre toward the airframe (`top`, its published position). The
 *  dots stand in the air, so they are overlay geometry (in `line`),
 *  depth-tested like every overlay: a roof under the airframe hides the
 *  ones below it, and the marker hides under the roof as paint on the
 *  ground does. */
function aircraftMarker(
  paint: MeshBuilder,
  line: MeshBuilder,
  pen: Pen,
  top: readonly [number, number, number],
  marker: UnitCircle,
  color: Rgba,
) {
  circleMarker(paint, pen, marker, color);
  dropLine(line, pen, top, color);
}

/** `color` at full brightness: its brightest channel at 1, its hue kept. */
export function fullBright(color: Rgba): [number, number, number] {
  const k = 1 / Math.max(color[0], color[1], color[2], 1e-6);
  return [color[0] * k, color[1] * k, color[2] * k];
}

/** A drop line of dots rising from the ground under `top` toward it, in
 *  `color` at full brightness: `drop_line_alpha` from the ground up to
 *  `drop_line_hold` of the height, then fading out by `drop_line_reach` of
 *  it. Under an airframe or its ghost. */
function dropLine(
  line: MeshBuilder,
  pen: Pen,
  top: readonly [number, number, number],
  color: Rgba,
) {
  const ground = pen.z(top[0], top[1]);
  const height = top[2] - ground;
  if (height < DROP_MIN_M) return;
  const { drop_line_alpha: alpha, drop_line_hold: hold, drop_line_reach: reach } = pen.style;
  const span = height * reach;
  const dot = Math.min(pen.drop, span / 2);
  const pitch = Math.max(dot * pen.style.drop_line_spacing, span / DROP_MAX_DOTS);
  const dots = Math.max(2, Math.floor((span - dot) / pitch) + 1);
  const [r, g, b] = fullBright(color);
  // Past the hold, each dot fades by how far it stands toward the reach;
  // the last is the faintest, never quite gone: the next would be.
  const fadeFrom = height * hold;
  const fadeOver = span - fadeFrom + pitch;
  for (let i = 0; i < dots; i++) {
    const z = ground + dot / 2 + (i / (dots - 1)) * (span - dot);
    const fade = Math.min(1, 1 - (z - ground - fadeFrom) / fadeOver);
    line.mark([top[0], top[1], z], dot / 2, [r, g, b, alpha * fade]);
  }
}

/** A soldier's marker: a plain ring (the unit's marker carries the facing),
 *  with a cover tier's pip (in `pipColor`) in its middle when he has cover
 *  and the order marks show. */
function soldierMark(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  pipColor: Rgba | null,
  color: Rgba,
  width = pen.soldier,
) {
  ring(mesh, pen, c, SOLDIER_R, color, { width });
  if (pipColor) pip(mesh, pen, c, pipColor);
}

/** The way a moving vehicle travels, as two small chevrons behind its marker
 *  (and its hull), painted on the ground like it: pointing along the marker's facing on a forward move, and
 *  against it on a reverse move (Q31), so a reversing vehicle reads at a
 *  glance. Drawn only under the moving unit, never at its destination. They
 *  go into the `animated` mesh: a pulse runs along them in the direction of
 *  travel (`OrderStyle.march`, on the frame's presentation clock). Each
 *  vertex's normal carries (phase, cycles a second, amplitude) for the
 *  overlay pass. */
function travelChevrons(
  mesh: MeshBuilder,
  pen: Pen,
  c: P2,
  radius: number,
  facing: number,
  reverse: boolean,
  color: Rgba,
) {
  const back = facing + Math.PI;
  const travel = reverse ? back : facing;
  const { cycles_per_s, amplitude } = pen.style.march;
  // Nearest the marker first; the pulse reaches the one ahead in the
  // direction of travel later.
  const start = radius + CHEVRONS_GAP_M;
  const spots = [start, start + 1.3];
  spots.forEach((d, k) => {
    const lead = reverse ? k : spots.length - 1 - k;
    const normal = [lead * 0.25, cycles_per_s, amplitude] as const;
    const tip = along(c, back, d - (reverse ? 0 : CHEVRON_M[0]));
    marchChevron(mesh, pen, tip, travel, color, normal);
  });
}

export interface DestinationMarker extends UnitCircle {
  placed: boolean;
  opacity: number;
  /** The order lets passengers out here: the unload mark. */
  unload?: boolean;
}

/** A destination and its facing while the player holds a right-drag. */
export function buildDestinationPreview(
  marks: readonly DestinationMarker[],
  z: SurfaceHeight,
  style: OrderStyle,
  { stroke }: OrderOverlayOptions,
) {
  const mesh = new MeshBuilder();
  const pen = orderPen(z, style, stroke);
  for (const mark of marks) {
    if (!mark.placed) continue;
    const color = fadeAlpha(glowing(style.color, style.glow.order), mark.opacity);
    circleMarker(mesh, pen, mark, color);
    if (mark.unload) unloadMark(mesh, pen, mark, mark.facing ?? 0, color);
  }
  return mesh.build();
}

/** The marker of what the pointer is over that is not yet a unit (a
 *  purchase being placed): its circle in the hover's colour, as a hovered
 *  unit's is drawn. */
export function buildHoverMarker(
  circle: UnitCircle,
  z: SurfaceHeight,
  style: OrderStyle,
  { stroke }: OrderOverlayOptions,
): Mesh {
  const mesh = new MeshBuilder();
  circleMarker(
    mesh,
    orderPen(z, style, stroke),
    circle,
    glowing(style.hovered, style.glow.selected),
  );
  return mesh.build();
}

/** An ally's unit's marker, selected (to read its panel) or aloft: form
 *  before colour, it is hollow where yours is whole. Its circle is a broken
 *  ring with no arrowhead, its soldiers' plain rings, an aircraft's drop
 *  line as yours; it draws nothing of its orders, which are not yours. */
function allyMarker(
  paint: MeshBuilder,
  soldiers: MeshBuilder,
  lines: MeshBuilder,
  pen: Pen,
  u: OrderView,
  color: Rgba,
) {
  const circle = unitCircle(u, pen.style);
  if (circle && !u.ringShared)
    ring(paint, pen, circle.c, circle.r, color, { width: pen.line, dashed: true });
  if (circle && u.airborne) dropLine(lines, pen, u.position, color);
  if (u.selected)
    for (const m of u.members)
      soldierMark(soldiers, pen, [m[0], m[1]], null, color, pen.soldierSelected);
}

/** A holding unit's soldiers walking to their posts: a marker at each post
 *  more than a body's width from where he stands. */
function soldierPosts(
  soldiers: MeshBuilder,
  pen: Pen,
  u: OrderView,
  pipOf: (cover: CoverTierName | null) => Rgba | null,
  color: Rgba,
) {
  u.memberOrders.forEach((m, k) => {
    const p = u.members[k];
    if (p && Math.hypot(m.spot[0] - p[0], m.spot[1] - p[1]) > 2 * SOLDIER_R)
      soldierMark(soldiers, pen, m.spot, pipOf(m.coverThere), color);
  });
}

/** An aircraft's ghost over where its orders end (`ghostAloft`), joined to
 *  that end's marker by a drop line: in `color` at its current goal, or
 *  `queued`'s when a queued step ends them. */
function ghostDrop(lines: MeshBuilder, pen: Pen, u: OrderView, color: Rgba, queued: Rgba) {
  const end = ordersEnd(u.steps, u.goal);
  if (!u.ghostAloft || !end) return;
  const top = [end[0], end[1], pen.z(end[0], end[1]) + u.ghostAloft] as const;
  dropLine(lines, pen, top, end !== u.goal ? queued : color);
}

/** A unit's circle where it stands, in `color`: an aircraft's marker and
 *  drop line, or its circle marker, unless a housemate paints its house's
 *  ring (`ringShared`). */
function standingCircle(
  paint: MeshBuilder,
  lines: MeshBuilder,
  pen: Pen,
  u: OrderView,
  circle: UnitCircle,
  color: Rgba,
) {
  if (u.airborne) aircraftMarker(paint, lines, pen, u.position, circle, color);
  else if (!u.ringShared) circleMarker(paint, pen, circle, color);
}

export function buildOrderOverlay(
  units: readonly OrderView[],
  z: SurfaceHeight,
  style: OrderStyle,
  { stroke }: OrderOverlayOptions,
): WorldMeshes {
  const pen = orderPen(z, style, stroke);
  // Every order mark is paint, still or marching (the travel chevrons),
  // each colour glowing past its full value. Soldiers' markers draw last, so
  // no other unit's ring or route crossing one covers his cover pip.
  const paint = new MeshBuilder();
  const soldiers = new MeshBuilder();
  const animated = new MeshBuilder();
  // Aircraft drop lines: overlay, the one mark standing in the air.
  const lines = new MeshBuilder();
  const order = glowing(style.color, style.glow.order);
  const selected = glowing(style.selected, style.glow.selected);
  const cannot = glowing(style.blocked, style.glow.order);
  const ally = glowing(style.ally, style.glow.selected);
  const hovered = glowing(style.hovered, style.glow.selected);
  for (const u of units) {
    if (u.ally) {
      // Hovered, an ally's unit shows its marker as selected: still an ally's.
      const lit = !!u.selected || !!u.hovered;
      allyMarker(
        paint,
        soldiers,
        lines,
        pen,
        { ...u, selected: lit },
        lit ? ally : fadeAlpha(ally, style.current_alpha),
      );
      continue;
    }
    // Its order marks (the Space view) at their opacity, or none; the
    // selection's own markers either way.
    const reveal = u.reveal ?? 0;
    const shown = reveal > 0;
    // Its own marker's colour while it is selected or hovered (selection
    // first), drawn then as a selected unit's; null otherwise.
    const highlight = u.selected ? selected : u.hovered ? hovered : null;
    if (!shown && !highlight && !u.airborne) continue;
    const fade = (c: Rgba): Rgba => fadeAlpha(c, reveal);
    const color = fade(order);
    const current = fade(fadeAlpha(order, style.current_alpha));
    const queued = fade(fadeAlpha(order, style.queued_alpha));
    const blockedColor = fade(cannot);
    const pipOf = (cover: CoverTierName | null | undefined) =>
      shown && cover ? fade(style.cover[cover]) : null;
    const moving = !!u.goal && u.state !== "route_blocked";
    const squad = u.members.length > 0;
    // The unit's marker: its circle and its soldiers' markers are one mark,
    // shown and hidden together (`unitCircle` decides when: moving, selected
    // or revealed), in the selection's colours when selected (the hover's
    // when hovered). The circle is a
    // squad's area ring round its anchor while it holds, the ring round its
    // soldiers while it moves, the ring round its building while it holds
    // one, or a vehicle's marker under its hull.
    const circle = unitCircle(u, style);
    // A squad with no circle to draw (no area) still shows its soldiers'
    // markers when the unit's marker would.
    if (circle || shown || highlight) {
      if (circle)
        standingCircle(
          paint,
          lines,
          pen,
          u,
          circle,
          highlight ??
            (u.airborne
              ? // An aircraft's marker shows whatever the reveal: at the current
                // markers' alpha, brighter only while a shown order moves it.
                fadeAlpha(
                  order,
                  moving ? Math.max(reveal, style.current_alpha) : style.current_alpha,
                )
              : moving
                ? color
                : current),
        );
      // Each soldier's marker, with his cover now when shown.
      const mark = highlight ?? current;
      u.members.forEach((m, k) =>
        soldierMark(
          soldiers,
          pen,
          [m[0], m[1]],
          pipOf(u.memberOrders[k]?.coverNow),
          mark,
          highlight ? pen.soldierSelected : pen.soldier,
        ),
      );
    }
    // A moving vehicle's travel shows as chevrons behind its hull.
    if (!squad && moving && circle)
      travelChevrons(
        animated,
        pen,
        circle.c,
        circle.r,
        u.finalFacing,
        u.direction === "reverse",
        highlight ?? current,
      );
    // The rest are order marks only.
    if (!shown) continue;
    // A holding squad's soldiers walking to their posts (cover, a step
    // out): each post is a marker only. An order is the unit's, one route
    // for the unit; no line ever runs from a soldier.
    if (!u.goal) soldierPosts(soldiers, pen, u, pipOf, color);
    const aim = currentAim(u);
    if (aim) {
      const attack = fade(glowing(style.attack, style.glow.order));
      const reticle = currentAttack(paint, pen, u, circle, aim, attack, color);
      queuedLegs(paint, pen, u, reticle, queued);
      ghostDrop(lines, pen, u, color, queued);
      continue;
    }
    if (!u.goal) continue;
    const blocked = u.state === "route_blocked";
    // The destination's circle marker: a squad's area ring (an order moves
    // its area, so a moving squad's is already the destination's), with
    // each soldier's spot in it; a vehicle's marker. Both point the final
    // facing.
    // A winged aircraft circles its goal rather than stopping on it facing
    // a bearing, so it has no destination circle.
    const dest: UnitCircle | null =
      blocked || u.fixedWing ? null : destinationCircle(u, u.goal, style);
    routeBetween(paint, pen, circle, u.route, dest, color);
    if (blocked) {
      const warning = warningRing(u.goal);
      routeBetween(paint, pen, circle, [u.goal], warning, blockedColor, { dashed: true });
      crossMark(paint, pen, u.goal, 6, blockedColor);
      ring(paint, pen, u.goal, warning.r, blockedColor, { width: pen.line });
    } else if (dest) {
      circleMarker(paint, pen, dest, color);
      if (currentUnloads(u.steps)) unloadMark(paint, pen, dest, u.finalFacing, color);
      if (squad)
        for (const m of u.memberOrders)
          soldierMark(soldiers, pen, m.spot, pipOf(m.coverThere), color);
    }
    queuedLegs(paint, pen, u, dest, queued);
    ghostDrop(lines, pen, u, blocked ? blockedColor : color, queued);
  }
  return {
    ...paintOnly(concatMeshes([paint.build(), soldiers.build()]), animated.build()),
    opaque: lines.build(),
  };
}

/** The side's view of an aircraft that isn't its own (an identified enemy):
 *  where it is, its facing and its hull's half length. */
export interface AircraftView {
  position: readonly [number, number, number];
  yaw: number;
  hullHalfLength: number;
  fixedWing?: boolean;
}

/** An identified aircraft's marker circle, on the ground under it: its
 *  hull's half length plus `vehicle_marker_margin_m`, as an own vehicle's. */
export function aircraftCircle(
  a: AircraftView,
  style: Pick<OrderStyle, "vehicle_marker_margin_m">,
): UnitCircle {
  return {
    c: [a.position[0], a.position[1]],
    r: a.hullHalfLength + style.vehicle_marker_margin_m,
    facing: a.yaw,
  };
}

/** Every one of `aircraft` as its ground marker and drop line
 *  (`aircraftMarker`), in `color` (its role's, glowing by the orders'). */
export function buildAircraftMarks(
  aircraft: readonly AircraftView[],
  color: Rgba,
  z: SurfaceHeight,
  style: OrderStyle,
  { stroke }: OrderOverlayOptions,
): WorldMeshes {
  const pen = orderPen(z, style, stroke);
  const paint = new MeshBuilder();
  const lines = new MeshBuilder();
  const glow = glowing(color, style.glow.order);
  for (const a of aircraft.filter((a) => !a.fixedWing))
    aircraftMarker(paint, lines, pen, a.position, aircraftCircle(a, style), glow);
  return { ...paintOnly(paint.build()), opaque: lines.build() };
}
