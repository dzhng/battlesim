import { useEffect, useRef, useState } from "react";
import { vec2, type Vec2, type Vec3 } from "math";
import type { SessionCatalog } from "@web/battle/catalog/compose";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { BattleView } from "../BattleView";
import type { BattleSession } from "../useBattleSession";
import { useBuiltScenario } from "../useBuiltScenario";
import { STREET_CAMERA, STREET_SEED } from "../streetScenario";
import { REVIEW_LANES, buildProjectileReview, reviewLanePositions } from "../projectileReview";
import { TickStatus } from "../TickStatus";
import { gameEffects } from "../effectFeed";

const _projectiles_xy: Vec2 = [0, 0];
function near(position: Vec3, at: Vec2) {
  return vec2.squaredDistance(vec2.fromBuffer(_projectiles_xy, position, 0), at) < 4;
}

export default function Projectiles() {
  const catalog = useSessionCatalog();
  const built = useBuiltScenario({}, (wasm) => buildProjectileReview(wasm, catalog));
  if (built && typeof built !== "string")
    return <main className="lab-rejected">{built.error}</main>;
  return built ? <Review scenario={built} /> : null;
}

function Review({ scenario }: { scenario: string }) {
  const { weapons } = useSessionCatalog();
  return (
    <BattleView
      fixture="projectiles"
      scenario={scenario}
      seed={STREET_SEED}
      camera={STREET_CAMERA}
      status={(session) => <ReviewControls session={session} />}
      diagnostics={(session) => ({
        show: (index: number) => showLane(weapons, index),
        lanes: () =>
          REVIEW_LANES.map((lane, index) => {
            const glow = gameEffects.tracers[lane.weapon].glow;
            return {
              ...lane,
              ...reviewLanePositions(weapons, index),
              ...weapons[lane.weapon],
              tick_hz: session.rules.tick_hz,
              tracerRGB: glow.color.map((c) => c * glow.intensity),
            };
          }),
      })}
    />
  );
}

function showLane(weapons: SessionCatalog["weapons"], index: number) {
  if (index < 0) window.__lab?.setCamera?.(STREET_CAMERA);
  else {
    const { from, to } = reviewLanePositions(weapons, index);
    window.__lab?.setCamera?.({
      ...STREET_CAMERA,
      target: [(from[0] + to[0]) / 2, from[1], 0],
      distance: (to[0] - from[0]) * 1.05,
      pitch: 0.95,
      yaw: -Math.PI / 2,
    });
  }
}

function ReviewControls({ session }: { session: BattleSession }) {
  const { observation } = session.sim;
  const ordered = useRef(new Set<number>());
  const [view, setView] = useState(-1);
  const { weapons } = useSessionCatalog();
  const weapon = view >= 0 ? weapons[REVIEW_LANES[view].weapon] : null;
  useEffect(() => {
    ordered.current.clear();
  }, [session.sim.client]);
  // Orders use the side's identified handles, just like player attack input.
  useEffect(() => {
    if (!observation) return;
    for (let i = 0; i < REVIEW_LANES.length; i++) {
      if (ordered.current.has(i)) continue;
      const { from, to } = reviewLanePositions(weapons, i);
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
  }, [observation, session.control, weapons]);
  const show = (index: number) => {
    setView(index);
    showLane(weapons, index);
  };
  return (
    <div data-testid="projectiles-panel">
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
            {lane.name} · {reviewLanePositions(weapons, index).to[0] - 100} m
          </button>
        ))}
      </div>
      <TickStatus tick={observation?.tick} status={session.sim.status.status} />
    </div>
  );
}
