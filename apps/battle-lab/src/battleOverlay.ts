import type { ScenarioRules } from "./battleSessionOptions";
import { stepBuildings, type StepBuildingIndex } from "./stepBuildings";
// What one side's battle view draws from its own observation besides the
// static world and its structures (the session's fitted props), one layer
// per concern: contact glyphs, visible flight and strike marks, garrison
// occupants, guided missiles, supply reach, the selection's orders, the
// public objective's zone when the scenario has one, and the playable area's
// border (built by the view per zoom step, passed in). Ground marks are for
// selection and movement and extents; every unit state is its info panel's
// (`readouts.tsx`). The one held-building outline is an extent the user
// asked for: which buildings a garrison holds, in the holder's side colour.
// The battle view composes the contacts, supply, orders, aircraft, held
// buildings, zone and border (an aircraft's ground marker and drop line, own or enemy, D18):
// combat effects (`effects/`) draw the flight, the flashes, the impacts and a
// missile's flare and smoke trail, and one ring round a house says where its
// garrison holds. The same outline marks the building a garrison order under
// the pointer would enter (`garrisonTargetOutline`, the pointer's paint).
// Labs compose the layers their fixture exercises, the rest among them as
// diagnostics.
import type { StrokeWidth } from "@packages/battle-renderer/src/strokeWidth";
import { gameHud } from "@web/battle/present/hudTheme";
import type { SideName } from "@web/battle/sim/protocol";
import { airborne, fixedWing, type UnitCatalog } from "@packages/scene-assets/src/units";
import { buildContactGlyphs, type ContactFacing } from "@packages/battle-renderer/src/contactGlyph";
import { buildFlightOverlay } from "@packages/battle-renderer/src/flightMesh";
import { buildConsequenceOverlay } from "@packages/battle-renderer/src/consequenceOverlay";
import {
  buildGarrisonOutlines,
  buildGarrisonOverlay,
  type HeldBuilding,
} from "@packages/battle-renderer/src/garrisonOverlay";
import type { BuildingIndex } from "@packages/battle-renderer/src/models/buildingReferences";
import { buildGuidanceOverlay } from "@packages/battle-renderer/src/guidanceOverlay";
import { buildSupplyOverlay } from "@packages/battle-renderer/src/supplyOverlay";
import {
  buildAircraftMarks,
  buildOrderOverlay,
  type AircraftView,
  type OrderView,
  type SurfaceHeight,
} from "@packages/battle-renderer/src/orderOverlay";
import {
  buildDeploymentMarker,
  type EntryMarker,
} from "@packages/battle-renderer/src/deploymentMarker";
import {
  combineWorldMeshes,
  concatMeshes,
  unpulsed,
  type Mesh,
  type Rgba,
} from "@packages/battle-renderer/src/mesh";
import { buildZoneCurtain, buildZoneRing } from "@packages/battle-renderer/src/playAreaOverlay";
import type { WorldMeshes } from "@packages/battle-renderer/src/scene";
import type { PresentedContact } from "@web/battle/present/contactPresentation";
import type { EntryLane } from "@web/battle/prepare/lanes";
import {
  aboardCarrier,
  inBuilding,
  type IdentifiedView,
  type ObservationView,
  type OwnUnitView,
} from "@web/battle/sim/observationView";
import type { RevealedOrders } from "@web/battle/present/orderReveal";
import { objectiveStanding, type ObjectiveStanding } from "@web/battle/present/objectiveMarkers";
import { gameContactStyle } from "./gameFog";
import { ghostAloft } from "./unitGhosts";
import {
  OPENING_METRES_PER_PX,
  gameOrderStyle,
  gameStroke,
  gameSupplyStyle,
  gameZone,
  gameCaptureCurtain,
} from "./gameOverlay";

type P3 = [number, number, number];
// The labs' own marks: tracers (the battle draws flight as effects) and
// where a round struck.
const OWN_TRACER = [0.98, 0.97, 0.9, 1] as const;
const ENEMY_TRACER = [1.0, 0.45, 0.4, 1] as const;
const IMPACT = [1.0, 0.86, 0.35, 1] as const;
/** A capture area's ring by its standing to the viewer, in its flag's colour;
 *  nobody's is the zone's neutral edge. */
const STANDING_RING: Record<ObjectiveStanding, Rgba> = {
  own: [...gameHud.accent, 1],
  enemy: [...gameHud.enemy, 1],
  neutral: gameZone,
};

/** What the overlay draws from the scenario itself. */
export interface BattleOverlayScenario {
  /** The side whose view this is; a watching client reads as Blue. */
  side: SideName;
  supplyRadius: number;
  zone: { center: readonly [number, number]; radius: number } | null;
  /** The viewer's side's roads in, each pointing into the map. */
  deployments: EntryMarker[];
}

/** The parts of the scenario the overlay reads (the view parses it once). */
export interface OverlayScenarioSource {
  map: { size: [number, number] };
  rules: { service: { radius_m: number } };
  /** Absent or null when the scenario has no completion rule. */
  encounter?: { success_zone_center: [number, number]; success_zone_radius_m: number } | null;
  skirmish?: {
    sites: { entries: { side: SideName; center: [number, number]; yaw: number }[] };
  } | null;
}

/** The scenario's own marks as `side` sees them. A side's players share
 *  its roads in, and a purchase may arrive by any of them, so each one
 *  carries the reinforcement cue, numbered by its lane (`lanes`, one per
 *  entry in list order, empty with no skirmish). */
export function battleOverlayScenario(
  s: OverlayScenarioSource,
  lanes: readonly EntryLane[],
  side: SideName,
): BattleOverlayScenario {
  return {
    side,
    supplyRadius: s.rules.service.radius_m,
    zone: s.encounter
      ? { center: s.encounter.success_zone_center, radius: s.encounter.success_zone_radius_m }
      : null,
    // Each marker faces the way its road runs in, as the entry does.
    deployments: (s.skirmish?.sites.entries ?? []).flatMap(({ side: entrySide, center, yaw }, i) =>
      entrySide === side ? [{ center, facing: yaw, lane: lanes[i].lane }] : [],
    ),
  };
}

/** What the view remembers between frames: recent strikes and missile paths. */
export class BattleMemory {
  impacts: { at: P3; tick: number }[] = [];
  trails = new Map<number, P3[]>();
  /** How long a strike mark stays (ticks), fading out. */
  readonly impactTicks: number;
  private readonly ownImpactsOnly: boolean;

  constructor({ impactTicks = 60, ownImpactsOnly = false } = {}) {
    this.impactTicks = impactTicks;
    this.ownImpactsOnly = ownImpactsOnly;
  }

  /** Fold in one decoded frame. */
  note(o: ObservationView) {
    this.impacts = this.impacts.filter((i) => o.tick - i.tick < this.impactTicks);
    for (const p of o.projectiles)
      if (p.hit !== "none" && (p.own || !this.ownImpactsOnly))
        this.impacts.push({ at: p.path[p.path.length - 1], tick: o.tick });
    const flying = new Set(o.guided.map((g) => g.id));
    for (const id of this.trails.keys()) if (!flying.has(id)) this.trails.delete(id);
    for (const g of o.guided) {
      const trail = this.trails.get(g.id) ?? [];
      trail.push(g.position);
      this.trails.set(g.id, trail.slice(-240));
    }
  }

  clear() {
    this.impacts = [];
    this.trails.clear();
  }
}

/** Glyphs consume the same presented reports as their labels; an aloft
 *  one's sign faces the camera as `facing` says (`useContactFacing`). */
export function contactLayer(
  contacts: readonly PresentedContact[],
  z: SurfaceHeight,
  facing: ContactFacing,
): WorldMeshes {
  return buildContactGlyphs(contacts, z, gameContactStyle, facing);
}

/** This tick's visible flight, own and enemy rounds tinted apart (or all in
 *  the neutral flying colour when `sideColors` is off). */
export function tracerLayer(o: ObservationView, { sideColors = true } = {}): WorldMeshes {
  return buildFlightOverlay(
    o.projectiles.map((p) => ({
      points: p.path,
      outcome: "flying" as const,
      color: sideColors ? (p.own ? OWN_TRACER : ENEMY_TRACER) : undefined,
    })),
    [],
    [],
    0.3,
  );
}

/** Recent strike marks from `memory`. The fallen themselves are drawn by
 *  the models layer. */
export function remainsLayer(
  o: ObservationView,
  memory: BattleMemory,
  z: SurfaceHeight,
): WorldMeshes {
  return buildConsequenceOverlay(
    memory.impacts.map((i) => ({
      at: i.at,
      fade: 1 - (o.tick - i.tick) / memory.impactTicks,
    })),
    z,
    IMPACT,
  );
}

/** Occupant pads of squads holding buildings. */
export function garrisonLayer(o: ObservationView, z: SurfaceHeight): WorldMeshes {
  return buildGarrisonOverlay(
    o.own.flatMap((u) => (inBuilding(u) ? [{ members: u.members, phase: u.hold!.phase }] : [])),
    z,
  );
}

/** The map building (its index in `buildings`) that prop `id` is part of:
 *  any part's prop, or a remains prop whose authored source is one. */
function buildingOfProp(
  o: Pick<ObservationView, "knownProps">,
  buildings: Pick<BuildingIndex, "partBuilding">,
  id: number,
): number | undefined {
  return (
    buildings.partBuilding.get(id) ??
    buildings.partBuilding.get(o.knownProps.find((k) => k.id === id)?.authoredProp ?? -1)
  );
}

/** The outline of the building prop `id` belongs to, in the side's own
 *  colour, as a garrison it holds would draw it: the building an order
 *  under the pointer would garrison. Null when it is no known building. */
export function garrisonTargetOutline(
  o: Pick<ObservationView, "knownProps">,
  buildings: Pick<BuildingIndex, "partBuilding" | "parts">,
  id: number,
  line: number,
  z: SurfaceHeight,
): Mesh | null {
  const i = buildingOfProp(o, buildings, id);
  return i === undefined
    ? null
    : buildGarrisonOutlines(
        [{ parts: buildings.parts[i], color: [...gameHud.accent, 1] }],
        line,
        gameOrderStyle.garrison_outline_margin_m,
        z,
      );
}

/** The buildings the side knows a garrison holds (inside or leaving): its
 *  own squads' in its own colour, and an identified enemy's in the enemy's,
 *  while it is seen (`buildingOfProp` finds each); the outline traces the
 *  map's parts, one outline per building however many squads hold it. */
export function heldBuildingsLayer(
  o: ObservationView,
  buildings: Pick<BuildingIndex, "partBuilding" | "parts">,
  line: number,
  z: SurfaceHeight,
): Partial<WorldMeshes> {
  // Each building once, by its index; one side holds a building at a time.
  const marks = new Map<number, HeldBuilding>();
  const held = (id: number, color: Rgba) => {
    const i = buildingOfProp(o, buildings, id);
    if (i !== undefined && !marks.has(i)) marks.set(i, { parts: buildings.parts[i], color });
  };
  const own: Rgba = [...gameHud.accent, 1];
  const enemy: Rgba = [...gameHud.enemy, 1];
  for (const u of o.own)
    if (u.hold?.host.kind === "building" && u.hold.phase !== "entering") held(u.hold.host.id, own);
  for (const e of o.identified) if (e.building !== null) held(e.building, enemy);
  return marks.size
    ? {
        painted: buildGarrisonOutlines(
          [...marks.values()],
          line,
          gameOrderStyle.garrison_outline_margin_m,
          z,
        ),
      }
    : {};
}

/** Own guided missiles with the paths `memory` has kept. */
export function guidanceLayer(
  o: ObservationView,
  memory: BattleMemory,
  z: SurfaceHeight,
): WorldMeshes {
  return buildGuidanceOverlay(
    o.guided.map((g) => ({ ...g, trail: memory.trails.get(g.id) ?? [] })),
    z,
  );
}

/** The reach (`radius` metres) of each stocked supply vehicle (a unit
 *  with stock) in `selected`, or of every one with `all` (Space held).
 *  Nothing otherwise; whether a truck is set up and supplying, and whether a
 *  unit is served, are their info panels'. */
export function supplyLayer(
  o: ObservationView,
  radius: number,
  z: SurfaceHeight,
  selected: readonly number[],
  stroke = gameStroke(OPENING_METRES_PER_PX),
  all = false,
): WorldMeshes {
  return buildSupplyOverlay(
    o.own
      .filter((u) => (all || selected.includes(u.id)) && u.stock !== null && u.stock > 0)
      .map((u) => ({ center: [u.position[0], u.position[1]], radius })),
    z,
    stroke(gameOrderStyle.line_px),
    gameSupplyStyle,
  );
}

/** An own unit as the orders draw it: selected or not, its order marks
 *  shown at `reveal` (0: none), an ally's or the player's, hovered or not. */
export function orderView(
  units: UnitCatalog,
  u: OwnUnitView,
  selected: boolean,
  reveal = 0,
  ally = false,
  hovered = false,
): OrderView {
  const g = u.hold;
  return {
    ...u,
    // Its footprint sizes its marker: a hull's half length, 0 for a squad;
    // a garrisoned squad's, its building's (inside or leaving, not entering).
    hullHalfLength: units.hull(u.kind)?.half_extents_m[0] ?? 0,
    fixedWing: fixedWing(units.type(u.kind)),
    airborne: airborne(units.type(u.kind)),
    building:
      g?.host.kind === "building" && g.phase !== "entering"
        ? { center: g.host.center, half: g.host.half }
        : null,
    selected,
    hovered,
    reveal,
    ally,
  };
}

/** The selection's markers under the units in `selected` (and the hovered
 *  unit's under `hovered`), the order
 *  marks (the Space view) of each unit `reveal` shows, at its opacity
 *  (`OrderReveal`), with an unload mark where a carrier's published
 *  order lets its passengers out, and every own aircraft's marker and drop
 *  line; a unit in `allies` (an ally's) takes the ally's marker. With
 *  `ghosts` (Space held: each revealed unit's ghost stands where its orders
 *  end) an aircraft's ghost is joined to that end's marker by its drop
 *  line too. Lines are the style's pixel widths resolved by the
 *  stroke rule (the opening camera's scale for a view that doesn't follow
 *  its camera). */
export function orderLayer(
  units: UnitCatalog,
  air: ScenarioRules["air"],
  o: ObservationView,
  selected: readonly number[],
  reveal: RevealedOrders,
  z: SurfaceHeight,
  {
    stroke = gameStroke(OPENING_METRES_PER_PX),
    ghosts = false,
    allies = new Map(),
    hovered = null,
    buildings = null,
  }: {
    stroke?: StrokeWidth;
    ghosts?: boolean;
    allies?: ReadonlyMap<number, unknown>;
    hovered?: number | null;
    /** The map's buildings: a queued garrison or exit rings its building. */
    buildings?: StepBuildingIndex | null;
  } = {},
): WorldMeshes {
  const aloft = ghostAloft(units, air);
  const shown = o.own
    // A squad a carrier holds has no marks of its own: it is off the field.
    .filter((u) => !aboardCarrier(u))
    .filter(
      (u) =>
        selected.includes(u.id) ||
        u.id === hovered ||
        reveal.has(u.id) ||
        airborne(units.type(u.kind)),
    );
  const views = shown.map((u) => ({
    ...orderView(
      units,
      u,
      selected.includes(u.id),
      reveal.get(u.id),
      allies.has(u.id),
      u.id === hovered,
    ),
    ghostAloft: ghosts && reveal.has(u.id) ? aloft(u.kind) : 0,
    steps: stepBuildings(u, buildings),
  }));
  return buildOrderOverlay(oneRingPerHouse(shown, views), z, gameOrderStyle, { stroke });
}

/** How brightly an occupant's marker reads: selected, then hovered, then
 *  the player's own over an ally's, then a moving one, then by its
 *  reveal's opacity. */
const ringRank = (v: OrderView) =>
  (v.selected ? 8 : 0) +
  (v.hovered ? 4 : 0) +
  (v.ally ? 0 : 2) +
  (v.goal && v.state !== "route_blocked" ? 1 : 0) +
  (v.reveal ?? 0) / 2;

/** One ring per house: of the occupants whose circle is their building's
 *  ring (inside or leaving), keyed by the building's structure owner (the
 *  host's id), only the brightest (`ringRank`) paints it; the others keep it
 *  as the circle their routes leave from. `views` are `own`'s, in order. */
function oneRingPerHouse(own: readonly OwnUnitView[], views: OrderView[]): OrderView[] {
  const house = (k: number) => {
    const host = own[k].hold?.host;
    return views[k].building && host?.kind === "building" ? host.id : null;
  };
  const painter = new Map<number, OrderView>();
  views.forEach((v, k) => {
    const id = house(k);
    const best = id === null ? undefined : painter.get(id);
    if (id !== null && (!best || ringRank(v) > ringRank(best))) painter.set(id, v);
  });
  return views.map((v, k) => {
    const id = house(k);
    return id !== null && painter.get(id) !== v ? { ...v, ringShared: true } : v;
  });
}

/** An identified enemy aircraft as its marker reads it. */
export function aircraftView(
  units: UnitCatalog,
  u: Pick<IdentifiedView, "kind" | "position" | "yaw">,
): AircraftView {
  return {
    position: u.position,
    yaw: u.yaw,
    hullHalfLength: units.hull(u.kind)?.half_extents_m[0] ?? 0,
    fixedWing: fixedWing(units.type(u.kind)),
  };
}

/** Every identified enemy aircraft's ground marker and drop line, in the
 *  enemy's colour, as an own aircraft's are drawn. */
export function aircraftLayer(
  units: UnitCatalog,
  o: ObservationView,
  z: SurfaceHeight,
  stroke = gameStroke(OPENING_METRES_PER_PX),
): WorldMeshes {
  return buildAircraftMarks(
    o.identified.filter((u) => airborne(units.type(u.kind))).map((u) => aircraftView(units, u)),
    [...gameHud.enemy, 1],
    z,
    gameOrderStyle,
    { stroke },
  );
}

/** The battle view's overlay: its layers (`battleOverlayParts`) combined. */
export function buildBattleOverlay(...args: Parameters<typeof battleOverlayParts>): WorldMeshes {
  return combineWorldMeshes(battleOverlayParts(...args));
}

/** The battle view's overlay layers, in drawing order; combined they are
 *  what the view uploads (`buildBattleOverlay`). A recording keeps them apart
 *  so a layer that holds still between rebuilds (the border) is kept once. */
export function battleOverlayParts(
  units: UnitCatalog,
  rules: Pick<ScenarioRules, "air" | "tick_hz">,
  o: ObservationView,
  selected: readonly number[],
  z: SurfaceHeight,
  scenario: BattleOverlayScenario,
  {
    showOrders,
    reveal,
    contacts,
    facing,
    allies,
    hovered = null,
    buildings = null,
  }: {
    showOrders: boolean;
    reveal: RevealedOrders;
    contacts: readonly PresentedContact[];
    facing: ContactFacing;
    /** The side's units an ally owns (`allyOwners`). */
    allies?: ReadonlyMap<number, unknown>;
    /** The own unit under the pointer, its model or its card (`ReadoutLayerHandle.place`). */
    hovered?: number | null;
    /** The map's buildings and their parts, which held buildings' outlines trace. */
    buildings?: Pick<BuildingIndex, "partBuilding" | "parts"> | null;
  },
  border: Mesh | null = null,
  stroke = gameStroke(OPENING_METRES_PER_PX),
): Partial<WorldMeshes>[] {
  const contactMarks = contactLayer(contacts, z, facing);
  const supply = supplyLayer(o, scenario.supplyRadius, z, selected, stroke, showOrders);
  const orders = orderLayer(units, rules.air, o, selected, reveal, z, {
    stroke,
    ghosts: showOrders,
    allies,
    hovered,
    buildings,
  });
  const aircraft = aircraftLayer(units, o, z, stroke);
  const deployments = scenario.deployments.map((marker) =>
    buildDeploymentMarker(marker, z, gameOrderStyle, gameHud.accent, { stroke }),
  );
  // The hold zone, a line of the orders' weight: dashed while blue is not
  // holding it. The zone and the border are paint, like the orders.
  const line = stroke(gameOrderStyle.line_px);
  const zone = scenario.zone
    ? buildZoneRing(
        scenario.zone.center,
        scenario.zone.radius,
        line,
        gameZone,
        !!o.encounter && o.encounter.heldS > 0,
        z,
      )
    : null;
  // A capture area's ring takes its flag's colour: the HUD's own colour for
  // the viewer's side, the enemy's for the other. A held one (a solid ring)
  // gives off a faint curtain of the same light.
  const objectives = (o.skirmish?.objectives ?? []).map((objective) => {
    const color: Rgba =
      objective.contested && Math.floor(o.tick / rules.tick_hz) % 2 === 1
        ? [...gameHud.warn, 1]
        : STANDING_RING[objectiveStanding(objective.owner, scenario.side)];
    // Contested capture areas stay a solid ring while their colour flashes.
    const held = objective.contested || objective.owner !== null;
    return {
      painted: buildZoneRing(objective.center, objective.radiusM, line, color, held, z),
      ...(held && {
        translucent: buildZoneCurtain(
          objective.center,
          objective.radiusM - line / 2,
          color,
          gameCaptureCurtain,
          z,
        ),
      }),
    };
  });
  return [
    ...objectives,
    ...(zone ? [{ painted: zone }] : []),
    ...(border ? [{ painted: border }] : []),
    ...(buildings ? [heldBuildingsLayer(o, buildings, line, z)] : []),
    contactMarks,
    // The roads in lie under every mark of the units: they draw first in
    // the marching layer, and the units' marks follow them there.
    ...deployments,
    overEntries(supply),
    overEntries(orders),
    overEntries(aircraft),
  ];
}

/** `part` with its still paint moved, unpulsed, into the marching layer
 *  ahead of its own marching paint, so it draws over the entry markers. */
function overEntries(part: Partial<WorldMeshes>): Partial<WorldMeshes> {
  const { painted, paintedMarching, ...rest } = part;
  const marks = [painted && unpulsed(painted), paintedMarching].filter((m) => !!m);
  return marks.length ? { ...rest, paintedMarching: concatMeshes(marks) } : rest;
}
