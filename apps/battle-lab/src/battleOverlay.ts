// What one side's battle view draws from its own observation besides the
// static world and its structures (the session's fitted props), one layer
// per concern: contact glyphs, visible flight and strike marks, garrison
// occupants, guided missiles, supply reach, the selection's orders, the
// public objective's zone when the scenario has one, and the playable area's
// border (built by the view per zoom step, passed in). Ground marks are for
// selection and movement and extents; every unit state is its info panel's
// (`readouts.tsx`).
// The battle view composes the contacts, supply, orders, zone and border:
// combat effects (`effects/`) draw the flight, the flashes, the impacts and a
// missile's flare and smoke trail, and a garrisoned squad's circle says where
// it holds. Labs compose the layers their fixture exercises, the rest among
// them as diagnostics.
import { gameHud } from "@web/battle/present/hudTheme";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { buildContactGlyphs } from "@packages/battle-renderer/src/contactGlyph";
import { buildFlightOverlay } from "@packages/battle-renderer/src/flightMesh";
import { buildConsequenceOverlay } from "@packages/battle-renderer/src/consequenceOverlay";
import { buildGarrisonOverlay } from "@packages/battle-renderer/src/garrisonOverlay";
import { buildGuidanceOverlay } from "@packages/battle-renderer/src/guidanceOverlay";
import { buildSupplyOverlay } from "@packages/battle-renderer/src/supplyOverlay";
import {
  buildOrderOverlay,
  buildDeploymentMarker,
  type OrderView,
  type SurfaceHeight,
} from "@packages/battle-renderer/src/orderOverlay";
import { combineWorldMeshes, type Mesh } from "@packages/battle-renderer/src/mesh";
import { buildZoneRing } from "@packages/battle-renderer/src/playAreaOverlay";
import type { WorldMeshes } from "@packages/battle-renderer/src/scene";
import type { PresentedContact } from "@web/battle/present/contactPresentation";
import type { ObservationView, OwnUnitView } from "@web/battle/sim/observation";
import type { RevealedOrders } from "@web/battle/present/orderReveal";
import { gameContactStyle } from "./gameFog";
import {
  OPENING_METRES_PER_PX,
  gameOrderStyle,
  gameStroke,
  gameSupplyStyle,
  gameZone,
} from "./gameOverlay";

type P3 = [number, number, number];
// The labs' own marks: tracers (the battle draws flight as effects) and
// where a round struck.
const OWN_TRACER = [0.98, 0.97, 0.9, 1] as const;
const ENEMY_TRACER = [1.0, 0.45, 0.4, 1] as const;
const IMPACT = [1.0, 0.86, 0.35, 1] as const;

/** What the overlay draws from the scenario itself. */
export interface BattleOverlayScenario {
  supplyRadius: number;
  zone: { center: readonly [number, number]; radius: number } | null;
  deployment: { center: readonly [number, number]; facing: number } | null;
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

/** Glyphs consume the same presented reports as their labels. */
export function contactLayer(contacts: readonly PresentedContact[], z: SurfaceHeight): WorldMeshes {
  return buildContactGlyphs(contacts, z, gameContactStyle);
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
    o.own.flatMap((u) => (u.garrison ? [{ members: u.members, phase: u.garrison.phase }] : [])),
    z,
  );
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
  metresPerPx = OPENING_METRES_PER_PX,
  all = false,
): WorldMeshes {
  return buildSupplyOverlay(
    o.own
      .filter((u) => (all || selected.includes(u.id)) && u.stock !== null && u.stock > 0)
      .map((u) => ({ center: [u.position[0], u.position[1]], radius })),
    z,
    gameStroke(metresPerPx)(gameOrderStyle.line_px),
    gameSupplyStyle,
  );
}

/** An own unit as the orders draw it: selected or not, its order marks
 *  shown at `reveal` (0: none). */
export function orderView(
  units: UnitCatalog,
  u: OwnUnitView,
  selected: boolean,
  reveal = 0,
): OrderView {
  const g = u.garrison;
  return {
    ...u,
    // Its footprint sizes its marker: a hull's half length, 0 for a squad;
    // a garrisoned squad's, its building's (inside or leaving, not entering).
    hullHalfLength: units.hull(u.kind)?.half_extents_m[0] ?? 0,
    building: g && g.phase !== "entering" ? { center: g.center, half: g.half } : null,
    selected,
    reveal,
  };
}

/** The selection's markers under the units in `selected`, and the order
 *  marks (the Space view) of each unit `reveal` shows, at its opacity
 *  (`OrderReveal`). Lines are the style's pixel widths at `metresPerPx` by
 *  the stroke rule (the opening camera's scale for a view that doesn't
 *  follow its camera). */
export function orderLayer(
  units: UnitCatalog,
  o: ObservationView,
  selected: readonly number[],
  reveal: RevealedOrders,
  z: SurfaceHeight,
  metresPerPx = OPENING_METRES_PER_PX,
): WorldMeshes {
  return buildOrderOverlay(
    o.own
      .filter((u) => selected.includes(u.id) || reveal.has(u.id))
      .map((u) => orderView(units, u, selected.includes(u.id), reveal.get(u.id))),
    z,
    gameOrderStyle,
    { stroke: gameStroke(metresPerPx) },
  );
}

export function buildBattleOverlay(
  units: UnitCatalog,
  o: ObservationView,
  selected: readonly number[],
  z: SurfaceHeight,
  scenario: BattleOverlayScenario,
  {
    showOrders,
    reveal,
    contacts,
  }: { showOrders: boolean; reveal: RevealedOrders; contacts: readonly PresentedContact[] },
  border: Mesh | null = null,
  metresPerPx = OPENING_METRES_PER_PX,
): WorldMeshes {
  const contactMarks = contactLayer(contacts, z);
  const supply = supplyLayer(o, scenario.supplyRadius, z, selected, metresPerPx, showOrders);
  const orders = orderLayer(units, o, selected, reveal, z, metresPerPx);
  const deployment = scenario.deployment
    ? buildDeploymentMarker(
        scenario.deployment.center,
        scenario.deployment.facing,
        z,
        gameOrderStyle,
        { stroke: gameStroke(metresPerPx) },
      )
    : null;
  // The hold zone, a line of the orders' weight: dashed while blue is not
  // holding it. The zone and the border are paint, like the orders.
  const line = gameStroke(metresPerPx)(gameOrderStyle.line_px);
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
  const objectives = (o.skirmish?.objectives ?? []).map((objective) => ({
    painted: buildZoneRing(
      objective.center,
      objective.radiusM,
      line,
      objective.owner === "red"
        ? [...gameHud.enemy, 1]
        : objective.owner === "blue"
          ? gameOrderStyle.selected
          : gameZone,
      objective.owner !== null && !objective.contested,
      z,
    ),
  }));
  return combineWorldMeshes([
    ...objectives,
    ...(zone ? [{ painted: zone }] : []),
    ...(border ? [{ painted: border }] : []),
    contactMarks,
    supply,
    orders,
    ...(deployment ? [{ paintedMarching: deployment, translucent: deployment }] : []),
  ]);
}
