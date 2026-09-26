// What one side's battle view draws from its own observation besides the
// static world. Two kinds, drawn by different passes of the battle frame:
// - structures: world geometry the side knows (standing buildings, remembered
//   ruins, wrecks and obstacles), lit and fogged with the world;
// - overlays: display-space marks, one layer per concern: uncertain evidence,
//   visible flight and strike marks, the fallen and suppression, garrisons,
//   guided missiles, supply reach and set-up progress, the selection's orders,
//   and the public objective's zone when the scenario has one.
// The battle view composes every layer; labs compose the layers their fixture
// exercises.
import { buildEvidenceOverlay } from "@packages/battle-renderer/src/evidenceOverlay";
import { buildKnownStructures } from "@packages/battle-renderer/src/knownStructures";
import { buildFlightOverlay } from "@packages/battle-renderer/src/flightMesh";
import { buildConsequenceOverlay } from "@packages/battle-renderer/src/consequenceOverlay";
import { buildGarrisonOverlay } from "@packages/battle-renderer/src/garrisonOverlay";
import { buildGuidanceOverlay } from "@packages/battle-renderer/src/guidanceOverlay";
import { buildSupplyOverlay } from "@packages/battle-renderer/src/supplyOverlay";
import { buildDeploymentOverlay } from "@packages/battle-renderer/src/deploymentOverlay";
import { buildOrderOverlay, type SurfaceHeight } from "@packages/battle-renderer/src/orderOverlay";
import {
  concatMeshes,
  groundAnnulus,
  MeshBuilder,
  type Mesh,
  type Rgba,
} from "@packages/battle-renderer/src/mesh";
import type { WorldMeshes } from "@packages/battle-renderer/src/scene";
import { SERVICE_WAITING } from "@web/battle/present/readouts";
import type { ObservationView } from "@web/battle/sim/observation";

type P3 = [number, number, number];
const OWN_TRACER = [0.98, 0.97, 0.9, 1] as const;
const ENEMY_TRACER = [1.0, 0.45, 0.4, 1] as const;
const ZONE_EDGE: Rgba = [1.0, 0.84, 0.3, 1];

/** What the overlay draws from the scenario itself. */
export interface BattleOverlayScenario {
  supplyRadius: number;
  zone: { center: readonly [number, number]; radius: number } | null;
}
/** How long a strike mark stays (ticks), fading out. */
export const IMPACT_TICKS = 60;

/** What the view remembers between frames: recent strikes and missile paths. */
export class BattleMemory {
  impacts: { at: P3; tick: number }[] = [];
  trails = new Map<number, P3[]>();
  readonly impactTicks: number;
  private readonly ownImpactsOnly: boolean;

  constructor({ impactTicks = IMPACT_TICKS, ownImpactsOnly = false } = {}) {
    this.impactTicks = impactTicks;
    this.ownImpactsOnly = ownImpactsOnly;
  }

  /** Fold in one decoded frame. */
  note(o: ObservationView) {
    this.impacts = this.impacts.filter((i) => o.tick - i.tick < this.impactTicks);
    for (const p of o.projectiles)
      if (p.hit !== "none" && (p.own || !this.ownImpactsOnly))
        this.impacts.push({ at: p.to, tick: o.tick });
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

/** Contact areas fading toward expiry. */
export function evidenceLayer(o: ObservationView, z: SurfaceHeight): WorldMeshes {
  return buildEvidenceOverlay(
    o.contacts.map((c) => ({
      center: c.center,
      radius: c.radius,
      source: c.source,
      freshness: Math.max(
        0,
        (c.expiresTick - o.tick) / Math.max(1, c.expiresTick - c.evidenceTick),
      ),
    })),
    z,
  );
}

/** The obstacles, ruins and wrecks the side has learned: world structures. */
export function knownStructures(o: ObservationView): Mesh {
  return buildKnownStructures(o.knownProps);
}

/** Everything the side knows stands: the buildings it has not seen fall
 *  (`standing`) and the props it has learned. */
export function battleStructures(o: ObservationView, standing: Mesh): Mesh {
  return concatMeshes([standing, knownStructures(o)]);
}

/** This tick's visible flight, own and enemy rounds tinted apart (or all in
 *  the neutral flying colour when `sideColors` is off). */
export function tracerLayer(o: ObservationView, { sideColors = true } = {}): WorldMeshes {
  return buildFlightOverlay(
    o.projectiles.map((p) => ({
      points: [p.from, p.to],
      outcome: "flying" as const,
      color: sideColors ? (p.own ? OWN_TRACER : ENEMY_TRACER) : undefined,
    })),
    [],
    [],
    0.3,
  );
}

/** The fallen, recent strike marks from `memory`, and a halo under each
 *  suppressed squad outside a building (unless `suppression` is off). */
export function remainsLayer(
  o: ObservationView,
  memory: BattleMemory | null,
  z: SurfaceHeight,
  { suppression = true } = {},
): WorldMeshes {
  return buildConsequenceOverlay(
    o.corpses,
    suppression
      ? o.own
          .filter((u) => u.members.length > 0 && u.garrison?.phase !== "inside")
          .map((u) => ({
            center: [u.position[0], u.position[1]],
            radius: 9,
            level: u.suppression,
          }))
      : [],
    (memory?.impacts ?? []).map((i) => ({
      at: i.at,
      fade: 1 - (o.tick - i.tick) / memory!.impactTicks,
    })),
    z,
  );
}

/** Occupant pads and entry/exit timers of squads in or at buildings. */
export function garrisonLayer(o: ObservationView, z: SurfaceHeight): WorldMeshes {
  return buildGarrisonOverlay(
    o.own.flatMap((u) =>
      u.garrison
        ? [
            {
              center: [u.position[0], u.position[1]] as const,
              members: u.members,
              phase: u.garrison.phase,
              progress: u.garrison.progress,
              suppression: u.suppression,
            },
          ]
        : [],
    ),
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

/** Each stocked supply vehicle's reach (`radius` metres; solid once set up
 *  and standing) and, under each unit being served or waiting to be, a full
 *  or broken ring. An empty truck reaches nobody: no ring. */
export function supplyLayer(o: ObservationView, radius: number, z: SurfaceHeight): WorldMeshes {
  return buildSupplyOverlay(
    o.own
      .filter((u) => u.stock !== null && u.stock > 0)
      .map((u) => ({
        center: [u.position[0], u.position[1]],
        radius,
        ready: u.deployment?.progress === 1 && u.state === "idle",
      })),
    o.own
      .filter(
        (u) => u.stock === null && (u.service === "serving" || SERVICE_WAITING.has(u.service)),
      )
      .map((u) => ({
        center: [u.position[0], u.position[1]],
        state: u.service === "serving" ? ("serving" as const) : ("waiting" as const),
      })),
    z,
  );
}

/** Set-up progress rings of own deployable units. */
export function deploymentLayer(o: ObservationView, z: SurfaceHeight): Mesh {
  return buildDeploymentOverlay(
    o.own.flatMap((u) =>
      u.deployment ? [{ position: u.position, yaw: u.yaw, ...u.deployment }] : [],
    ),
    z,
  );
}

/** Routes, destinations and queues of the own units in `units`. */
export function orderLayer(
  o: ObservationView,
  units: readonly number[],
  z: SurfaceHeight,
): WorldMeshes {
  return buildOrderOverlay(
    o.own.filter((u) => units.includes(u.id)),
    z,
  );
}

export function buildBattleOverlay(
  o: ObservationView,
  memory: BattleMemory,
  selected: readonly number[],
  z: SurfaceHeight,
  scenario: BattleOverlayScenario,
): WorldMeshes {
  const evidence = evidenceLayer(o, z);
  const tracers = tracerLayer(o);
  const remains = remainsLayer(o, memory, z);
  const garrisons = garrisonLayer(o, z);
  const guidance = guidanceLayer(o, memory, z);
  const supply = supplyLayer(o, scenario.supplyRadius, z);
  const setup = deploymentLayer(o, z);
  const orders = orderLayer(o, selected, z);
  // The hold zone: dashed while blue is not holding it, solid while it is.
  const zone = new MeshBuilder();
  if (scenario.zone) {
    const { center, radius } = scenario.zone;
    const held = !!o.encounter && o.encounter.heldS > 0;
    groundAnnulus(zone, center, radius - 2, radius, {
      z,
      lift: 0.35,
      segments: 64,
      colorIn: ZONE_EDGE,
      dashed: !held,
    });
  }
  const parts = [evidence, tracers, remains, garrisons, guidance, supply, orders];
  return {
    opaque: concatMeshes([setup, zone.build(), ...parts.map((p) => p.opaque)]),
    translucent: concatMeshes(parts.map((p) => p.translucent)),
  };
}
