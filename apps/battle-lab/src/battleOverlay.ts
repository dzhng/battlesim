// Everything one side's battle view draws over the world from its own
// observation: remembered obstacles and ruins, uncertain evidence, visible
// flight and strike marks, the fallen and suppression, garrisons, guided
// missiles, supply reach and set-up progress, the selection's orders, and
// the public objective: the village's hold zone.
// The village battle composes it; labs keep their narrower overlays.
import { buildEvidenceOverlay } from "@packages/battle-renderer/src/evidenceOverlay";
import { buildFlightOverlay } from "@packages/battle-renderer/src/flightMesh";
import {
  buildConsequenceOverlay,
  type ImpactMark,
} from "@packages/battle-renderer/src/consequenceOverlay";
import { buildGarrisonOverlay } from "@packages/battle-renderer/src/garrisonOverlay";
import { buildGuidanceOverlay } from "@packages/battle-renderer/src/guidanceOverlay";
import { buildSupplyOverlay } from "@packages/battle-renderer/src/supplyOverlay";
import { buildDeploymentOverlay } from "@packages/battle-renderer/src/deploymentOverlay";
import { buildOrderOverlay } from "@packages/battle-renderer/src/orderOverlay";
import {
  concatMeshes,
  groundRing,
  MeshBuilder,
  type Mesh,
  type Rgba,
} from "@packages/battle-renderer/src/mesh";
import type { WorldMeshes } from "@packages/battle-renderer/src/scene";
import type { ObservationView } from "@web/battle/sim/observation";
import village from "@fixtures/village.json";

type P3 = [number, number, number];
const OWN_TRACER = [0.98, 0.97, 0.9, 1] as const;
const ENEMY_TRACER = [1.0, 0.45, 0.4, 1] as const;
const SUPPLY_RADIUS = village.service.radius_m;
const ZONE = village.encounter;
const ZONE_EDGE: Rgba = [1.0, 0.84, 0.3, 1];
export const IMPACT_TICKS = 60;

/** What the view remembers between frames: recent strikes and missile paths. */
export class BattleMemory {
  impacts: { at: P3; tick: number }[] = [];
  trails = new Map<number, P3[]>();

  /** Fold in one decoded frame. */
  note(o: ObservationView) {
    this.impacts = this.impacts.filter((i) => o.tick - i.tick < IMPACT_TICKS);
    for (const p of o.projectiles) if (p.impact) this.impacts.push({ at: p.to, tick: o.tick });
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

export function buildBattleOverlay(
  o: ObservationView,
  memory: BattleMemory,
  selected: readonly number[],
  standing: Mesh,
  z: (x: number, y: number) => number,
): WorldMeshes {
  const evidence = buildEvidenceOverlay(
    o.contacts.map((c) => ({
      center: c.center,
      radius: c.radius,
      source: c.source,
      freshness: Math.max(
        0,
        (c.expiresTick - o.tick) / Math.max(1, c.expiresTick - c.evidenceTick),
      ),
    })),
    o.knownProps,
    z,
  );
  const tracers = buildFlightOverlay(
    o.projectiles.map((p) => ({
      points: [p.from, p.to],
      outcome: "flying" as const,
      color: p.own ? OWN_TRACER : ENEMY_TRACER,
    })),
    [],
    [],
    0.3,
  );
  const marks: ImpactMark[] = memory.impacts.map((i) => ({
    at: i.at,
    fade: 1 - (o.tick - i.tick) / IMPACT_TICKS,
  }));
  const remains = buildConsequenceOverlay(
    o.corpses,
    o.own
      .filter((u) => u.members.length > 0 && u.garrison?.phase !== "inside")
      .map((u) => ({ center: [u.position[0], u.position[1]], radius: 9, level: u.suppression })),
    marks,
    z,
  );
  const garrisons = buildGarrisonOverlay(
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
  const guidance = buildGuidanceOverlay(
    o.guided.map((g) => ({ ...g, trail: memory.trails.get(g.id) ?? [] })),
    z,
  );
  const trucks = o.own.filter((u) => u.stock !== null);
  const supply = buildSupplyOverlay(
    trucks
      .filter((u) => (u.stock ?? 0) > 0)
      .map((u) => ({
        center: [u.position[0], u.position[1]],
        radius: SUPPLY_RADIUS,
        ready: u.deployment?.progress === 1 && u.state === "idle",
      })),
    o.own
      .filter(
        (u) => u.stock === null && ["serving", "moving", "firing", "no_stock"].includes(u.service),
      )
      .map((u) => ({
        center: [u.position[0], u.position[1]],
        state: u.service === "serving" ? ("serving" as const) : ("waiting" as const),
      })),
    z,
  );
  const setup = buildDeploymentOverlay(
    trucks.flatMap((u) =>
      u.deployment ? [{ position: u.position, yaw: u.yaw, ...u.deployment }] : [],
    ),
    z,
  );
  const orders = buildOrderOverlay(
    o.own.filter((u) => selected.includes(u.id)),
    z,
  );
  // The hold zone: dashed while blue is not holding it, solid while it is.
  const zone = new MeshBuilder();
  const [cx, cy] = ZONE.success_zone_center;
  const r = ZONE.success_zone_radius_m;
  groundRing(zone, [cx, cy], r - 2, r, ZONE_EDGE, z, !(o.encounter && o.encounter.heldS > 0));
  const parts = [evidence, tracers, remains, garrisons, guidance, supply, orders];
  return {
    opaque: concatMeshes([standing, setup, zone.build(), ...parts.map((p) => p.opaque)]),
    translucent: concatMeshes(parts.map((p) => p.translucent)),
  };
}
