import { ReadoutLayer } from "@web/battle/present/readouts";
import { useCallback, useMemo } from "react";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import type { MountView, ObservationView, OwnUnitView } from "@web/battle/sim/observation";
import { REASON_TEXT } from "../reasonText";
import type { Order } from "@web/battle/sim/protocol";
import { UNITS, WEAPONS } from "@packages/scene-assets/src/shippedUnits";
import { AckLog } from "../AckLog";
import { FeedInspector } from "../FeedInspector";
import { contactLayer, tracerLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { SavedEncounter, type SavedBattle } from "../savedMaps";
import { useFeed } from "../feed";
import { gameCamera } from "../gameCamera";
import { TickStatus } from "../TickStatus";

// Blue's tank and rifle squad face a red tank shuttling past a short wall and
// a red squad firing from behind a wall every three seconds (a firing
// area, never identified). The wall is low (2 m): it hides the squad
// and stops the rifles, and the grenade launcher lobs over it at the
// report, within grenade reach. Both blue units look past the short wall
// along one line, so the red tank drops out of sight briefly: acquisition grace.
// The hidden squad reports fire while the tank is still alive, so target
// priority is checked independently of the tank's eventual death.
const SEED = 8;

const WEAPONS_CAMERA: Camera3DParams = {
  target: [265, 195, 0],
  distance: 300,
  pitch: 0.95,
  yaw: -1.57,
  ...gameCamera.lens,
};

/** Reference commands, exactly as a player would send them. */
const DEMOS: Record<string, (o: ObservationView, units: number[]) => Order | null> = {
  "Attack red tank": (o, units) => {
    const tank = o.identified.find((e) => e.kind === "tank");
    return tank ? { kind: "attack", units, target: { kind: "identified", id: tank.id } } : null;
  },
  "Attack firing area": (o, units) => {
    const area = o.contacts.find((c) => c.source === "firing");
    return area ? { kind: "attack", units, target: { kind: "contact", id: area.id } } : null;
  },
  "Attack ground": (_, units) => ({
    kind: "attack",
    units,
    target: { kind: "ground", point: [380, 330, 0] },
  }),
  "Attack-move east": (_, units) => ({
    kind: "attack_move",
    units,
    gesture: 9101,
    goal: [600, 230],
  }),
  "Return fire only": (_, units) => ({ kind: "set_engagement", units, policy: "return_fire_only" }),
  "Fire at will": (_, units) => ({ kind: "set_engagement", units, policy: "fire_at_will" }),
};

export default function Weapons() {
  return (
    <SavedEncounter fixture="weapons" encounter="weapons">
      {(battle) => <WeaponsLab battle={battle} />}
    </SavedEncounter>
  );
}

function WeaponsLab({ battle }: { battle: SavedBattle }) {
  const session = useBattleSession({ ...battle, seed: SEED });
  const { world, meshes, sim, control, surfaceZ } = session;
  const worldFeed = useFeed(meshes);
  const { observation } = sim;

  const overlay = useMemo(() => {
    if (!world || !observation) return undefined;
    const contacts = contactLayer(session.contacts, surfaceZ);
    // This tick's visible flight: own rounds whole, enemy rounds only over seen ground.
    const tracers = tracerLayer(observation, { sideColors: false });
    return {
      opaque: concatMeshes([contacts.opaque, tracers.opaque]),
      translucent: concatMeshes([contacts.translucent, tracers.translucent]),
    };
  }, [world, observation, surfaceZ, session.contacts]);
  const overlayFeed = useFeed(overlay);

  const runDemo = useCallback(
    async (name: string) => {
      const o = sim.latest.current;
      if (!o || !control.selected.length) return null;
      const order = DEMOS[name](o, control.selected);
      return order && control.issue(order);
    },
    [sim.latest, control],
  );

  // Lab-only probes for the scene harness; rebuilt each render.
  const diagnostics = {
    ...session.probes,
    command: (order: Order) => control.issue(order),
    demo: (name: string) => runDemo(name),
  };

  if (!meshes) return null;
  return (
    <>
      <LabViewport
        fixture="weapons"
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        buildings={session.buildingsFeed}
        overlay={overlayFeed}
        fog={session.fogFeed}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={WEAPONS_CAMERA}
        onPick={session.onPick}
        onRightPress={session.onRightPress}
        onCursor={session.onCursor}
        pointerMarks={session.pointerPaint.feed}
        onBox={session.onBox}
        onFrame={session.placePanels}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <ReadoutLayer
        own={[]}
        contacts={session.contacts}
        tick={observation?.tick}
        rules={session.rules}
        selected={[]}
        handle={session.readouts}
      />
      <aside className="hud-panel lab-panel" data-testid="weapons-panel" data-occludes-readouts>
        <strong>Weapons</strong>
        <div className="lab-hint">
          Click: select · Right‑click: move · S: stop · buttons act on the selection
        </div>
        <TickStatus tick={observation?.tick} status={sim.status.status} />
        <div className="lab-row">
          {Object.keys(DEMOS).map((name) => (
            <button
              key={name}
              type="button"
              onClick={() => void runDemo(name)}
              disabled={!control.selected.length}
            >
              {name}
            </button>
          ))}
          <button type="button" onClick={control.stop} disabled={!control.selected.length}>
            Stop
          </button>
          <button type="button" onClick={sim.restart}>
            Reset
          </button>
        </div>
        <div data-testid="action-panel" className="lab-actions">
          {control.selectedUnits.length === 0 && <div className="lab-hint">No unit selected</div>}
          {control.selectedUnits.map((u) => (
            <UnitReadiness key={u.id} unit={u} observation={observation!} />
          ))}
        </div>
        <AckLog acks={control.acks} />
        {observation && <FeedInspector observation={observation} />}
      </aside>
    </>
  );
}

function UnitReadiness({ unit, observation }: { unit: OwnUnitView; observation: ObservationView }) {
  return (
    <div className="lab-unit">
      <div>
        <strong>
          {unit.kind} #{unit.id}
        </strong>{" "}
        · {unit.engagement === "fire_at_will" ? "fire at will" : "return fire only"} · movement:{" "}
        {unit.state === "halted" ? "halted to engage" : unit.state.replace("_", " ")}
      </div>
      {unit.mounts.map((m) => (
        <MountRow key={m.mount} kind={unit.kind} mount={m} observation={observation} />
      ))}
    </div>
  );
}

// Raw diagnostic bars for the lab: aim and reload progress.
function MountRow({
  kind,
  mount,
  observation,
}: {
  kind: string;
  mount: MountView;
  observation: ObservationView;
}) {
  const spec = UNITS.type(kind).mounts[mount.mount];
  const weapon = WEAPONS[spec.weapons[mount.loaded ?? 0]];
  const ammo = mount.ammo.map((n, k) => `${spec.weapons[k]} ${n === null ? "∞" : n}`).join(" · ");
  return (
    <div className="lab-mount" data-reason={mount.reason}>
      <div>
        {spec.name}:{" "}
        <span className={`lab-reason lab-reason-${reasonTone(mount.reason)}`}>
          {REASON_TEXT[mount.reason] ?? mount.reason}
        </span>
      </div>
      <div className="lab-hint">
        {mount.loaded === null ? "nothing loaded" : `${spec.weapons[mount.loaded]} loaded`} · {ammo}{" "}
        · {describeTarget(mount, observation)}
      </div>
      <Bar label="aim" progress={mount.aim} seconds={weapon.aim_s} />
      {/* A loaded weapon is ready: its reload bar reads full. */}
      <Bar
        label="reload"
        progress={mount.loaded === null ? mount.reload : 1}
        seconds={weapon.reload_s}
      />
    </div>
  );
}

/** `progress` in [0, 1] of a `seconds`-long timer. */
function Bar({ label, progress, seconds }: { label: string; progress: number; seconds: number }) {
  return (
    <div className="lab-bar">
      <span>{label}</span>
      <meter min={0} max={1} value={progress} />
      <span>
        {/* Rounded down, so an unfinished timer never reads complete. */}
        {(Math.floor(progress * seconds * 100) / 100).toFixed(2)}/{seconds.toFixed(2)} s
      </span>
    </div>
  );
}

function describeTarget(mount: MountView, o: ObservationView): string {
  const t = mount.target;
  if (!t) return "no target";
  if (t.kind === "ground") return `ground (${t.point[0].toFixed(0)}, ${t.point[1].toFixed(0)})`;
  if (t.kind === "contact") return `area ${t.id}`;
  const e = o.identified.find((x) => x.id === t.id);
  return `enemy ${t.id} · ${e ? e.kind : "out of sight"}`;
}

/** Colour family of an action reason: acting, waiting on a timer, or prevented. */
function reasonTone(reason: string): "active" | "waiting" | "prevented" {
  if (reason === "firing") return "active";
  if (["aiming", "reloading", "turret_traversing", "tracking_last_sighting"].includes(reason))
    return "waiting";
  return "prevented";
}
