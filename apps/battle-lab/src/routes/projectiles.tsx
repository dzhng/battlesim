import { useEffect, useMemo, useRef, useState } from "react";
import { vec2, type Vec2, type Vec3 } from "math";
import { WEAPONS } from "@packages/scene-assets/src/shippedUnits";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { useBuiltScenario } from "../useBuiltScenario";
import { useFeed } from "../feed";
import { STREET_CAMERA, STREET_SEED } from "../streetScenario";
import { REVIEW_LANES, buildProjectileReview, reviewLanePositions } from "../projectileReview";
import { TickStatus } from "../TickStatus";
import { gameEffects } from "../effectFeed";

const _projectiles_xy: Vec2 = [0, 0];
function near(position: Vec3, at: Vec2) {
  return vec2.squaredDistance(vec2.fromBuffer(_projectiles_xy, position, 0), at) < 4;
}

export default function Projectiles() {
  const built = useBuiltScenario({}, buildProjectileReview);
  if (built && typeof built !== "string")
    return <main className="lab-rejected">{built.error}</main>;
  return built ? <Review scenario={built} /> : null;
}

function Review({ scenario }: { scenario: string }) {
  const map = useMemo(() => JSON.parse(scenario).map as unknown, [scenario]);
  const session = useBattleSession({ map, scenario, seed: STREET_SEED, destroyable: "apart" });
  const { observation } = session.sim;
  const ordered = useRef(new Set<number>());
  const [view, setView] = useState(-1);
  const weapon = view >= 0 ? WEAPONS[REVIEW_LANES[view].weapon] : null;
  useEffect(() => {
    ordered.current.clear();
  }, [session.sim.client]);
  // Orders use the side's identified handles, just like player attack input.
  useEffect(() => {
    if (!observation) return;
    for (let i = 0; i < REVIEW_LANES.length; i++) {
      if (ordered.current.has(i)) continue;
      const { from, to } = reviewLanePositions(i);
      const own = observation.own.find((u) => near(u.position, from));
      const target = observation.identified.find((u) => near(u.position, to));
      if (!own || !target) continue;
      ordered.current.add(i);
      void session.control
        .issue({ kind: "attack", units: [own.id], target: { kind: "identified", id: target.id } })
        .then((ack) => {
          if (ack?.error) ordered.current.delete(i);
        });
    }
  }, [observation, session.control]);
  const bare = useMemo(
    () => session.meshes && { ...session.meshes, grass: null },
    [session.meshes],
  );
  const world = useFeed(bare);
  const show = (index: number) => {
    setView(index);
    if (index < 0) window.__lab?.setCamera?.(STREET_CAMERA);
    else {
      const { from, to } = reviewLanePositions(index);
      window.__lab?.setCamera?.({
        ...STREET_CAMERA,
        target: [(from[0] + to[0]) / 2, from[1], 0],
        distance: (to[0] - from[0]) * 1.05,
        pitch: 0.95,
        yaw: -Math.PI / 2,
      });
    }
  };
  if (!bare) return null;
  return (
    <>
      <LabViewport
        fixture="projectiles"
        world={world}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        massing={session.massingFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={STREET_CAMERA}
        groundAt={session.surfaceZ}
        onReady={session.onReady}
        diagnostics={{
          ...session.probes,
          show,
          lanes: () =>
            REVIEW_LANES.map((lane, index) => {
              const glow = gameEffects.tracers[lane.weapon].glow;
              return {
                ...lane,
                ...reviewLanePositions(index),
                ...WEAPONS[lane.weapon],
                tick_hz: session.rules.tick_hz,
                tracerRGB: glow.color.map((c) => c * glow.intensity),
              };
            }),
        }}
      />
      <aside className="hud-panel lab-panel" data-testid="projectiles-panel">
        <strong>Projectile review</strong>
        <div className="lab-hint">
          Unlimited ammo · units and scenery take no damage · normal aim and reload cycles
        </div>
        {weapon && (
          <div className="lab-hint">
            {weapon.speed_mps}
            {typeof weapon.top_speed_mps === "number" ? ` → ${weapon.top_speed_mps}` : ""} m/s
          </div>
        )}
        <div className="lab-row">
          <button type="button" aria-pressed={view === -1} onClick={() => show(-1)}>
            Close fight
          </button>
          {REVIEW_LANES.map((lane, index) => (
            <button
              type="button"
              key={lane.weapon}
              aria-pressed={view === index}
              onClick={() => show(index)}
            >
              {lane.name} · {reviewLanePositions(index).to[0] - 100} m
            </button>
          ))}
        </div>
        <TickStatus tick={observation?.tick} status={session.sim.status.status} />
      </aside>
    </>
  );
}
